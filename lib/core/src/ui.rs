//! The view tree and everything that happens to it.
//!
//! Views live in one flat `Vec` and refer to each other by `ViewId`. There are
//! no parent pointers to keep alive, no reference counting and no interior
//! mutability — and, not by accident, the same handle that makes the borrow
//! checker happy here is the `u16` that crosses DOS's INT 60h and the
//! WebAssembly boundary. Designing for the hardest boundary first paid for
//! itself before either boundary existed.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::buffer::Buffer;
use crate::cell::{attr, attr_bg, attr_fg, glyph, Glyph};
use crate::event::{Button, Event, Key, Mouse, MouseKind};
use crate::geom::{Point, Rect};
use crate::files::{FileKind, FileList};
use crate::hex::HexView;
use crate::controls::{Cluster, ListBox, StaticText};
use crate::html::{Html, Style};
use crate::keymap::Keymap;
use crate::menu::{clone_items, merge_items, Cmd, MenuBar, MenuBox};
use crate::palette::{Palette, WinColors};
use crate::button::ButtonRow;
use crate::views::{Desktop, Dock, Kind, TextView, Window};

/// Where the path field sits inside the panel. One place, so the drawing and
/// the caret cannot disagree about it.
fn field_rect(panel: Rect) -> Rect {
    Rect::new(panel.x + 1, panel.y, panel.w - 2, 1)
}

const SP: u8 = 0x20;
const SP_CH: char = ' ';

fn trim(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Glyphs back into the core's kind of string, a char per glyph.
fn glyph_string(g: &[Glyph]) -> String {
    g.iter().map(|&c| char::from_u32(c as u32).unwrap_or('?')).collect()
}

/// A menu called Edit, whatever its hotkey.
fn is_edit_menu(it: &crate::menu::MenuItem) -> bool {
    !it.items.is_empty() && it.label().eq_ignore_ascii_case("edit")
}

/// The text position under a cell of a text view, `row` rows and `col`
/// columns into it, clamped to the text: past the end of a line is its end.
fn text_point(t: &TextView, row: i16, col: i16) -> Point {
    if t.wrapping() {
        return t.point_at(col, row);
    }
    let y = (t.top + row).clamp(0, (t.line_count() - 1).max(0));
    let len = t.lines.get(y as usize).map_or(0, |l| l.len()) as i16;
    Point::new((t.left + col).clamp(0, len), y)
}

/// A window's rectangle laid out in `from` moved to the same place in `to`,
/// in proportion: a resizable window's edges at the same fractions of the
/// work area, within its smallest and largest size; a fixed-size one the
/// same size, its middle at the same fraction. An edge at the work area's
/// own edge, or within two cells of it, stays that far from it: an editor
/// with a margin round it keeps the margin, and a window against the right
/// edge stays against it, while the edges between windows divide the
/// screen as they did.
fn rescale(r: Rect, from: Rect, to: Rect, w: &Window) -> Rect {
    const NEAR: i16 = 2;
    let at = |v: i16, f0: i16, flen: i16, t0: i16, tlen: i16| -> i16 {
        if flen <= 0 {
            return v;
        }
        if (0..=NEAR).contains(&(v - f0)) {
            return t0 + (v - f0);
        }
        if (0..=NEAR).contains(&(f0 + flen - v)) {
            return t0 + tlen - (f0 + flen - v);
        }
        let d = (v - f0) as i32 * tlen as i32;
        // Rounded to the nearest cell, either side of zero alike.
        let q = if d >= 0 { (2 * d + flen as i32) / (2 * flen as i32) } else { -((-2 * d + flen as i32) / (2 * flen as i32)) };
        t0 + q as i16
    };
    let x = |v| at(v, from.x, from.w, to.x, to.w);
    let y = |v| at(v, from.y, from.h, to.y, to.h);
    if !w.resizable {
        return Rect::new(x(r.x + r.w / 2) - r.w / 2, y(r.y + r.h / 2) - r.h / 2, r.w, r.h);
    }
    let (min_w, min_h) = (w.min_w.max(6), w.min_h.max(3));
    let max_w = if w.max_w > 0 { w.max_w } else { i16::MAX };
    let max_h = if w.max_h > 0 { w.max_h } else { i16::MAX };
    let (x0, y0) = (x(r.x), y(r.y));
    let nw = (x(r.x + r.w) - x0).clamp(min_w, max_w.max(min_w));
    let nh = (y(r.y + r.h) - y0).clamp(min_h, max_h.max(min_h));
    Rect::new(x0, y0, nw, nh)
}

/// The scroll bar a list or a tree draws in its own last column: that
/// column and the row of its marker, or `None` when everything fits and
/// there is no bar. One place for the drawing and the clicking alike.
fn inner_bar(abs: Rect, top: i16, total: i16, page: i16) -> Option<(i16, i16)> {
    if total <= page || abs.w < 4 || abs.h < 3 {
        return None;
    }
    let track = abs.h - 2;
    let span = (total - page).max(1);
    let thumb = (top as i32 * (track - 1) as i32 / span as i32).clamp(0, (track - 1) as i32);
    Some((abs.right() - 1, abs.y + 1 + thumb as i16))
}

/// What a console's selection is drawn in.
const CONSOLE_SELECTED: u8 = 0x70;

/// A console: black, whatever the window around it is, because a terminal
/// is black and the colours written into it were chosen against black.
fn draw_console(c: &crate::console::Console, abs: Rect, buf: &mut Buffer, clip: Rect) {
    buf.fill(abs, SP, crate::console::CONSOLE_ATTR, clip);
    let selecting = c.selection().is_some();
    for row in 0..abs.h {
        let Some(cells) = c.row(c.top + row) else {
            break;
        };
        let (line, start) = c.row_start(c.top + row).unwrap_or((0, 0));
        for (col, cell) in cells.iter().enumerate() {
            // Selected: one colour for the whole of it, black on grey, as a
            // terminal shows it. Each cell's own colours turned round made
            // a strip of every colour the lines happened to be in.
            let a = if selecting && c.is_selected(line, start + col) { CONSOLE_SELECTED } else { cell.attr };
            buf.put(abs.x + col as i16, abs.y + row, cell.ch, a, clip);
        }
    }
}

/// A labelled field. The label is plain and the field is a sunken box the eye
/// can find without reading anything, which is the only job the colours have.
fn draw_input(
    i: &crate::input::InputLine,
    r: Rect,
    buf: &mut Buffer,
    clip: Rect,
    p: &Palette,
) {
    buf.fill(r, b' ', p.file_path_label, clip);
    if !i.label.is_empty() {
        buf.text(r.x, r.y, &i.label, p.file_path_label, clip);
    }
    let fx = r.x + i.field_x();
    let fw = (r.right() - fx).max(0);
    let a = if i.focused {
        p.file_path_focus
    } else {
        p.file_path
    };
    buf.fill(Rect::new(fx, r.y, fw, 1), b' ', a, clip);
    buf.text(fx, r.y, &i.visible(r.w), a, clip);
    // A field with a past has a ▼ at its end: click it, or press Down.
    if !i.history.is_empty() && fw > 4 {
        buf.put(r.right() - 1, r.y, 0x1F as Glyph, p.file_path_label, clip);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ViewId(u32);

impl ViewId {
    fn ix(self) -> usize {
        self.0 as usize
    }

    /// The number inside, for sending across a boundary the type cannot
    /// cross: a pipe, an interrupt, a WebAssembly import. It is an index and
    /// nothing more — that is the whole reason handles are numbers here.
    pub fn raw(self) -> u32 {
        self.0
    }

    /// The other side of `raw`. Nothing is checked, because nothing can be
    /// without a `Ui`; ask `Ui::is_alive` before using one that came in from
    /// outside.
    pub fn from_raw(n: u32) -> ViewId {
        ViewId(n)
    }
}

struct Node {
    parent: Option<ViewId>,
    children: Vec<ViewId>,
    /// Position and size inside the parent's *client* area. The measure pass
    /// writes this for anything with a dock, so an application neither sets it
    /// nor may rely on what it set.
    rect: Rect,
    kind: Kind,
    dock: Dock,
    alive: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragMode {
    Move,
    Resize,
    /// Dragging a scrollbar's marker.
    Thumb(Axis),
}

/// Where a scrollbar is and what it is showing.
///
/// Positions are along the bar's own axis — a row for a vertical bar, a column
/// for a horizontal one — so one description serves both and there is no
/// second copy of the arithmetic to drift out of step.
#[derive(Clone, Copy)]
struct Bar {
    /// The leading arrow.
    start: i16,
    /// Length including both arrows.
    len: i16,
    /// Cells between the arrows.
    track: i16,
    /// How far the content can travel.
    span: i16,
    /// The marker's offset within the track.
    thumb: i16,
}

impl Bar {
    fn new(start: i16, len: i16, pos: i16, total: i16, page: i16) -> Option<Bar> {
        if len < 3 || total <= page {
            return None;
        }
        let track = len - 2;
        let span = (total - page).max(1);
        let thumb = ((pos as i32 * (track - 1) as i32) / span as i32)
            .clamp(0, (track - 1).max(0) as i32) as i16;
        Some(Bar {
            start,
            len,
            track,
            span,
            thumb,
        })
    }

    fn end(&self) -> i16 {
        self.start + self.len - 1
    }

    fn thumb_at(&self) -> i16 {
        self.start + 1 + self.thumb
    }

    /// Where the content would have to be for the marker to sit under `c`.
    fn pos_for(&self, c: i16) -> i16 {
        let off = (c - self.start - 1).clamp(0, (self.track - 1).max(0));
        ((off as i32 * self.span as i32) / (self.track - 1).max(1) as i32) as i16
    }
}

/// The clipboard as a host program sees it: how many times the core has
/// copied to it, and whether a Paste was chosen that the host should
/// answer from its own clipboard - a browser's, a desktop's - before the
/// core pastes. A host that never asks has the core's clipboard alone, as
/// before.
#[derive(Default)]
pub struct HostClip {
    pub on: bool,
    pub copied: u32,
    pub paste_wanted: Option<ViewId>,
}

/// The boxes on a window's top edge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TitleBox {
    Minimize,
    Zoom,
    Close,
}

/// The boxes on a window's top edge and the column of each one's `[`,
/// from the right-hand corner leftwards: close, zoom, minimize - where
/// every windowing system since has put them, and where the hand looks.
/// One list for the drawing and the clicking alike.
fn title_boxes(w: &Window, abs: Rect) -> Vec<(TitleBox, i16)> {
    let mut v = Vec::new();
    let mut x = abs.right() - 5;
    if w.closable && abs.w >= 8 {
        v.push((TitleBox::Close, x));
        x -= 3;
    }
    if w.zoomable && abs.w >= 12 {
        v.push((TitleBox::Zoom, x));
        x -= 3;
    }
    if w.minimizable && !w.modal && abs.w >= 18 {
        v.push((TitleBox::Minimize, x));
    }
    v
}

/// How many frames a window takes to fall into its bar, or rise out of it.
const ANIM_STEPS: u8 = 6;

/// A window on its way to its bar in the corner, or back: drawn as a box
/// with its title, from `from` to `to`, a step for every TICK.
struct Anim {
    id: ViewId,
    from: Rect,
    to: Rect,
    step: u8,
    minimizing: bool,
}

struct Drag {
    id: ViewId,
    mode: DragMode,
    /// Pointer position when the drag began.
    from: Point,
    /// Window rectangle when the drag began.
    orig: Rect,
}

/// A window being moved or resized from the keyboard: the classic
/// Ctrl+F5. Arrows move it, Shift+arrows resize it, Enter keeps the
/// result, Escape puts `orig` back.
struct Sizing {
    id: ViewId,
    orig: Rect,
}

/// The window list Alt+0 opens: the dialog, its list, and which window
/// each row stands for, front to back.
struct WindowList {
    dialog: ViewId,
    list: ViewId,
    windows: Vec<ViewId>,
}

/// Commands from this number up are the toolkit's own: the buttons of the
/// dialogs it builds for itself send them, and `take_pressed` acts on
/// them instead of handing them out.
pub const CM_INTERNAL: Cmd = 0xFF00;
const CM_WINLIST_OK: Cmd = 0xFF01;
const CM_WINLIST_CANCEL: Cmd = 0xFF02;
/// The rows of a history list, one command each: 0xFF10 + the row.
const CM_HISTORY: Cmd = 0xFF10;

// What an editor puts on the bars (`edit::offer`), and the buttons of the
// Find and Replace dialogs the core builds for it. They are the toolkit's
// own, like the window list's, and never reach the program.
const CM_ED_UNDO: Cmd = 0xFF40;
const CM_ED_REDO: Cmd = 0xFF41;
const CM_ED_CUT: Cmd = 0xFF42;
const CM_ED_COPY: Cmd = 0xFF43;
const CM_ED_PASTE: Cmd = 0xFF44;
const CM_ED_SELECT_ALL: Cmd = 0xFF45;
const CM_ED_FIND: Cmd = 0xFF46;
const CM_ED_FIND_NEXT: Cmd = 0xFF47;
const CM_ED_REPLACE: Cmd = 0xFF48;
const CM_ED_WRAP: Cmd = 0xFF49;
const CM_ED_READONLY: Cmd = 0xFF4A;
const CM_ED_HEX: Cmd = 0xFF4B;
const CM_ED_KEYS: Cmd = 0xFF4C;
const CM_SEARCH_GO: Cmd = 0xFF50;
const CM_SEARCH_REPLACE: Cmd = 0xFF51;
const CM_SEARCH_ALL: Cmd = 0xFF52;
const CM_SEARCH_CANCEL: Cmd = 0xFF53;
const CM_NOTICE_OK: Cmd = 0xFF54;
/// The context menu a right click opens over a text or a console: its
/// commands act on that view, whichever window is in front by then.
const CM_CTX_UNDO: Cmd = 0xFF58;
const CM_CTX_CUT: Cmd = 0xFF59;
const CM_CTX_COPY: Cmd = 0xFF5A;
const CM_CTX_PASTE: Cmd = 0xFF5B;
const CM_CTX_SELECT_ALL: Cmd = 0xFF5C;
/// Edit > Syntax: 0xFF60 is None, 0xFF61 on the languages in list order.
const CM_ED_SYNTAX: Cmd = 0xFF60;
const SYNTAX_MENU_MAX: Cmd = 64;

/// A command the editor answers itself.
fn is_editor_cmd(cmd: Cmd) -> bool {
    (CM_ED_UNDO..=CM_ED_KEYS).contains(&cmd) || (CM_ED_SYNTAX..CM_ED_SYNTAX + SYNTAX_MENU_MAX).contains(&cmd)
}

/// The Find or Replace dialog while it is up, and the editor it searches.
struct Search {
    dialog: ViewId,
    find: ViewId,
    with: Option<ViewId>,
    options: ViewId,
    target: ViewId,
}

/// What was searched for last, so Find next can go on without a dialog.
struct LastSearch {
    pattern: Vec<Glyph>,
    case_sensitive: bool,
    whole_word: bool,
}

pub struct Ui {
    nodes: Vec<Node>,
    root: ViewId,
    drag: Option<Drag>,
    sizing: Option<Sizing>,
    winlist: Option<WindowList>,
    /// The input line whose history is open as a panel.
    history_for: Option<ViewId>,
    /// The desktop's own keys, on by default: Alt+1..9 bring a numbered
    /// window to the front, Alt+0 lists them, Shift+F6 is the previous
    /// window, Ctrl+F5 moves or resizes the active one from the keyboard.
    /// An application that wants those keys for itself turns this off
    /// and calls the primitives from its own table.
    pub desktop_keys: bool,
    pub palette: Palette,
    /// Which arrangement of keys is in force. Data, not code.
    pub keymap: Keymap,
    /// One clipboard for the whole tree, so text moves between views. The
    /// classic toolkits used a hidden editor for this; a list of lines does the same job
    /// without the machinery.
    pub clipboard: Vec<Vec<Glyph>>,
    /// A command chosen from a menu, waiting to be collected.
    ///
    /// The core has no callbacks and will not grow any: a call back into the
    /// application would have to cross an interrupt or a WebAssembly boundary
    /// one day, and neither carries a closure. The application asks instead —
    /// the same bargain as a help page's `pending` link.
    command: Option<Cmd>,
    /// An item picked with the mouse, held back for a moment.
    ///
    /// A click that acts instantly leaves nobody sure they hit anything: the
    /// panel is gone before the eye registers which line was under the
    /// pointer. The classic DOS menus lit the item up first and then acted, and the
    /// pause is the whole of the feedback. The waiting is the backend's job —
    /// the core has no clock and must not grow one.
    pending_pick: Option<(ViewId, Cmd)>,
    /// An editor's Find or Replace dialog, while one is up.
    search: Option<Search>,
    last_search: Option<LastSearch>,
    /// The box that says a search found nothing, or how many it replaced.
    notice: Option<ViewId>,
    /// Ctrl+Q has been pressed in a classic editor, and the next key says
    /// what for: the old DOS editors' two-key commands.
    chord: bool,
    /// The languages texts can be coloured as: the core's own, read from
    /// `syntax.ini` the first time one is asked for, then the program's.
    /// A text holds an index, so nothing is ever taken out; a later one of
    /// the same name is found first.
    syntaxes: Vec<crate::syntax::Syntax>,
    syntaxes_loaded: bool,
    /// A text or a console being selected with the mouse: the button went
    /// down on it and has not come up.
    selecting: Option<ViewId>,
    /// The view a context menu was opened over, for the commands it sends.
    context_for: Option<ViewId>,
    /// A list or a tree whose scroll bar marker is held by the mouse.
    bar_drag: Option<ViewId>,
    /// The clipboard as the host program shares it. See `HostClip`.
    pub host_clip: HostClip,
    /// Minimized windows, in the order their bars stack up from the
    /// bottom right corner.
    minimized: Vec<ViewId>,
    /// A window falling into its bar, or rising out of it.
    anim: Option<Anim>,
}

impl Ui {
    pub fn new(w: i16, h: i16) -> Self {
        let root = Node {
            parent: None,
            children: Vec::new(),
            rect: Rect::sized(w, h),
            kind: Kind::Desktop(Desktop::default()),
            dock: Dock::Fill,
            alive: true,
        };
        Ui {
            nodes: vec![root],
            root: ViewId(0),
            drag: None,
            sizing: None,
            winlist: None,
            history_for: None,
            desktop_keys: true,
            palette: Palette::classic(),
            keymap: Keymap::Modern,
            clipboard: Vec::new(),
            command: None,
            pending_pick: None,
            search: None,
            last_search: None,
            notice: None,
            chord: false,
            syntaxes: Vec::new(),
            syntaxes_loaded: false,
            selecting: None,
            context_for: None,
            bar_drag: None,
            host_clip: HostClip::default(),
            minimized: Vec::new(),
            anim: None,
        }
    }

    pub fn root(&self) -> ViewId {
        self.root
    }

    pub fn insert(&mut self, parent: ViewId, rect: Rect, kind: Kind) -> ViewId {
        let id = ViewId(self.nodes.len() as u32);
        // A window on the desktop takes the first free number from 1 to 9,
        // as the classic DOS desktops did, so Alt+that brings it to the front. A modal
        // one does not: it is the only window there is while it is up. A
        // tenth window has no number and is reached from the list.
        let mut kind = kind;
        if parent == self.root {
            if let Kind::Window(w) = &mut kind {
                if w.number.is_none() && !w.modal {
                    let taken: Vec<u8> = self.nodes[self.root.ix()]
                        .children
                        .iter()
                        .filter_map(|c| match &self.nodes[c.ix()].kind {
                            Kind::Window(o) => o.number,
                            _ => None,
                        })
                        .collect();
                    w.number = (1..=9).find(|n| !taken.contains(n));
                }
            }
        }
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            rect,
            kind,
            dock: Dock::Fill,
            alive: true,
        });
        self.nodes[parent.ix()].children.push(id);
        id
    }

    /// Whether a handle names a view that exists and has not been closed.
    ///
    /// Every other accessor trusts its handle, which is right for handles the
    /// program made itself and wrong for ones that arrived over a wire.
    pub fn is_alive(&self, id: ViewId) -> bool {
        self.nodes.get(id.ix()).is_some_and(|n| n.alive)
    }

    pub fn kind(&self, id: ViewId) -> &Kind {
        &self.nodes[id.ix()].kind
    }

    pub fn kind_mut(&mut self, id: ViewId) -> &mut Kind {
        &mut self.nodes[id.ix()].kind
    }

    pub fn rect(&self, id: ViewId) -> Rect {
        self.nodes[id.ix()].rect
    }

    pub fn set_rect(&mut self, id: ViewId, r: Rect) {
        self.nodes[id.ix()].rect = r;
    }

    pub fn set_dock(&mut self, id: ViewId, dock: Dock) {
        self.nodes[id.ix()].dock = dock;
    }

    pub fn children(&self, id: ViewId) -> &[ViewId] {
        &self.nodes[id.ix()].children
    }

    /// The screen grew or shrank. The desktop always fills it; windows keep
    /// their place but are pushed back inside if they would fall off.
    /// The desktop changed size.
    ///
    /// Every window keeps its proportions: its edges sit at the same
    /// fractions of the work area as before, so a layout of three windows
    /// side by side is still three windows side by side, each its share of
    /// the screen, and an editor that filled the screen still fills it. A
    /// window with a fixed size keeps that size and its middle stays at the
    /// same fraction; a centred one is centred again by the measure pass.
    /// The new place is worked out from where the window was put, not from
    /// the last resize, so a browser made smaller and back puts it back.
    ///
    /// It used to grow and shrink every window by the same number of cells,
    /// corners where they were: a page made a little narrower pushed every
    /// window into its corner and left the middle empty. Before that it
    /// shrank windows to fit and never grew one back, so a console taken
    /// down to a sliver and restored came back empty.
    pub fn resize(&mut self, w: i16, h: i16) {
        let old_work = self.work_area();
        self.nodes[self.root.ix()].rect = Rect::sized(w, h);
        let kids: Vec<ViewId> = self.nodes[self.root.ix()].children.clone();
        for id in kids {
            match &self.nodes[id.ix()].kind {
                Kind::MenuBar(_) => {
                    self.nodes[id.ix()].rect = Rect::new(0, 0, w, 1);
                    continue;
                }
                Kind::Status(_) => {
                    self.nodes[id.ix()].rect = Rect::new(0, h - 1, w, 1);
                    continue;
                }
                _ => {}
            }
        }
        // The bars are in place: the work area is the new one.
        let new_work = self.work_area();
        for id in self.nodes[self.root.ix()].children.clone() {
            let cur = self.nodes[id.ix()].rect;
            let Kind::Window(win) = &mut self.nodes[id.ix()].kind else {
                continue;
            };
            // A zoomed window is the work area; it follows by definition,
            // and the place it goes back to keeps its proportions.
            if let Some(prev) = win.unzoomed {
                win.unzoomed = Some(rescale(prev, old_work, new_work, win));
                win.home = None;
                self.nodes[id.ix()].rect = new_work;
                continue;
            }
            let (base, base_work) = match win.home {
                Some((base, base_work, fitted)) if fitted == cur => (base, base_work),
                _ => (cur, old_work),
            };
            let r = rescale(base, base_work, new_work, win);
            win.home = Some((base, base_work, r));
            self.nodes[id.ix()].rect = r;
        }
    }

    // ---------------------------------------------------------------- geometry

    /// Absolute position of a view's own rectangle.
    fn abs_origin(&self, id: ViewId) -> Point {
        let n = &self.nodes[id.ix()];
        match n.parent {
            None => Point::new(n.rect.x, n.rect.y),
            Some(p) => {
                let po = self.abs_origin(p);
                let inset = self.nodes[p.ix()].kind.client_inset();
                Point::new(po.x + inset + n.rect.x, po.y + inset + n.rect.y)
            }
        }
    }

    pub fn abs_rect(&self, id: ViewId) -> Rect {
        let o = self.abs_origin(id);
        let r = self.nodes[id.ix()].rect;
        Rect::new(o.x, o.y, r.w, r.h)
    }

    /// Where windows are allowed to be.
    ///
    /// The whole screen less anything nailed to an edge. Without this a window
    /// can be dragged up until its title bar is under the menu bar, and then
    /// there is nothing left to take hold of — the window is still there, still
    /// on top, and completely out of reach.
    pub fn work_area(&self) -> Rect {
        let mut r = self.nodes[self.root.ix()].rect;
        for c in &self.nodes[self.root.ix()].children {
            match self.nodes[c.ix()].kind {
                Kind::MenuBar(_) => {
                    r.y += 1;
                    r.h -= 1;
                }
                Kind::Status(_) => {
                    r.h -= 1;
                }
                _ => {}
            }
        }
        r
    }

    /// Where a window's inside is on the screen, and whether anything is
    /// drawn over any of it: a window above it, that window's shadow, an
    /// open menu.
    ///
    /// For a program that puts something of its own where the window is -
    /// a web page laying an emulator's picture over a window's inside. The
    /// picture is not drawn by us, so it cannot be clipped by us either; the
    /// program shows it while nothing covers the window and hides it while
    /// something does, and a menu dropped over it is still a menu you can
    /// read.
    pub fn place(&self, id: ViewId) -> Option<(Rect, bool)> {
        if !self.is_alive(id) || !matches!(self.nodes[id.ix()].kind, Kind::Window(_)) {
            return None;
        }
        let inside = self.client_abs(id);
        // Put away, or on its way there or back: nothing of it is showing.
        if self.is_minimized(id) || self.anim.as_ref().is_some_and(|a| a.id == id) {
            return Some((inside, true));
        }
        let mut order: Vec<ViewId> = self.nodes[self.root.ix()].children.clone();
        order.sort_by_key(|c| self.nodes[c.ix()].kind.layer());
        let at = order.iter().position(|&c| c == id)?;
        let covered = order[at + 1..].iter().any(|&c| {
            let mut r = self.abs_rect(c);
            match &self.nodes[c.ix()].kind {
                Kind::Window(w) => {
                    if w.shadow {
                        r.w += 2;
                        r.h += 1;
                    }
                }
                Kind::MenuBox(_) => {
                    r.w += 2;
                    r.h += 1;
                }
                _ => return false,
            }
            !r.intersect(&inside).is_empty()
        });
        Some((inside, covered))
    }

    /// The area inside a view that its children may use.
    fn client_abs(&self, id: ViewId) -> Rect {
        self.abs_rect(id).inset(self.nodes[id.ix()].kind.client_inset())
    }

    // ------------------------------------------------------------------- order

    /// The frontmost window is the last child of the desktop, so "activate"
    /// is just "move to the end". Painter's order does the rest.
    pub fn activate(&mut self, id: ViewId) {
        // A window put away comes back to be in front: Alt+its number, the
        // window list, a program bringing it up.
        if self.is_minimized(id) {
            return self.restore(id);
        }
        let Some(parent) = self.nodes[id.ix()].parent else {
            return;
        };
        let kids = &mut self.nodes[parent.ix()].children;
        if kids.last() == Some(&id) {
            return;
        }
        if let Some(pos) = kids.iter().position(|k| *k == id) {
            let v = kids.remove(pos);
            kids.push(v);
        }
    }

    /// The topmost modal window, if one is up.
    pub fn modal(&self) -> Option<ViewId> {
        self.nodes[self.root.ix()]
            .children
            .iter()
            .rev()
            .copied()
            .find(|id| {
                self.nodes[id.ix()].alive
                    && matches!(&self.nodes[id.ix()].kind, Kind::Window(w) if w.modal)
            })
    }

    pub fn active_window(&self) -> Option<ViewId> {
        // A modal window is the active one for as long as it is up, whatever
        // else has been clicked. That single line is most of what modal means.
        if let Some(m) = self.modal() {
            return Some(m);
        }
        self.nodes[self.root.ix()]
            .children
            .iter()
            .rev()
            .copied()
            .find(|id| {
                self.nodes[id.ix()].alive && matches!(&self.nodes[id.ix()].kind, Kind::Window(w) if !w.minimized)
            })
    }

    fn is_minimized(&self, id: ViewId) -> bool {
        matches!(&self.nodes[id.ix()].kind, Kind::Window(w) if w.minimized)
    }

    // -------------------------------------------------------------- minimize

    /// Put a window away: it falls, as a box with its title, into a bar of
    /// its own in the bottom right corner, the bars stacking upwards. The
    /// window in front of the rest becomes the active one.
    pub fn minimize(&mut self, id: ViewId) {
        if !self.is_alive(id) || self.is_minimized(id) {
            return;
        }
        let from = self.abs_rect(id);
        let Kind::Window(w) = &mut self.nodes[id.ix()].kind else { return };
        if !w.minimizable || w.modal {
            return;
        }
        w.minimized = true;
        self.finish_anim();
        self.drag = None;
        self.close_menu();
        self.minimized.push(id);
        let to = self.bar_rect(id);
        self.anim = Some(Anim { id, from, to, step: 0, minimizing: true });
    }

    /// Bring a minimized window back where it was, rising out of its bar,
    /// and in front.
    pub fn restore(&mut self, id: ViewId) {
        if !self.is_minimized(id) {
            return;
        }
        self.finish_anim();
        let from = self.bar_rect(id);
        self.minimized.retain(|m| *m != id);
        let to = self.abs_rect(id);
        self.anim = Some(Anim { id, from, to, step: 0, minimizing: false });
    }

    /// Whether a window is falling or rising: each frame is held, and each
    /// TICK is a step.
    pub fn animating(&self) -> bool {
        self.anim.is_some()
    }

    fn step_anim(&mut self) {
        let done = match &mut self.anim {
            Some(a) => {
                a.step += 1;
                a.step >= ANIM_STEPS
            }
            None => return,
        };
        if done {
            self.finish_anim();
        }
    }

    /// The animation to its end at once: the window is in its bar, or back.
    fn finish_anim(&mut self) {
        let Some(a) = self.anim.take() else { return };
        if !a.minimizing && self.is_alive(a.id) {
            if let Kind::Window(w) = &mut self.nodes[a.id.ix()].kind {
                w.minimized = false;
            }
            self.activate(a.id);
        }
    }

    /// Where a minimized window's bar is: the bottom right of the work
    /// area, one row each, upwards in the order they were put away; a
    /// column to the left when one fills.
    fn bar_rect(&self, id: ViewId) -> Rect {
        let work = self.work_area();
        let i = self.minimized.iter().position(|m| *m == id).unwrap_or(self.minimized.len()) as i16;
        let title = match &self.nodes[id.ix()].kind {
            Kind::Window(w) => w.title.chars().count() as i16,
            _ => 0,
        };
        let w = (title + 10).clamp(18, 32).min(work.w.max(1));
        let per = work.h.max(1);
        let (col, row) = (i / per, i % per);
        Rect::new((work.right() - w * (col + 1)).max(work.x), work.bottom() - 1 - row, w, 1)
    }

    /// The minimized window whose bar is under the pointer.
    fn bar_at(&self, p: Point) -> Option<ViewId> {
        self.minimized.iter().rev().copied().find(|&id| self.is_alive(id) && self.bar_rect(id).contains(p))
    }

    /// A press on a bar: its `[■]` closes the window, as the close box
    /// would; anywhere else brings it back.
    fn bar_click(&mut self, id: ViewId, p: Point) {
        let r = self.bar_rect(id);
        let (closable, close_cmd) = match &self.nodes[id.ix()].kind {
            Kind::Window(w) => (w.closable, w.close_cmd),
            _ => return,
        };
        if closable && p.x >= r.right() - 4 && p.x <= r.right() - 2 {
            if close_cmd != 0 {
                // The program decides, as for the close box, and is asked
                // about a window it can see.
                self.restore(id);
                self.finish_anim();
                self.command = Some(close_cmd);
            } else {
                self.close(id);
            }
            return;
        }
        self.restore(id);
    }

    fn draw_bars(&self, buf: &mut Buffer, clip: Rect) {
        for &id in &self.minimized {
            if !self.is_alive(id) || self.anim.as_ref().is_some_and(|a| a.id == id) {
                continue;
            }
            let Kind::Window(w) = &self.nodes[id.ix()].kind else { continue };
            let r = self.bar_rect(id);
            let p = self.palette.window(w.palette);
            buf.fill(r, b' ', p.frame_active, clip);
            let room = (r.w - 9).max(0) as usize;
            let title: String = w.title.chars().take(room).collect();
            buf.text(r.x + 1, r.y, &title, p.frame_active, clip);
            // Up, to come back; the square, to close - the same boxes, in
            // the same places, as on the window's own top edge.
            let rx = r.right() - 7;
            buf.put(rx, r.y, b'[', p.frame_active, clip);
            buf.put(rx + 1, r.y, 0x18 as Glyph, p.handle, clip);
            buf.put(rx + 2, r.y, b']', p.frame_active, clip);
            if w.closable {
                let cx = r.right() - 4;
                buf.put(cx, r.y, b'[', p.frame_active, clip);
                buf.put(cx + 1, r.y, glyph::SQUARE, p.handle, clip);
                buf.put(cx + 2, r.y, b']', p.frame_active, clip);
            }
        }
    }

    /// The window in flight: a box with its title, between where it was
    /// and where it is going, nearer the end with every step.
    fn draw_anim(&self, buf: &mut Buffer, clip: Rect) {
        let Some(a) = &self.anim else { return };
        let Kind::Window(w) = &self.nodes[a.id.ix()].kind else { return };
        let t = a.step as i32 + 1;
        let n = ANIM_STEPS as i32 + 1;
        let lerp = |f: i16, to: i16| (f as i32 + (to as i32 - f as i32) * t / n) as i16;
        let r = Rect::new(lerp(a.from.x, a.to.x), lerp(a.from.y, a.to.y), lerp(a.from.w, a.to.w).max(4), lerp(a.from.h, a.to.h).max(1));
        let p = self.palette.window(w.palette);
        let fa = p.frame_active;
        buf.fill(r, b' ', p.body, clip);
        if r.h >= 2 {
            buf.hline(r.x + 1, r.y, r.w - 2, glyph::SL_H, fa, clip);
            buf.hline(r.x + 1, r.bottom() - 1, r.w - 2, glyph::SL_H, fa, clip);
            buf.vline(r.x, r.y + 1, r.h - 2, glyph::SL_V, fa, clip);
            buf.vline(r.right() - 1, r.y + 1, r.h - 2, glyph::SL_V, fa, clip);
            buf.put(r.x, r.y, glyph::SL_TL, fa, clip);
            buf.put(r.right() - 1, r.y, glyph::SL_TR, fa, clip);
            buf.put(r.x, r.bottom() - 1, glyph::SL_BL, fa, clip);
            buf.put(r.right() - 1, r.bottom() - 1, glyph::SL_BR, fa, clip);
        } else {
            buf.fill(r, b' ', fa, clip);
        }
        let room = (r.w - 4).max(0) as usize;
        let title: String = w.title.chars().take(room).collect();
        if !title.is_empty() {
            let tx = r.x + (r.w - title.chars().count() as i16) / 2;
            buf.text(tx, r.y, &title, fa, clip);
        }
    }

    pub fn take_command(&mut self) -> Option<Cmd> {
        let cmd = self.command.take()?;
        if cmd >= CM_HISTORY && cmd < CM_HISTORY + 32 {
            // A row of a history list: into its field, and no further.
            let row = (cmd - CM_HISTORY) as usize;
            if let Some(id) = self.history_for.take() {
                if let Kind::Input(i) = &mut self.nodes[id.ix()].kind {
                    if let Some(t) = i.history.get(row).cloned() {
                        i.set_text(&t);
                    }
                }
            }
            return None;
        }
        if cmd >= CM_INTERNAL {
            return None;
        }
        Some(cmd)
    }

    /// Drop an input line's history under it as a panel: a row a line,
    /// newest first, chosen like a menu item. Nothing happens for a
    /// field that has none.
    pub fn open_history(&mut self, id: ViewId) {
        let (items, abs) = match &self.nodes[id.ix()].kind {
            Kind::Input(i) if !i.history.is_empty() => {
                let items: Vec<crate::menu::MenuItem> = i
                    .history
                    .iter()
                    .take(32)
                    .enumerate()
                    .map(|(n, h)| crate::menu::MenuItem::new(h, "", CM_HISTORY + n as Cmd))
                    .collect();
                (items, self.abs_rect(id))
            }
            _ => return,
        };
        self.close_menu();
        let mut b = MenuBox::new(items);
        b.parent = None;
        let (w, h) = (b.width(), b.height());
        let screen = self.nodes[self.root.ix()].rect;
        let fx = match &self.nodes[id.ix()].kind {
            Kind::Input(i) => abs.x + i.field_x(),
            _ => abs.x,
        };
        let x = fx.min((screen.w - w).max(0));
        let y = (abs.y + 1).min((screen.h - h).max(0));
        let root = self.root;
        self.insert(root, Rect::new(x, y, w, h), Kind::MenuBox(b));
        self.history_for = Some(id);
    }

    /// Find in a text view: the next match after the caret, selected.
    pub fn find_text(&mut self, id: ViewId, pat: &[Glyph], case_sensitive: bool, whole_word: bool) -> bool {
        match &mut self.nodes[id.ix()].kind {
            Kind::Text(t) => t.find(pat, case_sensitive, whole_word),
            _ => false,
        }
    }

    /// Replace the selected match and find the next: (replaced, found).
    pub fn replace_text(&mut self, id: ViewId, pat: &[Glyph], with: &[Glyph], case_sensitive: bool, whole_word: bool) -> (bool, bool) {
        match &mut self.nodes[id.ix()].kind {
            Kind::Text(t) => t.replace(pat, with, case_sensitive, whole_word),
            _ => (false, false),
        }
    }

    /// Replace every match; how many.
    pub fn replace_all_text(&mut self, id: ViewId, pat: &[Glyph], with: &[Glyph], case_sensitive: bool, whole_word: bool) -> u16 {
        match &mut self.nodes[id.ix()].kind {
            Kind::Text(t) => t.replace_all(pat, with, case_sensitive, whole_word),
            _ => 0,
        }
    }

    // ------------------------------------------------------------ the editor
    //
    // What an editor offers (`TextView::offers`) it brings with it: while its
    // window is active the Edit menu has its items, the status line says
    // where the caret is, and the keys work - and the core answers all of
    // it itself, Find and Replace dialogs included. A program sets the bits
    // once and has an editor; it writes no handler for any of this.

    /// The text a window is about: its first child, the text a hex view is
    /// standing in for, or the memo holding the focus in a dialog.
    fn window_editor(&self, win: ViewId) -> Option<ViewId> {
        let first = *self.nodes[win.ix()].children.first()?;
        match &self.nodes[first.ix()].kind {
            Kind::Text(_) => Some(first),
            Kind::Hex(h) => h.source.filter(|s| self.is_alive(*s)),
            _ => self
                .focused()
                .filter(|f| matches!(self.nodes[f.ix()].kind, Kind::Text(_))),
        }
    }

    fn editor_keymap(&self, t: &TextView) -> Keymap {
        t.keymap.unwrap_or(self.keymap)
    }

    /// The Edit menu an editor brings, or `None` when it offers nothing.
    fn editor_menu(&self, tid: ViewId) -> Option<crate::menu::MenuItem> {
        use crate::edit::offer::*;
        use crate::menu::MenuItem;
        let Kind::Text(t) = &self.nodes[tid.ix()].kind else { return None };
        let o = t.offers;
        if o & ALL == 0 {
            return None;
        }
        let classic = self.editor_keymap(t) == Keymap::Classic;
        let keys = |modern: &str, old: &str| if classic { old.to_string() } else { modern.to_string() };
        let hex = t.hex.is_some();
        let (ro, text) = (t.readonly, !hex);
        let sel = t.selection().is_some();
        let item = |label: &str, key: String, cmd: Cmd, hint: &str, on: bool| {
            let mut m = MenuItem::new(label, &key, cmd).hint(hint);
            m.enabled = on;
            m
        };
        let mut items: Vec<MenuItem> = Vec::new();
        if o & EDIT != 0 {
            items.push(item("~U~ndo", keys("Ctrl+Z", "Ctrl+U"), CM_ED_UNDO,
                "Take back the last change", text && !ro && !t.undo_stack.is_empty()));
            items.push(item("~R~edo", keys("Ctrl+Y", ""), CM_ED_REDO,
                "Put back what Undo took back", text && !ro && !t.redo_stack.is_empty()));
            items.push(MenuItem::line());
            items.push(item("Cu~t~", keys("Ctrl+X", "Shift+Del"), CM_ED_CUT,
                "The selected text to the clipboard, and out of the text", text && !ro && sel));
            items.push(item("~C~opy", keys("Ctrl+C", "Ctrl+Ins"), CM_ED_COPY,
                "The selected text to the clipboard", text && sel));
            items.push(item("~P~aste", keys("Ctrl+V", "Shift+Ins"), CM_ED_PASTE,
                "What is on the clipboard, into the text at the caret", text && !ro && (self.host_clip.on || !self.clipboard.is_empty())));
            items.push(item("Select ~a~ll", keys("Ctrl+A", ""), CM_ED_SELECT_ALL,
                "The whole text, to copy it or type over it", text));
        }
        if o & (FIND | REPLACE) != 0 {
            if !items.is_empty() {
                items.push(MenuItem::line());
            }
            if o & FIND != 0 {
                items.push(item("~F~ind...", keys("Ctrl+F", "Ctrl+Q F"), CM_ED_FIND,
                    "Look for a word or a phrase in this text", text));
                items.push(item("Find ~n~ext", "Ctrl+L".into(), CM_ED_FIND_NEXT,
                    "The next place the last search matches", text && self.last_search.is_some()));
            }
            if o & REPLACE != 0 {
                items.push(item("R~e~place...", keys("Ctrl+H", "Ctrl+Q A"), CM_ED_REPLACE,
                    "Find a word and put another one in its place", text && !ro));
            }
        }
        if o & (WRAP | READONLY | HEX | KEYS | SYNTAX) != 0 {
            if !items.is_empty() {
                items.push(MenuItem::line());
            }
            if o & SYNTAX != 0 {
                let current = t.syntax.and_then(|c| self.syntaxes.get(c as usize)).map(|s| s.name.as_str());
                let mut langs = vec![item("~N~one", String::new(), CM_ED_SYNTAX, "Plain text, all in one colour", text)
                    .checked(current.is_none())];
                for (n, &i) in self.syntax_list().iter().enumerate().take(SYNTAX_MENU_MAX as usize - 1) {
                    let name = &self.syntaxes[i].name;
                    let hint = format!("Colour it as {name}: its keywords, comments, strings and numbers");
                    let on = current.is_some_and(|c| c.eq_ignore_ascii_case(name));
                    langs.push(item(name, String::new(), CM_ED_SYNTAX + 1 + n as Cmd, &hint, text).checked(on));
                }
                let mut sub = MenuItem::sub("S~y~ntax", langs);
                sub.hint = "Which language the text is coloured as".into();
                items.push(sub);
            }
            if o & WRAP != 0 {
                items.push(item("~W~ord wrap", String::new(), CM_ED_WRAP,
                    "Long lines folded at the window's edge; the file itself is not changed", text)
                    .checked(t.wrap));
            }
            if o & READONLY != 0 {
                items.push(item("Read ~o~nly", String::new(), CM_ED_READONLY,
                    "Look without changing anything: typing does nothing", text)
                    .checked(ro));
            }
            if o & HEX != 0 {
                items.push(item("~H~ex view", String::new(), CM_ED_HEX,
                    "The same text as bytes, each one in hexadecimal", true)
                    .checked(hex));
            }
            if o & KEYS != 0 {
                items.push(item("C~l~assic keys", String::new(), CM_ED_KEYS,
                    "WordStar keys, as the old DOS editors had them (Ctrl+S D E X move); off: Ctrl+C, V, Z", text)
                    .checked(classic));
            }
        }
        Some(MenuItem::sub("~E~dit", items))
    }

    /// The keys an editor puts on the status line - only when there is no
    /// menu bar to show them, because there is room on one line for the
    /// program's keys or for these, rarely for both.
    fn editor_status(&self, tid: ViewId) -> Vec<crate::status::StatusItem> {
        use crate::edit::offer::*;
        use crate::status::StatusItem;
        let mut out = Vec::new();
        let Kind::Text(t) = &self.nodes[tid.ix()].kind else { return out };
        if self.menu_bar_id().is_some() || t.hex.is_some() {
            return out;
        }
        let classic = self.editor_keymap(t) == Keymap::Classic;
        if t.offers & FIND != 0 {
            out.push(StatusItem::new(if classic { "~Ctrl+Q F~ Find" } else { "~Ctrl+F~ Find" }, None, CM_ED_FIND));
        }
        if t.offers & REPLACE != 0 && !t.readonly {
            out.push(StatusItem::new(if classic { "~Ctrl+Q A~ Replace" } else { "~Ctrl+H~ Replace" }, None, CM_ED_REPLACE));
        }
        out
    }

    /// Where the caret is and how the editor is set, for the right end of
    /// the status line: `Ln 12 Col 5  Wrap`.
    fn editor_indicator(&self, tid: ViewId) -> String {
        let Kind::Text(t) = &self.nodes[tid.ix()].kind else { return String::new() };
        if t.offers == 0 {
            return String::new();
        }
        if let Some(h) = t.hex {
            return match &self.nodes[h.ix()].kind {
                Kind::Hex(hv) => format!("Hex  byte {} of {} ", hv.cursor + 1, hv.bytes.len()),
                _ => String::new(),
            };
        }
        let mut s = format!("Ln {} Col {}", t.cur.y as i32 + 1, t.cur.x as i32 + 1);
        if let Some(lang) = t.syntax.and_then(|c| self.syntaxes.get(c as usize)) {
            s.push_str("  ");
            s.push_str(&lang.name);
        }
        if t.wrap {
            s.push_str("  Wrap");
        }
        if t.readonly {
            s.push_str("  Read only");
        }
        if self.editor_keymap(t) == Keymap::Classic {
            s.push_str("  Classic keys");
        }
        s.push(' ');
        s
    }

    /// One of the editor's own commands, on the active window's editor.
    fn editor_command(&mut self, cmd: Cmd) {
        use crate::edit::Cmd as E;
        let Some(win) = self.active_window() else { return };
        let Some(tid) = self.window_editor(win) else { return };
        let hex = matches!(&self.nodes[tid.ix()].kind, Kind::Text(t) if t.hex.is_some());
        if hex && cmd != CM_ED_HEX {
            return;
        }
        match cmd {
            CM_ED_UNDO => self.run_edit(tid, E::Undo),
            CM_ED_REDO => self.run_edit(tid, E::Redo),
            CM_ED_CUT => self.run_edit(tid, E::Cut),
            CM_ED_COPY => self.run_edit(tid, E::Copy),
            CM_ED_PASTE => self.run_edit(tid, E::Paste),
            CM_ED_SELECT_ALL => self.run_edit(tid, E::SelectAll),
            CM_ED_FIND => self.open_search(tid, false),
            CM_ED_REPLACE => self.open_search(tid, true),
            CM_ED_FIND_NEXT => self.search_again(tid),
            CM_ED_WRAP => {
                let r = self.abs_rect(tid);
                if let Kind::Text(t) = &mut self.nodes[tid.ix()].kind {
                    t.width = r.w;
                    let on = !t.wrap;
                    t.set_wrap(on, r.h);
                }
                self.follow_x(tid);
            }
            CM_ED_READONLY => {
                let ro = matches!(&self.nodes[tid.ix()].kind, Kind::Text(t) if t.readonly);
                self.set_readonly(tid, !ro);
                if ro && self.focused().is_none() {
                    self.focus_first();
                }
            }
            CM_ED_HEX => self.toggle_hex(tid),
            CM_ED_KEYS => {
                let now = match &self.nodes[tid.ix()].kind {
                    Kind::Text(t) => self.editor_keymap(t),
                    _ => return,
                };
                if let Kind::Text(t) = &mut self.nodes[tid.ix()].kind {
                    t.keymap = Some(if now == Keymap::Classic { Keymap::Modern } else { Keymap::Classic });
                }
            }
            c if (CM_ED_SYNTAX..CM_ED_SYNTAX + SYNTAX_MENU_MAX).contains(&c) => {
                let ix = (c - CM_ED_SYNTAX) as usize;
                let name = match ix {
                    0 => String::new(),
                    _ => self.syntax_list().get(ix - 1).map(|&i| self.syntaxes[i].name.clone()).unwrap_or_default(),
                };
                self.set_syntax(tid, &name);
            }
            _ => {}
        }
    }

    /// An editing command, as if its key had been pressed.
    fn run_edit(&mut self, tid: ViewId, c: crate::edit::Cmd) {
        self.edit(tid, c, false);
    }

    /// One editing command on a text, and what it means for the clipboard
    /// the host shares: a copy or a cut is counted, so the host can carry
    /// it to its own clipboard; a paste, while the host shares one, waits
    /// for the host to put its clipboard in first (`host_paste`).
    fn edit(&mut self, tid: ViewId, c: crate::edit::Cmd, extend: bool) {
        use crate::edit::Cmd as E;
        let r = self.abs_rect(tid);
        let selected = matches!(&self.nodes[tid.ix()].kind, Kind::Text(t) if t.selection().is_some());
        if c == E::Paste && self.host_clip.on {
            let ro = matches!(&self.nodes[tid.ix()].kind, Kind::Text(t) if t.readonly);
            if !ro {
                self.host_clip.paste_wanted = Some(tid);
            }
            return;
        }
        let clip = &mut self.clipboard;
        if let Kind::Text(t) = &mut self.nodes[tid.ix()].kind {
            t.width = r.w;
            t.exec(c, extend, r.h, clip);
        }
        if selected && matches!(c, E::Copy | E::Cut) {
            self.host_clip.copied = self.host_clip.copied.wrapping_add(1);
        }
        self.follow_x(tid);
    }

    /// The host's clipboard, given: it becomes the core's, and a paste
    /// that was waiting for it happens now. `now` pastes into the text
    /// with the focus even when nothing was waiting - the host's own paste
    /// key, Ctrl+V in a browser. True if something was pasted.
    pub fn host_paste(&mut self, lines: Option<Vec<Vec<Glyph>>>, now: bool) -> bool {
        if let Some(l) = lines {
            if !l.is_empty() {
                self.clipboard = l;
            }
        }
        let target = self.host_clip.paste_wanted.take().filter(|t| self.is_alive(*t)).or_else(|| {
            if !now {
                return None;
            }
            // A field with the focus - Find, a file's name - before the
            // window's text.
            if let Some(f) = self.focused().filter(|f| matches!(self.nodes[f.ix()].kind, Kind::Input(_))) {
                return Some(f);
            }
            let win = self.active_window()?;
            self.window_editor(win)
        });
        let Some(tid) = target else { return false };
        if let Kind::Input(i) = &mut self.nodes[tid.ix()].kind {
            let Some(l) = self.clipboard.first() else { return false };
            i.insert(&glyph_string(l));
            return true;
        }
        if self.clipboard.is_empty() || !matches!(&self.nodes[tid.ix()].kind, Kind::Text(t) if !t.readonly && t.hex.is_none()) {
            return false;
        }
        let on = core::mem::replace(&mut self.host_clip.on, false);
        self.edit(tid, crate::edit::Cmd::Paste, false);
        self.host_clip.on = on;
        true
    }

    /// Scroll sideways to the caret, as the vertical scroll already does.
    /// A folding view has nothing to its right.
    fn follow_x(&mut self, tid: ViewId) {
        let r = self.abs_rect(tid);
        if let Kind::Text(t) = &mut self.nodes[tid.ix()].kind {
            if t.wrapping() {
                t.left = 0;
                t.follow_caret(r.h);
                return;
            }
            if t.cur.x < t.left {
                t.left = t.cur.x;
            } else if t.cur.x >= t.left + r.w {
                t.left = t.cur.x - r.w + 1;
            }
            t.left = t.left.max(0);
        }
    }

    /// The keys an editor's offers bring: Find, Replace, Find next, and
    /// the old DOS editors' Ctrl+Q chords for them. True when the key was one.
    fn editor_key(&mut self, tid: ViewId, k: Key) -> bool {
        use crate::edit::offer::*;
        use crate::event::KeyCode as K;
        let (offers, ro, km) = match &self.nodes[tid.ix()].kind {
            Kind::Text(t) => (t.offers, t.readonly, self.editor_keymap(t)),
            _ => return false,
        };
        if offers & (FIND | REPLACE) == 0 {
            self.chord = false;
            return false;
        }
        if self.chord {
            // The second key of Ctrl+Q something: with Ctrl or without, as
            // the old DOS editors took it. Anything else is let go of quietly.
            self.chord = false;
            if let K::Char(c) = k.code {
                match c.to_ascii_lowercase() {
                    'f' if offers & FIND != 0 => self.open_search(tid, false),
                    'a' if offers & REPLACE != 0 && !ro => self.open_search(tid, true),
                    _ => {}
                }
            }
            return true;
        }
        if !(k.mods.ctrl && !k.mods.alt && !k.mods.shift) {
            return false;
        }
        let K::Char(c) = k.code else { return false };
        match (km, c.to_ascii_lowercase()) {
            (Keymap::Classic, 'q') => self.chord = true,
            (Keymap::Modern, 'f') if offers & FIND != 0 => self.open_search(tid, false),
            (Keymap::Modern, 'h') if offers & REPLACE != 0 && !ro => self.open_search(tid, true),
            (_, 'l') if offers & FIND != 0 => self.search_again(tid),
            _ => return false,
        }
        true
    }

    /// A key for a text view: the editor's own keys first, then its keymap.
    fn text_key(&mut self, id: ViewId, k: Key) {
        if self.editor_key(id, k) {
            return;
        }
        let km = match &self.nodes[id.ix()].kind {
            Kind::Text(t) => self.editor_keymap(t),
            _ => return,
        };
        let Some((cmd, extend)) = km.lookup(k) else { return };
        self.edit(id, cmd, extend);
    }

    /// The Find dialog, or the Replace dialog: a modal the core builds and
    /// answers itself. It starts with the selected words, when a few are
    /// selected, or with what was looked for last.
    fn open_search(&mut self, tid: ViewId, replace: bool) {
        use crate::button::Button;
        use crate::controls::Cluster;
        use crate::input::InputLine;
        if self.modal().is_some() || self.search.is_some() {
            return;
        }
        let (start, case, whole) = {
            let Kind::Text(t) = &self.nodes[tid.ix()].kind else { return };
            let sel = t.selected_text();
            let from_sel = (sel.len() == 1 && !sel[0].is_empty()).then(|| glyph_string(&sel[0]));
            let last = self.last_search.as_ref();
            (
                from_sel.or_else(|| last.map(|l| glyph_string(&l.pattern))).unwrap_or_default(),
                last.is_some_and(|l| l.case_sensitive),
                last.is_some_and(|l| l.whole_word),
            )
        };
        let work = self.work_area();
        let w = 56.min(work.w - 2).max(30);
        let h = if replace { 12 } else { 10 };
        let mut win = Window::new(if replace { "Replace" } else { "Find" });
        win.palette = crate::palette::WinPalette::Gray;
        win.modal = true;
        win.resizable = false;
        win.zoomable = false;
        win.closable = false;
        win.centred = true;
        let r = Rect::new(work.x + (work.w - w) / 2, work.y + (work.h - h) / 2, w, h);
        let root = self.root;
        let dialog = self.insert(root, r, Kind::Window(win));
        // Both labels the same length, so the two fields start in one column.
        let find = self.insert(dialog, Rect::new(2, 1, w - 6, 1), Kind::Input(InputLine::new("Text to find", &start)));
        self.set_dock(find, Dock::Manual);
        let with = if replace {
            let id = self.insert(dialog, Rect::new(2, 3, w - 6, 1), Kind::Input(InputLine::new("Replace with", "")));
            self.set_dock(id, Dock::Manual);
            Some(id)
        } else {
            None
        };
        let top = if replace { 5 } else { 3 };
        let mut c = Cluster::checks(&["~C~ase sensitive", "~W~hole words only"]);
        c.on[0] = case;
        c.on[1] = whole;
        let options = self.insert(dialog, Rect::new(2, top, 30, 2), Kind::Cluster(c));
        self.set_dock(options, Dock::Manual);
        let buttons = if replace {
            vec![
                Button::new("~R~eplace", CM_SEARCH_REPLACE).default(),
                Button::new("Replace ~a~ll", CM_SEARCH_ALL),
                Button::new("Cancel", CM_SEARCH_CANCEL).cancel(),
            ]
        } else {
            vec![
                Button::new("~F~ind", CM_SEARCH_GO).default(),
                Button::new("Cancel", CM_SEARCH_CANCEL).cancel(),
            ]
        };
        let row = self.insert(dialog, Rect::default(), Kind::Buttons(ButtonRow::new(buttons)));
        self.set_dock(row, Dock::BottomRight(w - 2, 2));
        self.focus_first();
        self.search = Some(Search { dialog, find, with, options, target: tid });
    }

    /// A button of the Find or Replace dialog.
    fn search_command(&mut self, cmd: Cmd) {
        let Some(s) = self.search.take() else { return };
        if cmd == CM_SEARCH_CANCEL || !self.is_alive(s.target) {
            self.close(s.dialog);
            return;
        }
        let input = |ui: &Ui, id: ViewId| match &ui.nodes[id.ix()].kind {
            Kind::Input(i) => i.text.clone(),
            _ => String::new(),
        };
        let text = input(self, s.find);
        let with = s.with.map(|id| crate::cell::glyphs(&input(self, id))).unwrap_or_default();
        let (case, whole) = match &self.nodes[s.options.ix()].kind {
            Kind::Cluster(c) => (c.on[0], c.on[1]),
            _ => (false, false),
        };
        let pattern = crate::cell::glyphs(&text);
        if pattern.is_empty() {
            // Nothing to look for yet: the dialog stays for it to be typed.
            self.search = Some(s);
            return;
        }
        self.last_search = Some(LastSearch { pattern: pattern.clone(), case_sensitive: case, whole_word: whole });
        let t = s.target;
        match cmd {
            CM_SEARCH_GO => {
                self.close(s.dialog);
                if !self.find_around(t) {
                    self.notice("Find", &format!("\"{text}\" is not in this text."));
                }
            }
            CM_SEARCH_REPLACE => {
                // The first press finds; each press after replaces what is
                // selected and finds the next, so every change is seen.
                let (_, found) = self.replace_text(t, &pattern, &with, case, whole);
                let found = found || self.find_around(t);
                self.follow_x(t);
                if found {
                    self.search = Some(s);
                } else {
                    self.close(s.dialog);
                    self.notice("Replace", &format!("There is no more \"{text}\" in this text."));
                }
            }
            CM_SEARCH_ALL => {
                self.close(s.dialog);
                let n = self.replace_all_text(t, &pattern, &with, case, whole);
                self.follow_x(t);
                let said = match n {
                    0 => format!("\"{text}\" is not in this text."),
                    1 => "1 replaced.".to_string(),
                    n => format!("{n} replaced."),
                };
                self.notice("Replace", &said);
            }
            _ => {}
        }
    }

    /// The last search again, from the caret on and round from the top;
    /// with none yet, the Find dialog.
    fn search_again(&mut self, tid: ViewId) {
        if self.last_search.is_none() {
            return self.open_search(tid, false);
        }
        if !self.find_around(tid) {
            let text = self.last_search.as_ref().map(|l| glyph_string(&l.pattern)).unwrap_or_default();
            self.notice("Find", &format!("\"{text}\" is not in this text."));
        }
    }

    /// The last search after the caret and, failing that, from the top: a
    /// person asking to find a word means anywhere in the text, not only
    /// below where they happen to be standing.
    fn find_around(&mut self, tid: ViewId) -> bool {
        let Some(l) = &self.last_search else { return false };
        let (pat, case, whole) = (l.pattern.clone(), l.case_sensitive, l.whole_word);
        let r = self.abs_rect(tid);
        let found = match &mut self.nodes[tid.ix()].kind {
            Kind::Text(t) => {
                t.width = r.w;
                if t.find(&pat, case, whole) {
                    true
                } else {
                    let (cur, anchor) = (t.cur, t.anchor);
                    t.cur = Point::new(0, 0);
                    t.anchor = None;
                    let again = t.find(&pat, case, whole);
                    if !again {
                        t.cur = cur;
                        t.anchor = anchor;
                    }
                    again
                }
            }
            _ => false,
        };
        self.follow_x(tid);
        found
    }

    /// A box with a sentence and OK, which the core closes itself.
    fn notice(&mut self, title: &str, text: &str) {
        if let Some(n) = self.notice.take() {
            self.close(n);
        }
        let row = ButtonRow::new(vec![crate::button::Button::new("~O~K", CM_NOTICE_OK).default()]);
        self.notice = Some(self.message_box(title, text, row));
    }

    /// Show the text as bytes, or the bytes as text again. The hex view
    /// takes the text's place in the window and the text waits, alive and
    /// untouched, until it comes back - so the program's handle to it goes
    /// on working the whole time.
    fn toggle_hex(&mut self, tid: ViewId) {
        let Some(win) = self.nodes[tid.ix()].parent else { return };
        let showing = match &mut self.nodes[tid.ix()].kind {
            Kind::Text(t) => t.hex.take(),
            _ => return,
        };
        if let Some(h) = showing {
            if let Kind::Hex(hv) = &mut self.nodes[h.ix()].kind {
                hv.source = None;
            }
            let at = self.nodes[win.ix()].children.iter().position(|k| *k == h);
            self.close(h);
            let kids = &mut self.nodes[win.ix()].children;
            kids.insert(at.unwrap_or(0).min(kids.len()), tid);
            return;
        }
        let bytes: Vec<u8> = match &self.nodes[tid.ix()].kind {
            Kind::Text(t) => {
                let mut b = Vec::new();
                for (i, l) in t.lines.iter().enumerate() {
                    if i > 0 {
                        b.push(b'\n');
                    }
                    b.extend(l.iter().map(|&g| u8::try_from(g as u32).unwrap_or(b'?')));
                }
                b
            }
            _ => return,
        };
        let mut hv = crate::hex::HexView::new(bytes);
        hv.source = Some(tid);
        let (rect, dock) = (self.nodes[tid.ix()].rect, self.nodes[tid.ix()].dock);
        let h = self.insert(win, rect, Kind::Hex(hv));
        self.set_dock(h, dock);
        let kids = &mut self.nodes[win.ix()].children;
        kids.retain(|k| *k != h);
        if let Some(pos) = kids.iter().position(|k| *k == tid) {
            kids[pos] = h;
        } else {
            kids.insert(0, h);
        }
        if let Kind::Text(t) = &mut self.nodes[tid.ix()].kind {
            t.hex = Some(h);
        }
    }

    /// What an editor offers and how it is set, all at once: `offers` is the
    /// bits of `edit::offer`, `state` those of `edit::state`. False for a
    /// view that is not a text.
    pub fn set_editor(&mut self, id: ViewId, offers: u8, state: u8) -> bool {
        use crate::edit::state::*;
        if offers & crate::edit::offer::SYNTAX != 0 {
            // The Syntax menu lists them; they are read now, once.
            self.load_syntaxes();
        }
        let r = self.abs_rect(id);
        let hex_now = match &mut self.nodes[id.ix()].kind {
            Kind::Text(t) => {
                t.offers = offers;
                if r.w > 0 {
                    t.width = r.w;
                }
                if t.wrap != (state & WRAP != 0) {
                    t.set_wrap(state & WRAP != 0, r.h.max(1));
                }
                t.keymap = (state & CLASSIC != 0).then_some(Keymap::Classic);
                t.hex.is_some()
            }
            _ => return false,
        };
        self.set_readonly(id, state & READONLY != 0);
        if hex_now != (state & HEX != 0) {
            self.toggle_hex(id);
        }
        true
    }

    /// An editor's offers, its state (`edit::state` bits) and where its
    /// caret is, line and column from 0. `None` for a view that is not a
    /// text.
    pub fn editor_state(&self, id: ViewId) -> Option<(u8, u8, i16, i16)> {
        use crate::edit::state::*;
        let Kind::Text(t) = &self.nodes[id.ix()].kind else { return None };
        let mut s = 0;
        if t.wrap {
            s |= WRAP;
        }
        if t.readonly {
            s |= READONLY;
        }
        if self.editor_keymap(t) == Keymap::Classic {
            s |= CLASSIC;
        }
        if t.hex.is_some() {
            s |= HEX;
        }
        if t.syntax.is_some() {
            s |= SYNTAX;
        }
        Some((t.offers, s, t.cur.y, t.cur.x))
    }

    // ------------------------------------------------------------ syntax

    fn load_syntaxes(&mut self) {
        if !self.syntaxes_loaded {
            self.syntaxes_loaded = true;
            let mut own = crate::syntax::parse(crate::syntax::BUILTIN);
            own.append(&mut self.syntaxes);
            self.syntaxes = own;
        }
    }

    /// Languages of the program's own, in the format of `syntax.ini`. One
    /// with the name of a language the core has takes its place. How many
    /// were read.
    pub fn add_syntax(&mut self, ini: &str) -> usize {
        self.load_syntaxes();
        let mut more = crate::syntax::parse(ini);
        let n = more.len();
        self.syntaxes.append(&mut more);
        n
    }

    /// The languages by name, each once, in the order they came: what
    /// Edit > Syntax lists.
    pub fn syntax_names(&mut self) -> Vec<String> {
        self.load_syntaxes();
        self.syntax_list().into_iter().map(|i| self.syntaxes[i].name.clone()).collect()
    }

    /// The index of each language that counts: the last one of each name.
    fn syntax_list(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for (i, s) in self.syntaxes.iter().enumerate() {
            match out.iter().position(|&o| self.syntaxes[o].name.eq_ignore_ascii_case(&s.name)) {
                Some(p) => out[p] = i,
                None => out.push(i),
            }
        }
        out
    }

    /// Colour a text as a language, named by its name (`Pascal`), an
    /// extension (`PAS`) or a file name (`DEMO.PAS`); an empty name, or one
    /// nobody answers to, turns the colours off. The language's name, or
    /// `None` when there is no such language.
    pub fn set_syntax(&mut self, id: ViewId, what: &str) -> Option<String> {
        self.load_syntaxes();
        let found = self.syntax_list().into_iter().rev().find(|&i| self.syntaxes[i].answers_to(what));
        let name = found.map(|i| self.syntaxes[i].name.clone());
        if let Kind::Text(t) = &mut self.nodes[id.ix()].kind {
            t.syntax = found.map(|i| i as u16);
            t.states_valid = 0;
        }
        name
    }

    /// The language a text is coloured as.
    pub fn syntax_of(&self, id: ViewId) -> Option<String> {
        match &self.nodes[id.ix()].kind {
            Kind::Text(t) => t.syntax.and_then(|i| self.syntaxes.get(i as usize)).map(|s| s.name.clone()),
            _ => None,
        }
    }

    /// A tree node somebody tried to open that has no children yet: its
    /// path, once. The program answers with `tree_set_children`.
    pub fn tree_take_expand(&mut self, id: ViewId) -> Option<Vec<usize>> {
        match &mut self.nodes[id.ix()].kind {
            Kind::Tree(t) => t.pending.take(),
            _ => None,
        }
    }

    pub fn tree_set_children(&mut self, id: ViewId, path: &[usize], children: Vec<crate::tree::TreeNode>) -> bool {
        match &mut self.nodes[id.ix()].kind {
            Kind::Tree(t) => t.set_children(path, children),
            _ => false,
        }
    }

    /// The texts from the root down to the current row of a tree.
    pub fn tree_path(&self, id: ViewId) -> Vec<String> {
        match &self.nodes[id.ix()].kind {
            Kind::Tree(t) => t.texts(&t.current_path()),
            _ => Vec::new(),
        }
    }

    /// The texts from the root down to a node.
    pub fn tree_texts(&self, id: ViewId, path: &[usize]) -> Vec<String> {
        match &self.nodes[id.ix()].kind {
            Kind::Tree(t) => t.texts(path),
            _ => Vec::new(),
        }
    }

    /// The history of an input line, newest first.
    pub fn history(&self, id: ViewId) -> Vec<String> {
        match &self.nodes[id.ix()].kind {
            Kind::Input(i) => i.history.clone(),
            _ => Vec::new(),
        }
    }

    pub fn set_history(&mut self, id: ViewId, items: Vec<String>) -> bool {
        match &mut self.nodes[id.ix()].kind {
            Kind::Input(i) => {
                i.history = items;
                true
            }
            _ => false,
        }
    }

    /// True when something has been chosen and is showing as chosen: a menu
    /// item clicked, a button pressed by a key. Draw one frame, wait a
    /// moment - long enough to be seen, about 90 ms - then call
    /// `complete_pick`. The core has no clock; the waiting is the backend's.
    ///
    /// A window falling into its bar, or rising out of it, is held the same
    /// way, a frame for every TICK: every client already waits and ticks
    /// for a chosen item, so every client animates without knowing it.
    pub fn pick_pending(&self) -> bool {
        if self.pending_pick.is_some() || self.anim.is_some() {
            return true;
        }
        let Some(w) = self.active_window() else { return false };
        self.button_rows(w).iter().any(|row| match &self.nodes[row.ix()].kind {
            Kind::Buttons(b) => b.pending.is_some(),
            _ => false,
        })
    }

    /// Deliver what `pick_pending` was holding.
    pub fn complete_pick(&mut self) {
        self.step_anim();
        if let Some((_, cmd)) = self.pending_pick.take() {
            self.close_menu();
            self.emit(cmd);
        }
        if let Some(w) = self.active_window() {
            for row in self.button_rows(w) {
                if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                    b.complete();
                }
            }
        }
    }

    fn find_kind(&self, f: impl Fn(&Kind) -> bool) -> Option<ViewId> {
        self.nodes[self.root.ix()]
            .children
            .iter()
            .copied()
            .find(|id| f(&self.nodes[id.ix()].kind))
    }

    fn status_id(&self) -> Option<ViewId> {
        self.find_kind(|k| matches!(k, Kind::Status(_)))
    }

    fn status_command(&self, k: Key) -> Option<u16> {
        let sid = self.status_id()?;
        match &self.nodes[sid.ix()].kind {
            Kind::Status(s) => s.command_for(k),
            _ => None,
        }
    }

    /// The control named by a label in this window whose hotkey is `c`.
    fn label_target(&self, win: ViewId, c: char) -> Option<ViewId> {
        let c = c.to_ascii_lowercase();
        self.nodes[win.ix()].children.iter().find_map(|k| match &self.nodes[k.ix()].kind {
            Kind::Label(l) if l.hotkey() == Some(c) => l.target,
            _ => None,
        })
    }

    /// Put the focus on one control of a window and take it off the rest.
    pub fn focus_on(&mut self, win: ViewId, target: ViewId) {
        if !self.is_alive(target) {
            return;
        }
        for c in self.focus_chain(win) {
            self.set_view_focus(c, c == target);
        }
    }

    /// Set or clear the tick on the menu item that sends `cmd`, wherever it
    /// is in the bar. For an option that a program turns on and off.
    ///
    /// On the application's menus and on every window's, because the bar
    /// is composed from those before each frame and a tick set on the
    /// composition alone would be gone by the next.
    pub fn set_menu_checked(&mut self, cmd: Cmd, on: bool) -> bool {
        let mut found = false;
        if let Some(bar) = self.menu_bar_id() {
            if let Kind::MenuBar(m) = &mut self.nodes[bar.ix()].kind {
                found |= crate::menu::MenuItem::set_checked(&mut m.base, cmd, on);
                crate::menu::MenuItem::set_checked(&mut m.items, cmd, on);
            }
        }
        for n in &mut self.nodes {
            if let Kind::Window(w) = &mut n.kind {
                found |= crate::menu::MenuItem::set_checked(&mut w.menu, cmd, on);
            }
        }
        found
    }

    /// What a window adds to the status line while it is active.
    pub fn set_window_status(&mut self, id: ViewId, items: Vec<crate::status::StatusItem>) -> bool {
        match &mut self.nodes[id.ix()].kind {
            Kind::Window(w) => {
                w.status = items;
                true
            }
            _ => false,
        }
    }

    /// What a window adds to the menu bar while it is active.
    pub fn set_window_menu(&mut self, id: ViewId, items: Vec<crate::menu::MenuItem>) -> bool {
        match &mut self.nodes[id.ix()].kind {
            Kind::Window(w) => {
                w.menu = items;
                true
            }
            _ => false,
        }
    }

    /// Put the active window's keys and menus onto the bars. Before every
    /// event, so a key is bound the moment its window comes to the front,
    /// and before every frame, so the picture agrees. Not while a panel
    /// is open: the panel was cut from the bar as it was, and its numbers
    /// have to keep meaning the same items until it closes.
    fn compose_bars(&mut self) {
        if self.menu_box_id().is_some() {
            return;
        }
        let active = self.active_window();
        let (mut status, mut menu) = match active.map(|a| &self.nodes[a.ix()].kind) {
            Some(Kind::Window(w)) => (w.status.clone(), clone_items(&w.menu)),
            _ => (Vec::new(), Vec::new()),
        };
        // The editor's own, after the window's: an Edit menu, its keys, and
        // where the caret is.
        let editor = active.and_then(|a| self.window_editor(a));
        let own_edit = menu.iter().any(|it| is_edit_menu(it));
        let mut right = String::new();
        let mut edit_added = false;
        if let Some(tid) = editor {
            if let Some(edit) = self.editor_menu(tid) {
                menu.push(edit);
                edit_added = true;
            }
            status.extend(self.editor_status(tid));
            right = self.editor_indicator(tid);
        } else if let Some(Kind::Window(w)) = active.map(|a| &self.nodes[a.ix()].kind) {
            // Not an editor: the window's own words, if it has any.
            right = w.indicator.clone();
        }
        if let Some(sid) = self.status_id() {
            if let Kind::Status(s) = &mut self.nodes[sid.ix()].kind {
                // A key the window carries takes the place of the program's
                // own on the same key while the window is in front: in a file
                // manager's window F3 is View, whatever F3 is elsewhere.
                s.items = s
                    .base
                    .iter()
                    .filter(|b| b.key.is_none() || !status.iter().any(|w| w.key == b.key))
                    .cloned()
                    .collect();
                s.items.extend(status);
                s.right = right;
            }
        }
        if let Some(bar) = self.menu_bar_id() {
            if let Kind::MenuBar(m) = &mut self.nodes[bar.ix()].kind {
                let bar_edit = m.base.iter().any(|it| is_edit_menu(it));
                m.items = merge_items(&m.base, &menu);
                // An Edit menu nobody else had goes where everybody looks
                // for it, after the first menu, not on the end of the bar.
                if edit_added && !bar_edit && !own_edit {
                    if let Some(pos) = m.items.iter().rposition(|it| is_edit_menu(it)) {
                        let it = m.items.remove(pos);
                        m.items.insert(1.min(m.items.len()), it);
                    }
                }
            }
        }
    }

    fn menu_bar_id(&self) -> Option<ViewId> {
        self.find_kind(|k| matches!(k, Kind::MenuBar(_)))
    }

    /// The open menu panel, if one is showing.
    ///
    /// Public because an application asking "is a key mine?" needs the same
    /// answer the toolkit's own handlers use, and working it out from the
    /// desktop's children — which is what the tests used to do — is both
    /// tedious and a guess.
    pub fn menu_open(&self) -> Option<ViewId> {
        self.menu_box_id()
    }

    /// The panel on top: the one keys go to. With a submenu open that is
    /// the submenu; its parent panel is still on the screen underneath.
    fn menu_box_id(&self) -> Option<ViewId> {
        self.menu_boxes().last().copied()
    }

    /// Every open panel, bottom to top.
    fn menu_boxes(&self) -> Vec<ViewId> {
        self.nodes[self.root.ix()]
            .children
            .iter()
            .copied()
            .filter(|id| matches!(self.nodes[id.ix()].kind, Kind::MenuBox(_)))
            .collect()
    }

    /// Close the topmost panel only. Left in a submenu, or Escape: the
    /// panel it came from is still there to go on choosing from.
    fn close_top_menu(&mut self) {
        if let Some(mb) = self.menu_box_id() {
            self.close(mb);
        }
        if self.menu_box_id().is_none() {
            if let Some(bar) = self.menu_bar_id() {
                if let Kind::MenuBar(m) = &mut self.nodes[bar.ix()].kind {
                    m.open = None;
                }
            }
        }
    }

    /// Open the submenu of an item, beside its panel, its first item on the
    /// row of the item it came from - which is where the classic DOS menus put it.
    fn open_submenu(&mut self, mb: ViewId, ix: usize) {
        let items = match &self.nodes[mb.ix()].kind {
            Kind::MenuBox(m) => match m.items.get(ix) {
                Some(it) if !it.items.is_empty() => clone_items(&it.items),
                _ => return,
            },
            _ => return,
        };
        // Whatever was open above this panel goes first.
        while self.menu_box_id().is_some_and(|top| top != mb) {
            self.close_top_menu();
        }
        let mut b = MenuBox::new(items);
        b.parent = None;
        let (w, h) = (b.width(), b.height());
        let pr = self.abs_rect(mb);
        let screen = self.nodes[self.root.ix()].rect;
        let x = (pr.right() - 1).min((screen.w - w).max(0));
        let y = (pr.y + ix as i16).min((screen.h - h).max(0));
        let root = self.root;
        self.insert(root, Rect::new(x, y, w, h), Kind::MenuBox(b));
    }

    /// Drop a panel out of the bar. Closing whatever was open first means the
    /// tree never holds two, so "is a menu open" is one lookup and not a state
    /// machine.
    pub fn open_menu(&mut self, bar_ix: usize) {
        self.close_menu();
        let Some(bar) = self.menu_bar_id() else { return };
        let (x, items) = {
            let Kind::MenuBar(m) = &self.nodes[bar.ix()].kind else {
                return;
            };
            if bar_ix >= m.items.len() || m.items[bar_ix].items.is_empty() {
                return;
            }
            (m.item_x(bar_ix), clone_items(&m.items[bar_ix].items))
        };
        if let Kind::MenuBar(m) = &mut self.nodes[bar.ix()].kind {
            m.open = Some(bar_ix);
        }
        let mut b = MenuBox::new(items);
        b.parent = Some(bar_ix);
        let (w, h) = (b.width(), b.height());
        let bar_y = self.nodes[bar.ix()].rect.y;
        // One column left of the label, so the frame lines up under it.
        let r = Rect::new((x - 2).max(0), bar_y + 1, w, h);
        let root = self.root;
        self.insert(root, r, Kind::MenuBox(b));
    }

    pub fn close_menu(&mut self) {
        for mb in self.menu_boxes() {
            self.close(mb);
        }
        if let Some(bar) = self.menu_bar_id() {
            if let Kind::MenuBar(m) = &mut self.nodes[bar.ix()].kind {
                m.open = None;
            }
        }
    }

    fn bar_len(&self) -> usize {
        match self.menu_bar_id() {
            Some(b) => match &self.nodes[b.ix()].kind {
                Kind::MenuBar(m) => m.items.len(),
                _ => 0,
            },
            None => 0,
        }
    }

    /// Choose the item the panel is standing on, with the pause that tells the
    /// person they hit it. Only for the mouse: an item reached with the arrow
    /// keys has been lit up since they arrived on it, so there is nothing left
    /// to show them and the delay would be pure latency.
    fn pick_menu_slowly(&mut self, mb: ViewId) {
        let Kind::MenuBox(m) = &self.nodes[mb.ix()].kind else {
            return;
        };
        let Some(it) = m.items.get(m.current) else {
            return;
        };
        if !it.selectable() {
            return;
        }
        // A submenu opens at once: there is nothing to show chosen, only
        // more to choose from.
        if !it.items.is_empty() {
            let ix = m.current;
            self.open_submenu(mb, ix);
            return;
        }
        if it.cmd == 0 {
            return;
        }
        self.pending_pick = Some((mb, it.cmd));
    }

    /// Choose the item the panel is standing on.
    fn pick_menu(&mut self, mb: ViewId) {
        let Kind::MenuBox(m) = &self.nodes[mb.ix()].kind else {
            return;
        };
        let Some(it) = m.items.get(m.current) else {
            return;
        };
        if !it.selectable() {
            return;
        }
        if !it.items.is_empty() {
            let ix = m.current;
            self.open_submenu(mb, ix);
            return;
        }
        if it.cmd == 0 {
            return;
        }
        let cmd = it.cmd;
        self.close_menu();
        self.emit(cmd);
    }

    /// A command from the menu or the status line. An editor's own is done
    /// here and now; any other waits for the program to collect it.
    fn emit(&mut self, cmd: Cmd) {
        if (CM_CTX_UNDO..=CM_CTX_SELECT_ALL).contains(&cmd) {
            self.context_command(cmd);
        } else if is_editor_cmd(cmd) {
            self.editor_command(cmd);
        } else {
            self.command = Some(cmd);
        }
    }

    /// Close a view and everything inside it. A handle to a child of a
    /// closed window must say it is dead, or `is_alive` is not worth asking.
    pub fn close(&mut self, id: ViewId) {
        // A text and the hex view standing in for it go together: the text
        // is out of the window while the bytes are shown, so closing the
        // window would otherwise leave it behind, alive and unreachable.
        let partner = match &mut self.nodes[id.ix()].kind {
            Kind::Hex(h) => h.source.take(),
            Kind::Text(t) => t.hex.take(),
            _ => None,
        };
        let kids = core::mem::take(&mut self.nodes[id.ix()].children);
        for k in kids {
            self.close(k);
        }
        self.nodes[id.ix()].alive = false;
        if let Some(parent) = self.nodes[id.ix()].parent {
            self.nodes[parent.ix()].children.retain(|k| *k != id);
        }
        // Its bar goes with it, and the bars above come down a row.
        self.minimized.retain(|m| *m != id);
        if self.anim.as_ref().is_some_and(|a| a.id == id) {
            self.anim = None;
        }
        if let Some(p) = partner.filter(|p| self.nodes[p.ix()].alive) {
            match &mut self.nodes[p.ix()].kind {
                Kind::Hex(h) => h.source = None,
                Kind::Text(t) => t.hex = None,
                _ => {}
            }
            self.close(p);
        }
    }

    /// Send the frontmost window to the back — the classic Alt+F6 / F6.
    pub fn cycle_windows(&mut self) {
        if self.modal().is_some() {
            return;
        }
        // Round until a window that is showing is in front: a minimized one
        // there would be a turn of F6 that did nothing.
        let n = self.nodes[self.root.ix()].children.len();
        for _ in 0..n {
            let kids = &mut self.nodes[self.root.ix()].children;
            if kids.len() > 1 {
                let v = kids.pop().unwrap();
                kids.insert(0, v);
            }
            let last = self.nodes[self.root.ix()].children.last().copied();
            if !last.is_some_and(|l| self.is_minimized(l)) {
                break;
            }
        }
    }

    /// The other way round: the window at the back comes to the front.
    /// The classic Shift+F6.
    pub fn cycle_windows_back(&mut self) {
        if self.modal().is_some() {
            return;
        }
        let root = self.root.ix();
        let first = self.nodes[root]
            .children
            .iter()
            .position(|k| matches!(&self.nodes[k.ix()].kind, Kind::Window(w) if !w.minimized));
        if let Some(pos) = first {
            let kids = &mut self.nodes[root].children;
            if kids.len() > 1 {
                let v = kids.remove(pos);
                kids.push(v);
            }
        }
    }

    /// The window numbered `n` comes to the front, if there is one.
    pub fn activate_numbered(&mut self, n: u8) -> bool {
        if self.modal().is_some() {
            return false;
        }
        let hit = self.nodes[self.root.ix()]
            .children
            .iter()
            .copied()
            .find(|c| matches!(&self.nodes[c.ix()].kind, Kind::Window(w) if w.number == Some(n)));
        match hit {
            Some(id) => {
                self.activate(id);
                true
            }
            None => false,
        }
    }

    /// Start moving or resizing the active window from the keyboard:
    /// the classic Ctrl+F5. Arrows move, Shift+arrows resize, Enter
    /// keeps it, Escape puts it back. The frame shows the dragging colour
    /// meanwhile, as it does under the mouse.
    pub fn begin_size_move(&mut self) -> bool {
        if self.modal().is_some() {
            return false;
        }
        let Some(id) = self.active_window() else { return false };
        let ok = matches!(&self.nodes[id.ix()].kind, Kind::Window(w) if w.movable || w.resizable);
        if !ok {
            return false;
        }
        self.sizing = Some(Sizing { id, orig: self.nodes[id.ix()].rect });
        true
    }

    /// Whether a window is being moved or resized from the keyboard.
    pub fn sizing(&self) -> Option<ViewId> {
        self.sizing.as_ref().map(|s| s.id)
    }

    fn sizing_key(&mut self, k: Key) {
        use crate::event::KeyCode as K;
        let Some(s) = &self.sizing else { return };
        let (id, orig) = (s.id, s.orig);
        let (movable, resizable) = match &self.nodes[id.ix()].kind {
            Kind::Window(w) => (w.movable, w.resizable),
            _ => (false, false),
        };
        let (dx, dy) = match k.code {
            K::Left => (-1, 0),
            K::Right => (1, 0),
            K::Up => (0, -1),
            K::Down => (0, 1),
            K::Enter => {
                self.sizing = None;
                return;
            }
            K::Esc => {
                self.nodes[id.ix()].rect = orig;
                self.sizing = None;
                return;
            }
            _ => return,
        };
        let r = self.nodes[id.ix()].rect;
        let area = self.work_area();
        let wanted = if k.mods.shift {
            if !resizable {
                return;
            }
            Rect::new(r.x, r.y, r.w + dx, r.h + dy)
        } else {
            if !movable {
                return;
            }
            Rect::new(r.x + dx, r.y + dy, r.w, r.h)
        };
        // The same limits the mouse has: a window may not be smaller than
        // its own floor nor leave the screen, and a resize is a resize
        // and not a move.
        self.arrange_free(id, wanted, area, k.mods.shift);
    }

    /// A rect for a window, kept to what it allows, without the tiling
    /// rule that pulls it fully inside the work area: a window dragged by
    /// hand may hang off the edge, as under the mouse.
    fn arrange_free(&mut self, id: ViewId, r: Rect, area: Rect, resizing: bool) {
        let screen = self.nodes[self.root.ix()].rect;
        let Kind::Window(w) = &mut self.nodes[id.ix()].kind else { return };
        let mut r = r;
        if resizing {
            let (min_w, min_h) = (w.min_w.max(6), w.min_h.max(3));
            let max_w = if w.max_w > 0 { w.max_w } else { i16::MAX };
            let max_h = if w.max_h > 0 { w.max_h } else { i16::MAX };
            r.w = r.w.clamp(min_w, max_w.max(min_w));
            r.h = r.h.clamp(min_h, max_h.max(min_h));
        } else {
            r.x = r.x.clamp(1 - r.w, screen.w - 1);
            r.y = r.y.clamp(area.y, screen.h - 1);
        }
        w.centred = false;
        w.unzoomed = None;
        self.nodes[id.ix()].rect = r;
    }

    /// The list of windows, as a modal dialog: the classic Alt+0. A
    /// row per window, front to back, its number first when it has one;
    /// Enter or OK brings the chosen one to the front.
    pub fn window_list(&mut self) {
        if self.modal().is_some() {
            return;
        }
        let windows: Vec<ViewId> = self.nodes[self.root.ix()]
            .children
            .iter()
            .rev()
            .copied()
            .filter(|c| matches!(&self.nodes[c.ix()].kind, Kind::Window(_)))
            .collect();
        if windows.is_empty() {
            return;
        }
        let names: Vec<String> = windows
            .iter()
            .map(|id| match &self.nodes[id.ix()].kind {
                Kind::Window(w) => match w.number {
                    Some(n) => format!("{n}  {}", w.title),
                    None => format!("   {}", w.title),
                },
                _ => String::new(),
            })
            .collect();
        let work = self.work_area();
        let rows = (names.len() as i16).clamp(3, (work.h - 8).max(3));
        let w = 44.min(work.w - 4).max(24);
        let h = rows + 6;
        let mut win = Window::new("Windows");
        win.palette = crate::palette::WinPalette::Gray;
        win.modal = true;
        win.resizable = false;
        win.zoomable = false;
        win.closable = false;
        let r = Rect::new(work.x + (work.w - w) / 2, work.y + (work.h - h) / 2, w, h);
        let root = self.root;
        let dialog = self.insert(root, r, Kind::Window(win));
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let list = self.insert(dialog, Rect::new(2, 1, w - 6, rows), Kind::List(crate::controls::ListBox::new(&refs)));
        self.set_dock(list, Dock::Manual);
        let row = self.insert(
            dialog,
            Rect::default(),
            Kind::Buttons(ButtonRow::new(vec![
                crate::button::Button::new("~O~K", CM_WINLIST_OK).default(),
                crate::button::Button::new("~C~ancel", CM_WINLIST_CANCEL).cancel(),
            ])),
        );
        self.set_dock(row, Dock::BottomRight(w - 2, 2));
        self.focus_first();
        self.winlist = Some(WindowList { dialog, list, windows });
    }

    /// A command from a dialog the toolkit built for itself. True when it
    /// was one and has been dealt with.
    fn internal_command(&mut self, cmd: Cmd) -> bool {
        if cmd < CM_INTERNAL {
            return false;
        }
        if (CM_SEARCH_GO..=CM_SEARCH_CANCEL).contains(&cmd) {
            self.search_command(cmd);
            return true;
        }
        if cmd == CM_NOTICE_OK {
            if let Some(n) = self.notice.take() {
                self.close(n);
            }
            return true;
        }
        if is_editor_cmd(cmd) {
            self.editor_command(cmd);
            return true;
        }
        if let Some(wl) = self.winlist.take() {
            let chosen = match &self.nodes[wl.list.ix()].kind {
                Kind::List(l) => wl.windows.get(l.current).copied(),
                _ => None,
            };
            self.close(wl.dialog);
            if cmd == CM_WINLIST_OK {
                if let Some(id) = chosen {
                    if self.is_alive(id) {
                        self.activate(id);
                    }
                }
            }
        }
        true
    }

    /// The windows the desktop may arrange: on the desktop, not modal,
    /// and allowed to move. Back to front, which is the order they are
    /// kept in.
    fn arrangeable(&self) -> Vec<ViewId> {
        self.nodes[self.root.ix()]
            .children
            .iter()
            .copied()
            .filter(|c| matches!(&self.nodes[c.ix()].kind, Kind::Window(w) if w.movable && !w.modal))
            .collect()
    }

    /// Give a window a new place and size, within what it allows: a fixed
    /// window keeps its size and only moves; a resizable one is clamped to
    /// its own limits. Either forgets that it was centred or zoomed - it
    /// has been put somewhere on purpose now.
    fn arrange(&mut self, id: ViewId, r: Rect, area: Rect) {
        let own = self.nodes[id.ix()].rect;
        let Kind::Window(w) = &mut self.nodes[id.ix()].kind else {
            return;
        };
        let mut r = r;
        if !w.resizable {
            r.w = own.w;
            r.h = own.h;
        } else {
            let (min_w, min_h) = (w.min_w.max(6), w.min_h.max(3));
            let max_w = if w.max_w > 0 { w.max_w } else { i16::MAX };
            let max_h = if w.max_h > 0 { w.max_h } else { i16::MAX };
            r.w = r.w.clamp(min_w, max_w.max(min_w));
            r.h = r.h.clamp(min_h, max_h.max(min_h));
        }
        // Inside the work area if it can be; a window bigger than the
        // desktop keeps its top-left corner in sight.
        r.x = r.x.min(area.x + area.w - r.w).max(area.x);
        r.y = r.y.min(area.y + area.h - r.h).max(area.y);
        w.centred = false;
        w.unzoomed = None;
        self.nodes[id.ix()].rect = r;
    }

    /// Lay the windows along the diagonal, the back one at the top-left of
    /// the work area and each one in front a cell down and to the right,
    /// every title bar showing, as the classic desktops cascaded.
    ///
    /// The desktop's job and not the application's: which windows there
    /// are, which may move and which are modal is the desktop's knowledge,
    /// and an application that had to ask for all of it to draw one
    /// staircase would be doing the desktop's work with worse tools.
    pub fn cascade(&mut self) {
        if self.modal().is_some() {
            return;
        }
        let area = self.work_area();
        for (i, id) in self.arrangeable().into_iter().enumerate() {
            let off = (i as i16).min(area.w - 6).min(area.h - 3).max(0);
            let r = Rect::new(area.x + off, area.y + off, area.w - off, area.h - off);
            self.arrange(id, r, area);
        }
    }

    /// Divide the work area between the windows so that none overlaps:
    /// the square root of their number in columns, the rest in rows, the
    /// columns on the right one window taller when it does not come out
    /// even - the road the classic desktops took too.
    ///
    /// A window that cannot be resized gets a cell like the others and
    /// stands in its top-left corner at its own size. The first cut left
    /// such windows where they were, and on a desktop of dialogs - a
    /// calculator, a calendar - Tile then did nothing at all, which is
    /// worse than a dialog that overhangs its cell.
    pub fn tile(&mut self) {
        if self.modal().is_some() {
            return;
        }
        let area = self.work_area();
        let wins: Vec<ViewId> = self.arrangeable();
        let n = wins.len() as i16;
        if n == 0 || area.w < 6 || area.h < 3 {
            return;
        }
        let cols = (1..=n).rev().find(|c| c * c <= n).unwrap_or(1);
        let rows = n / cols;
        let extra = n % cols;
        let mut next = wins.into_iter();
        for c in 0..cols {
            let x0 = area.x + (area.w as i32 * c as i32 / cols as i32) as i16;
            let x1 = area.x + (area.w as i32 * (c + 1) as i32 / cols as i32) as i16;
            let in_col = rows + if c >= cols - extra { 1 } else { 0 };
            for r in 0..in_col {
                let y0 = area.y + (area.h as i32 * r as i32 / in_col as i32) as i16;
                let y1 = area.y + (area.h as i32 * (r + 1) as i32 / in_col as i32) as i16;
                if let Some(id) = next.next() {
                    self.arrange(id, Rect::new(x0, y0, x1 - x0, y1 - y0), area);
                }
            }
        }
    }

    pub fn toggle_zoom(&mut self, id: ViewId) {
        // Zoomed means the work area, not the screen: a window covering the
        // menu bar is a window you cannot get out from under.
        let full = self.work_area();
        let Kind::Window(w) = &mut self.nodes[id.ix()].kind else {
            return;
        };
        if !w.zoomable {
            return;
        }
        match w.unzoomed.take() {
            Some(prev) => self.nodes[id.ix()].rect = prev,
            None => {
                let prev = self.nodes[id.ix()].rect;
                if let Kind::Window(w) = &mut self.nodes[id.ix()].kind {
                    w.unzoomed = Some(prev);
                }
                self.nodes[id.ix()].rect = full;
            }
        }
    }

    // ----------------------------------------------------------------- drawing

    /// Where the caret is on screen, if it should be seen at all.
    ///
    /// The core does not draw it. The classic DOS toolkits did not either — they moved the
    /// video card's own cursor, and a terminal has the same thing. Left to the
    /// hardware it blinks by itself and costs nothing per frame; drawn into
    /// the grid it would have to be erased and repainted on every keystroke,
    /// and it would fight the selection for the same cell.
    ///
    /// `None` means hide it: no editable view has focus, or the caret has been
    /// scrolled out of its own window, or something is drawn over it - an
    /// open menu, a right-click menu, a window going away or coming back.
    /// The card's cursor is above every cell, so a caret under a menu
    /// blinked through it.
    pub fn cursor(&self) -> Option<Point> {
        let win = self.active_window()?;
        let p = self.caret_point(win)?;
        if self.anim.is_some() {
            return None;
        }
        let mut order: Vec<ViewId> = self.nodes[self.root.ix()].children.clone();
        order.sort_by_key(|c| self.nodes[c.ix()].kind.layer());
        let at = order.iter().position(|&c| c == win)?;
        let covered = order[at + 1..].iter().any(|&c| {
            let mut r = self.abs_rect(c);
            match &self.nodes[c.ix()].kind {
                Kind::Window(_) if self.is_minimized(c) => return false,
                Kind::Window(w) if w.shadow => {
                    r.w += 2;
                    r.h += 1;
                }
                Kind::MenuBox(_) => {
                    r.w += 2;
                    r.h += 1;
                }
                _ => {}
            }
            r.contains(p)
        });
        (!covered).then_some(p)
    }

    fn caret_point(&self, win: ViewId) -> Option<Point> {

        // A file panel's path field has a caret of its own while it is being
        // typed into. Without it, editing a path is typing into a box with no
        // sign of where the next letter will land.
        if let Some(fid) = self.scrolling_child(win) {
            if let Kind::Files(f) = &self.nodes[fid.ix()].kind {
                // The same rectangle the field is drawn in, and not the
                // panel's: the panel is two columns wider, so working the
                // caret out from it put it a column off and scrolled the text
                // by a different amount than the drawing did.
                let r = field_rect(self.abs_rect(fid));
                return f
                    .path
                    .caret_x(r.w)
                    .map(|x| Point::new(r.x + x, r.y))
                    .filter(|p| r.contains(*p));
            }
        }

        // Whatever holds the focus, not whatever happens to be first. In a
        // dialog the editable thing is rarely the first child, and a caret
        // that only ever appears for the first one appears in the wrong place
        // or not at all.
        let tid = self.focused().or_else(|| self.text_child(win))?;
        let abs = self.abs_rect(tid);
        match &self.nodes[tid.ix()].kind {
            Kind::Text(t) if !t.readonly && t.wrapping() => t
                .screen_of(t.cur, abs.h)
                .map(|(c, r)| Point::new(abs.x + c, abs.y + r))
                .filter(|p| abs.contains(*p)),
            Kind::Text(t) if !t.readonly => {
                let p = Point::new(abs.x + t.cur.x - t.left, abs.y + t.cur.y - t.top);
                abs.contains(p).then_some(p)
            }
            Kind::Input(i) => i
                .caret_x(abs.w)
                .map(|x| Point::new(abs.x + x, abs.y))
                .filter(|p| abs.contains(*p)),
            _ => None,
        }
    }

    pub fn draw(&mut self, buf: &mut Buffer) {
        self.compose_bars();
        self.measure();
        let clip = buf.rect();
        let blue = self.palette.blue;
        self.draw_node(self.root, buf, clip, &blue);
        self.draw_anim(buf, clip);
    }

    /// The measure pass.
    ///
    /// Two rules so far, and in this order: a window's content fills its
    /// client area, and a help page is then broken into lines for the width it
    /// ended up with. The order is not incidental — laying the text out
    /// against yesterday's width and then resizing the view leaves a page
    /// wrapped for a window that no longer exists.
    ///
    /// This used to be done by the demonstration program, by hand, for the one
    /// kind of view it knew about. Resizing a window that held anything else
    /// left the content its old size with the window's background showing
    /// round it. Layout is the toolkit's job; an application that has to
    /// remember to do it will forget.
    fn measure(&mut self) {
        let screen_w = self.nodes[self.root.ix()].rect.w;

        // Windows stay inside the work area. Doing it here rather than in the
        // drag means it also holds for a window that was created too high, for
        // one pushed up by a resize, and for one that was fine until a menu
        // bar appeared above it.
        let work = self.work_area();
        let kids: Vec<ViewId> = self.nodes[self.root.ix()].children.clone();
        for id in kids {
            if !matches!(self.nodes[id.ix()].kind, Kind::Window(_)) {
                continue;
            }
            let mut r = self.nodes[id.ix()].rect;
            if matches!(&self.nodes[id.ix()].kind, Kind::Window(w) if w.centred) {
                r.x = work.x + (work.w - r.w) / 2;
                r.y = work.y + (work.h - r.h) / 2;
            }
            r.y = r.y.clamp(work.y, (work.bottom() - 1).max(work.y));
            r.x = r.x.clamp(1 - r.w, (work.right() - 1).max(0));
            let before = self.nodes[id.ix()].rect;
            self.nodes[id.ix()].rect = r;
            // Nudged by this pass, not moved by a hand: the window is still
            // where the last resize put it, as far as resizing goes.
            if let Kind::Window(w) = &mut self.nodes[id.ix()].kind {
                if let Some((_, _, fitted)) = &mut w.home {
                    if *fitted == before {
                        *fitted = r;
                    }
                }
            }
        }
        for i in 0..self.nodes.len() {
            if !matches!(self.nodes[i].kind, Kind::Window(_)) {
                continue;
            }
            let r = self.nodes[i].rect;
            let mut free = Rect::new(0, 0, (r.w - 2).max(0), (r.h - 2).max(0));
            let kids: Vec<ViewId> = self.nodes[i].children.clone();

            // Edges first, then what is left goes to the one that fills. The
            // other order hands the filling view the whole client area and
            // then draws the strips on top of it.
            for k in &kids {
                match self.nodes[k.ix()].dock {
                    Dock::Top(h) => {
                        let h = h.min(free.h);
                        self.nodes[k.ix()].rect = Rect::new(free.x, free.y, free.w, h);
                        free.y += h;
                        free.h -= h;
                    }
                    Dock::Bottom(h) => {
                        let h = h.min(free.h);
                        self.nodes[k.ix()].rect = Rect::new(free.x, free.bottom() - h, free.w, h);
                        free.h -= h;
                    }
                    Dock::Fill | Dock::FillFrom(_) | Dock::BottomRight(..) | Dock::Manual => {}
                }
            }
            for k in &kids {
                if let Dock::BottomRight(w, h) = self.nodes[k.ix()].dock {
                    let w = w.min(free.w);
                    let h = h.min(free.h);
                    self.nodes[k.ix()].rect =
                        Rect::new(free.right() - w, free.bottom() - h, w, h);
                }
            }
            for k in &kids {
                match self.nodes[k.ix()].dock {
                    Dock::Fill => self.nodes[k.ix()].rect = free,
                    Dock::FillFrom(top) => {
                        let top = top.clamp(0, free.h);
                        self.nodes[k.ix()].rect = Rect::new(free.x, free.y + top, free.w, free.h - top);
                    }
                    _ => {}
                }
            }
        }

        for i in 0..self.nodes.len() {
            let id = ViewId(i as u32);
            let r = self.abs_rect(id);
            let taken = self.foot_taken(id);
            match &mut self.nodes[i].kind {
                Kind::Html(h) => h.layout(r.w),
                Kind::Console(c) => c.layout(r.w, r.h),
                Kind::Files(f) => {
                    f.set_foot_taken(taken);
                    f.layout(r, screen_w);
                }
                Kind::Hex(h) => h.layout(r),
                Kind::Text(t) => {
                    t.width = r.w;
                    // The lines the window shows need their starting
                    // colouring state; lines below them are left for later.
                    if let Some(s) = t.syntax.and_then(|s| self.syntaxes.get(s as usize)) {
                        let upto = t.top.max(0) as usize + r.h.max(0) as usize + 1;
                        crate::syntax::update_states(t, s, upto);
                    }
                }
                Kind::List(l) => l.set_rows(r.h),
                Kind::Tree(t) => t.set_rows(r.h),
                _ => {}
            }
        }
    }

    /// `wc` is the colour family of the window this view is inside. It is
    /// carried down rather than looked up, because a view has no idea which
    /// window it is in and should not acquire one: the same text view is blue
    /// in a document and cyan in a help topic, and that is a property of where
    /// it was put, not of what it is.
    fn draw_node(&self, id: ViewId, buf: &mut Buffer, parent_clip: Rect, wc: &WinColors) {
        // Put away: its bar is drawn on the desktop instead, shadow and all
        // of the window left out.
        if self.is_minimized(id) {
            return;
        }
        let abs = self.abs_rect(id);

        // The shadow falls outside the view, so it is drawn against the
        // parent's clip and before the view itself — under this window, over
        // whatever is behind it.
        if let Kind::Window(w) = &self.nodes[id.ix()].kind {
            if w.shadow {
                self.draw_shadow(abs, buf, parent_clip);
            }
        }

        let clip = parent_clip.intersect(&abs);
        if clip.is_empty() {
            return;
        }

        match &self.nodes[id.ix()].kind {
            Kind::Desktop(d) => {
                buf.fill(abs, d.glyph, self.palette.desktop, clip);
                // The bars of the windows put away lie on the desktop,
                // under every window: a window moved over the corner
                // covers them, as it would anything else on the desk.
                self.draw_bars(buf, clip);
            }
            Kind::Window(w) => self.draw_window(id, w, abs, buf, clip),
            Kind::Text(t) => self.draw_text(t, abs, buf, clip, wc),
            Kind::Html(h) => self.draw_html(h, abs, buf, clip, wc),
            Kind::Console(c) => draw_console(c, abs, buf, clip),
            Kind::Files(f) => {
                let active = self.parent_active(id);
                self.draw_files(f, abs, buf, clip, wc, self.foot_taken(id), active)
            }
            Kind::Input(i) => draw_input(i, abs, buf, clip, &self.palette),
            Kind::Buttons(b) => self.draw_buttons(b, abs, buf, clip, wc),
            Kind::Hex(h) => self.draw_hex(h, abs, buf, clip),
            Kind::Cluster(c) => self.draw_cluster(c, abs, buf, clip),
            Kind::List(l) => self.draw_list(l, abs, buf, clip),
            Kind::Static(t) => self.draw_static(t, abs, buf, clip, wc),
            Kind::Tree(t) => self.draw_tree(t, abs, buf, clip),
            Kind::Status(s) => self.draw_status(s, abs, buf, clip),
            Kind::Label(l) => self.draw_label(l, abs, buf, clip),
            Kind::Progress(pr) => self.draw_progress(pr, abs, buf, clip, wc),
            Kind::Canvas(c) => {
                // As it is: the cells are the picture. A clear cell is not
                // drawn at all, so the window shows through it - which is
                // what "outside the board" looks like.
                for y in 0..abs.h.min(c.h) {
                    for x in 0..abs.w.min(c.w) {
                        if let Some(cell) = c.get(x, y) {
                            if cell != crate::controls::Canvas::CLEAR {
                                buf.put(abs.x + x, abs.y + y, cell.ch, cell.attr, clip);
                            }
                        }
                    }
                }
            }
            Kind::MenuBar(m) => self.draw_menu_bar(m, abs, buf, clip),
            Kind::MenuBox(m) => self.draw_menu_box(m, abs, buf, clip, parent_clip),
        }

        let child_clip = clip.intersect(&self.client_abs(id));
        if child_clip.is_empty() {
            return;
        }
        let wc = match &self.nodes[id.ix()].kind {
            Kind::Window(w) => *self.palette.window(w.palette),
            _ => *wc,
        };
        let mut kids: Vec<ViewId> = self.nodes[id.ix()].children.clone();
        kids.sort_by_key(|c| self.nodes[c.ix()].kind.layer());
        for c in kids {
            self.draw_node(c, buf, child_clip, &wc);
        }
    }

    /// Darken what lies under the window, keeping the glyphs. A shadow over
    /// text still shows the text, which is why this recolours rather than
    /// fills.
    fn draw_shadow(&self, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let a = self.palette.shadow;
        let right = Rect::new(abs.right(), abs.y + 1, 2, abs.h - 1);
        let below = Rect::new(abs.x + 2, abs.bottom(), abs.w, 1);
        for r in [right, below] {
            let r = r.intersect(&clip);
            for y in r.y..r.bottom() {
                for x in r.x..r.right() {
                    buf.recolor(x, y, a, clip);
                }
            }
        }
    }

    fn draw_window(&self, id: ViewId, w: &Window, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let active = self.active_window() == Some(id);
        let dragging = self.drag.as_ref().map(|d| d.id) == Some(id) || self.sizing.as_ref().map(|s| s.id) == Some(id);
        let p = self.palette.window(w.palette);
        let fa = if dragging {
            p.frame_dragging
        } else if active {
            p.frame_active
        } else {
            p.frame_passive
        };

        // Active windows wear a double frame, inactive a single one. The
        // classic trick: you can tell which window has focus with the colours
        // turned off. A window being dragged drops to a single frame too —
        // it is in flight, not settled.
        let (h, v, tl, tr, bl, br) = if active && !dragging {
            (
                glyph::DL_H,
                glyph::DL_V,
                glyph::DL_TL,
                glyph::DL_TR,
                glyph::DL_BL,
                glyph::DL_BR,
            )
        } else {
            (
                glyph::SL_H,
                glyph::SL_V,
                glyph::SL_TL,
                glyph::SL_TR,
                glyph::SL_BL,
                glyph::SL_BR,
            )
        };

        if abs.w < 2 || abs.h < 2 {
            return;
        }

        // Body, then the four edges over it.
        let body = if active { p.body } else { p.body_passive };
        buf.fill(abs.inset(1), b' ', body, clip);
        buf.hline(abs.x + 1, abs.y, abs.w - 2, h, fa, clip);
        buf.hline(abs.x + 1, abs.bottom() - 1, abs.w - 2, h, fa, clip);
        buf.vline(abs.x, abs.y + 1, abs.h - 2, v, fa, clip);
        buf.vline(abs.right() - 1, abs.y + 1, abs.h - 2, v, fa, clip);
        buf.put(abs.x, abs.y, tl, fa, clip);
        buf.put(abs.right() - 1, abs.y, tr, fa, clip);
        buf.put(abs.x, abs.bottom() - 1, bl, fa, clip);

        // The resize grip. Two cells, not one: a short single line running into
        // a single-line corner, in green. One cell is a corner; two read as a
        // handle, and you can actually hit it with a mouse.
        if w.resizable {
            buf.put(abs.right() - 2, abs.bottom() - 1, glyph::SL_H, p.handle, clip);
            buf.put(abs.right() - 1, abs.bottom() - 1, glyph::SL_BR, p.handle, clip);
        } else {
            buf.put(abs.right() - 1, abs.bottom() - 1, br, fa, clip);
        }

        // The boxes appear on the active window only. An inactive window
        // shows its title and its number and nothing you could click, which
        // is honest: clicking it would only activate it anyway.
        //
        // They sit together at the right: minimize, zoom, close, the close
        // box last, in the corner - where every windowing system since has
        // put them and where the hand goes looking. The classic DOS desktops
        // had the close box alone at the left; that was the one thing about
        // them a person who grew up on anything later could not find.
        let boxes = title_boxes(w, abs);
        if active {
            for &(b, x) in &boxes {
                let icon: Glyph = match b {
                    TitleBox::Minimize => 0x19,                              // ↓, down into the corner
                    TitleBox::Zoom if w.is_zoomed() => 0x12,                 // ↕, back to its size
                    TitleBox::Zoom => 0x18,                                  // ↑, the whole desktop
                    TitleBox::Close => glyph::SQUARE as Glyph,
                };
                buf.put(x, abs.y, b'[', fa, clip);
                buf.put(x + 1, abs.y, icon, p.handle, clip);
                buf.put(x + 2, abs.y, b']', fa, clip);
            }
        }
        // Left of the boxes: the window's number, and the title in the
        // middle of what is left.
        let boxes_x = boxes.last().map_or(abs.right() - 2, |&(_, x)| x);
        let mut title_end = boxes_x - 1;
        if let Some(n) = w.number {
            if abs.w >= 16 {
                buf.put(boxes_x - 2, abs.y, b'0' + (n % 10), fa, clip);
                title_end = boxes_x - 3;
            }
        }

        if !w.title.is_empty() {
            // The title takes the frame's own colour. The classic palettes gave it no
            // entry of its own, and it does not need one: a title in a colour
            // the frame is not reads as a label stuck on rather than part of
            // the window.
            let ta = fa;
            // A modal window says so after its name - `[modal]`, the
            // brackets white and the word green - because a window that
            // will not let the others be touched, and does not say why,
            // reads as a desktop that has stopped working.
            //
            // The same tag says `[view]` on a window whose text cannot be
            // typed into and `[hex]` on a hex dump: a viewer is the same
            // blue as an editor, as it was in the file managers people
            // learned on, and the word is how the two are told apart.
            let tag = self.title_tag(id, w);
            let extra = tag.map_or(0, |t| t.chars().count() as i16 + 3);
            let start = abs.x + 2;
            let room = title_end - start;
            // Too long for the room left of the boxes: cut from the front,
            // `..` in place of what went - a commander's title is a folder,
            // and the end of a path is the part that says where you are.
            let full = w.title.chars().count() as i16;
            let keep = room - 2 - extra;
            let title: String = if full <= keep {
                w.title.clone()
            } else if keep >= 6 {
                let tail: String = w.title.chars().skip((full - (keep - 2)) as usize).collect();
                format!("..{tail}")
            } else {
                String::new()
            };
            let n = title.chars().count() as i16 + 2 + extra;
            if !title.is_empty() && n <= room {
                // Centred on the window when that clears the boxes; pushed
                // left of them when it does not.
                let tx = (abs.x + (abs.w - n) / 2).min(title_end - n).max(start);
                buf.put(tx, abs.y, b' ', ta, clip);
                let mut used = buf.text(tx + 1, abs.y, &title, ta, clip);
                if let Some(t) = tag {
                    let bg = fa & 0xF0;
                    let bracket = bg | crate::cell::Color::White as u8;
                    let word = bg | crate::cell::Color::LightGreen as u8;
                    let x = tx + 1 + used;
                    buf.put(x, abs.y, b' ', ta, clip);
                    buf.put(x + 1, abs.y, b'[', bracket, clip);
                    let n = buf.text(x + 2, abs.y, t, word, clip);
                    buf.put(x + 2 + n, abs.y, b']', bracket, clip);
                    used += extra;
                }
                buf.put(tx + 1 + used, abs.y, b' ', ta, clip);
            }
        }

        if !w.footer.is_empty() && abs.w > 8 {
            buf.text(abs.x + 2, abs.bottom() - 1, &w.footer, fa, clip);
        }

        // Scrollbars belong to the frame, but their state belongs to whatever
        // is inside. Ask the first child; if it does not scroll, no bars.
        let (v, h) = self.bars(id);

        if let Some(v) = v {
            let x = abs.right() - 1;
            buf.put(x, v.start, glyph::ARROW_UP, p.scroll, clip);
            buf.put(x, v.end(), glyph::ARROW_DOWN, p.scroll, clip);
            buf.vline(x, v.start + 1, v.track, glyph::MEDIUM_SHADE, p.scroll, clip);
            buf.put(x, v.thumb_at(), glyph::SQUARE, p.scroll, clip);
        }

        if let Some(h) = h {
            let y = abs.bottom() - 1;
            buf.put(h.start, y, glyph::ARROW_LEFT, p.scroll, clip);
            buf.put(h.end(), y, glyph::ARROW_RIGHT, p.scroll, clip);
            buf.hline(h.start + 1, y, h.track, glyph::MEDIUM_SHADE, p.scroll, clip);
            buf.put(h.thumb_at(), y, glyph::SQUARE, p.scroll, clip);
        }
    }

    /// The first child, if it is an editable text view.
    fn text_child(&self, id: ViewId) -> Option<ViewId> {
        let first = *self.nodes[id.ix()].children.first()?;
        match self.nodes[first.ix()].kind {
            Kind::Text(_) => Some(first),
            _ => None,
        }
    }

    /// How much of a panel's bottom-right corner belongs to something else.
    ///
    /// The buttons are drawn over the foot rather than beside it, so the panel
    /// has to be told where to stop writing. It cannot work this out: it has
    /// no idea it has siblings, and giving it one would be giving every view a
    /// reason to look at the tree.
    fn foot_taken(&self, id: ViewId) -> i16 {
        let Some(parent) = self.nodes[id.ix()].parent else {
            return 0;
        };
        match self.button_row(parent) {
            Some(row) => self.nodes[row.ix()].rect.w + 1,
            None => 0,
        }
    }

    /// The dialog's own buttons: the row docked against an edge, which is
    /// where Enter finds its default and Escape its Cancel. Rows placed by
    /// hand - a keypad - are buttons too, but they are not *the* buttons.
    fn button_row(&self, win: ViewId) -> Option<ViewId> {
        self.nodes[win.ix()]
            .children
            .iter()
            .copied()
            .find(|c| matches!(self.nodes[c.ix()].kind, Kind::Buttons(_)) && self.nodes[c.ix()].dock != Dock::Manual)
    }

    /// Every row of buttons in a window, docked or placed.
    fn button_rows(&self, win: ViewId) -> Vec<ViewId> {
        self.nodes[win.ix()]
            .children
            .iter()
            .copied()
            .filter(|c| matches!(self.nodes[c.ix()].kind, Kind::Buttons(_)))
            .collect()
    }

    /// Turn a button on or off by its place in its row.
    pub fn set_button_enabled(&mut self, row: ViewId, ix: usize, on: bool) -> bool {
        match &mut self.nodes[row.ix()].kind {
            Kind::Buttons(b) => match b.buttons.get_mut(ix) {
                Some(btn) => {
                    btn.enabled = on;
                    true
                }
                None => false,
            },
            _ => false,
        }
    }

    /// A command from a button, waiting to be collected. The same door as the
    /// menu's, because it is the same kind of news.
    pub fn take_pressed(&mut self) -> Option<Cmd> {
        loop {
            let win = self.active_window()?;
            let mut got = None;
            for row in self.button_rows(win) {
                if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                    if let Some(cmd) = b.pressed.take() {
                        got = Some(cmd);
                        break;
                    }
                }
            }
            let cmd = got?;
            // The toolkit's own dialogs answer to the toolkit, not to the
            // application: their buttons never come out of here.
            if !self.internal_command(cmd) {
                return Some(cmd);
            }
        }
    }

    /// Escape: the button marked `cancel`, wherever its row is, and
    /// failing that the last button of the docked row, which is Cancel by
    /// convention. A dialog with neither simply cannot be escaped from,
    /// which is a thing it is allowed to want.
    fn press_cancel(&mut self, win: ViewId) -> bool {
        for row in self.button_rows(win) {
            if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                if let Some(ix) = b.buttons.iter().position(|x| x.cancel) {
                    b.press_by_key(ix);
                    return true;
                }
            }
        }
        if let Some(row) = self.button_row(win) {
            if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                if !b.buttons.is_empty() {
                    let last = b.buttons.len() - 1;
                    b.press_by_key(last);
                    return true;
                }
            }
        }
        false
    }

    /// The window a view is in: itself if it is one, else the nearest
    /// window above it.
    pub fn window_of(&self, id: ViewId) -> Option<ViewId> {
        let mut at = Some(id);
        while let Some(v) = at {
            if matches!(self.nodes[v.ix()].kind, Kind::Window(_)) {
                return Some(v);
            }
            at = self.nodes[v.ix()].parent;
        }
        None
    }

    /// Enter, with nothing else wanting it: the default button, wherever
    /// its row is - the docked row first, then any placed one.
    fn press_default(&mut self, win: ViewId) -> bool {
        let mut rows = self.button_rows(win);
        if let Some(docked) = self.button_row(win) {
            rows.retain(|r| *r != docked);
            rows.insert(0, docked);
        }
        for row in rows {
            if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                if let Some(ix) = b.default_ix() {
                    b.press_by_key(ix);
                    return true;
                }
            }
        }
        false
    }

    /// A file panel in the active window, if there is one.
    fn files_id(&self) -> Option<ViewId> {
        let win = self.active_window()?;
        let first = *self.nodes[win.ix()].children.first()?;
        matches!(self.nodes[first.ix()].kind, Kind::Files(_)).then_some(first)
    }

    /// A name the person settled on. Opening it — reading a directory, loading
    /// a file — is the application's business, as all I/O is.
    pub fn take_chosen(&mut self) -> Option<String> {
        let id = self.files_id()?;
        match &mut self.nodes[id.ix()].kind {
            Kind::Files(f) => f.chosen.take(),
            _ => None,
        }
    }

    fn files_key(&mut self, win: ViewId, id: ViewId, k: Key) {
        use crate::event::KeyCode as K;
        use crate::files::Focus;

        // Escape leaves, always and from anywhere. A dialog with a way out
        // that depends on where the focus is has, for the person holding it, a
        // way out that sometimes is not there.
        // Tab belongs to the dialog, not to whatever is holding the focus.
        // It was below the buttons' own key handling before, so once the
        // buttons had the focus their catch-all swallowed it and the ring
        // stopped turning — the fault being not in the ring but in who got
        // asked first.
        if k.code == K::Tab || k.code == K::BackTab {
            return self.step_focus(win, id, k.code == K::BackTab);
        }

        if k.code == K::Esc {
            self.press_cancel(win);
            return;
        }

        // The buttons, when the focus is on them.
        if let Some(row) = self.button_row(win) {
            let focused = matches!(&self.nodes[row.ix()].kind, Kind::Buttons(b) if b.focused);
            if focused {
                let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind else {
                    return;
                };
                match k.code {
                    K::Left => b.step(-1),
                    K::Right => b.step(1),
                    K::Enter | K::Char(' ') => {
                        let c = b.current;
                        b.press_by_key(c);
                    }
                    K::Up => {
                        b.focused = false;
                        if let Kind::Files(f) = &mut self.nodes[id.ix()].kind {
                            f.focus = Focus::List;
                        }
                    }
                    // A bare letter is not a button's business even here: it
                    // belongs to the path field and to jumping about the list,
                    // and a button that answers to one steals it from both.
                    _ => {}
                }
                return;
            }
        }

        let Kind::Files(f) = &mut self.nodes[id.ix()].kind else {
            return;
        };
        let rows = f.rows();

        // Re-read, for a directory that changed underneath. Every dialog of
        // this kind needs one, because every such directory eventually does.
        if k.code == K::F(5) {
            f.pending_path = Some(f.path.text.clone());
            return;
        }
        if f.focus == Focus::Path {
            // Down leaves the line for the list, which is what a hand reaching
            // for the arrow keys means by it.
            if k.code == K::Down {
                f.focus = Focus::List;
                f.path.focused = false;
                return;
            }
            f.edit_path(k);
            return;
        }

        match k.code {
            K::Down => f.step(1),
            K::Up => f.step(-1),
            K::Insert => f.toggle_mark(),
            // Left and Right move by a whole column, because the names flow
            // downward. Moving by one entry would look like the cursor
            // jumping about at random.
            K::Right => f.step(rows),
            K::Left => f.step(-rows),
            K::PageDown => f.step(rows * f.cols()),
            K::PageUp => f.step(-rows * f.cols()),
            K::Home => f.home(),
            K::End => f.end(),
            // Enter on a directory enters it; on a file it is the same as
            // pressing Open, because that is what the person meant and making
            // them reach for the button to say it again is ceremony.
            K::Enter => {
                let is_dir = f.selected().map(|e| e.is_dir()).unwrap_or(false);
                f.choose();
                if !is_dir {
                    self.press_default(win);
                }
            }
            // A letter jumps to a name. It used to be pushed into the path
            // field along with the focus, which put the letter where nobody
            // was looking and moved the next keystroke somewhere else again.
            K::Char(c) if k.mods.is_none() => {
                f.jump_to(c);
            }
            _ => {}
        }
    }

    /// The first child, if it is anything that scrolls at all.
    /// The word after a window's title, if it has one: what the window
    /// is that its name does not say.
    fn title_tag<'a>(&self, id: ViewId, w: &'a Window) -> Option<&'a str> {
        if w.modal {
            return Some("modal");
        }
        if !w.tag.is_empty() {
            return Some(&w.tag);
        }
        let first = self.scrolling_child(id)?;
        match &self.nodes[first.ix()].kind {
            Kind::Text(t) if t.readonly => Some("view"),
            Kind::Hex(_) => Some("hex"),
            _ => None,
        }
    }

    /// Make a text view a viewer, or an editor again. A viewer keeps no
    /// caret and takes no focus, so turning one on lets go of both.
    pub fn set_readonly(&mut self, id: ViewId, on: bool) -> bool {
        match &mut self.nodes[id.ix()].kind {
            Kind::Text(t) => {
                t.readonly = on;
                if on {
                    t.focused = false;
                }
                true
            }
            _ => false,
        }
    }

    fn scrolling_child(&self, id: ViewId) -> Option<ViewId> {
        let first = *self.nodes[id.ix()].children.first()?;
        match self.nodes[first.ix()].kind {
            Kind::Text(_) | Kind::Html(_) | Kind::Files(_) | Kind::Hex(_) | Kind::Console(_) => Some(first),
            _ => None,
        }
    }

    /// Both scrollbars of a window, or `None` for an axis that does not scroll.
    ///
    /// One calculation, used by the drawing and by the hit testing alike. Two
    /// calculations that agree today are two calculations that stop agreeing
    /// the first time either is touched, and the symptom — a bar you can see
    /// but cannot press — looks like a mouse bug rather than an arithmetic
    /// one.
    fn bars(&self, id: ViewId) -> (Option<Bar>, Option<Bar>) {
        let Some(tid) = self.scrolling_child(id) else {
            return (None, None);
        };
        let abs = self.abs_rect(id);
        let inner = abs.inset(1);

        // A help page is wrapped to the width it was given, so it has nothing
        // to scroll sideways and gets no horizontal bar. Offering one that
        // never moves is worse than offering none.
        // The last value is the horizontal page: how much of the content one
        // panel's width holds. For text that is characters and for a file
        // panel it is whole columns, which is why it cannot just be `inner.w`.
        let (top, rows, left, cols, hpage) = match &self.nodes[tid.ix()].kind {
            // Folded text has nothing to scroll sideways, and its vertical
            // bar counts lines, not rows: counting rows would mean folding
            // the whole text on every frame to place one square.
            Kind::Text(t) if t.wrapping() => (t.top, t.line_count(), 0, 0, inner.w),
            Kind::Text(t) => (t.top, t.line_count(), t.left, t.longest(), inner.w),
            Kind::Html(h) => (h.top, h.line_count(), 0, 0, inner.w),
            // Folded to its width, like a help page: nothing sideways.
            Kind::Console(c) => (c.top, c.row_count(), 0, 0, inner.w),
            Kind::Hex(h) => (h.top, h.total_rows(), 0, 0, inner.w),
            // A file panel runs off the edge sideways, never downwards: the
            // names flow to the bottom of the column and then start a new one.
            // So it gets the horizontal bar and no vertical one, and the units
            // are columns rather than characters.
            Kind::Files(f) => {
                let (l, total) = f.overflow();
                (0, 0, l, total, f.cols())
            }
            _ => return (None, None),
        };

        let v = if inner.h >= 3 {
            Bar::new(abs.y + 1, inner.h, top, rows, inner.h)
        } else {
            None
        };

        // The bar is a guest on the bottom edge, not its owner: it starts where
        // the footer slot ends and runs to the resize grip, with no frame
        // character left between, or a stray piece of border sticks out of the
        // end of it.
        //
        // Measuring settled this. A plain classic window starts the bar two cells
        // in, because a plain window has nothing in its footer; the IDE's
        // editor starts it further along because its line:column indicator is
        // sitting there. One rule, both behaviours — and an empty footer
        // reproduces the classic layout exactly.
        let footer = match &self.nodes[id.ix()].kind {
            Kind::Window(w) => w.footer.chars().count() as i16,
            _ => 0,
        };
        let x0 = abs.x + 2 + footer;
        let hlen = abs.right() - 2 - x0;
        let h = if inner.w >= 12 && hlen >= 4 {
            Bar::new(x0, hlen, left, cols, hpage)
        } else {
            None
        };

        (v, h)
    }

    /// The file dialog's body.
    ///
    /// Grey dialog, coloured panels sitting on it with a margin. The margin is
    /// doing the work the rules used to do, and doing it better: a rule says
    /// "these are two regions" and a margin says "this is a thing on top of
    /// that thing", which is the truer description and the one the eye reads
    /// without being taught.
    /// Whether the window a view sits in is the active one. A cursor bar
    /// in a window nobody is working in is a second cursor, and two panels
    /// each with one is how a person deletes from the wrong side.
    fn parent_active(&self, id: ViewId) -> bool {
        match self.nodes[id.ix()].parent {
            Some(p) => self.active_window() == Some(p),
            None => true,
        }
    }

    fn draw_files(
        &self,
        f: &FileList,
        abs: Rect,
        buf: &mut Buffer,
        clip: Rect,
        wc: &crate::palette::WinColors,
        foot_taken: i16,
        active: bool,
    ) {
        let p = &self.palette;
        buf.fill(abs, b' ', wc.body, clip);

        if f.path_line {
            draw_input(&f.path, field_rect(abs), buf, clip, p);
        }

        let colw = f.column_width();
        let rows = f.rows();
        let list = Rect::new(abs.x + 1, abs.y + f.top(), abs.w - 2, rows);
        buf.fill(list, b' ', p.file_plain, clip);

        // Every divider, every time — not only the ones with names beside
        // them. The rules are the shape of the panel; drawing them only where
        // there is something to separate makes a half-empty directory look
        // like a different control.
        for col in 1..f.cols() {
            let x = list.x + col * colw;
            buf.vline(x - 1, list.y, rows, glyph::SL_V, p.file_frame, clip);
        }

        for col in 0..f.used_cols() {
            let x = list.x + col * colw;
            for row in 0..rows {
                let ix = ((f.left + col) * rows + row) as usize;
                let Some(e) = f.at(ix) else { continue };
                let y = list.y + row;
                let marked = f.is_marked(ix);
                // The cursor bar is drawn in the active window only: solid
                // while the list has the focus, dimmed while the path line
                // has it, gone when the window itself is behind another.
                let a = if ix == f.current && active {
                    let sel = if f.focus == crate::files::Focus::List {
                        p.file_selected
                    } else {
                        p.file_selected_passive
                    };
                    if marked { attr(attr_fg(p.list_marked), attr_bg(sel)) } else { sel }
                } else if marked {
                    p.list_marked
                } else {
                    match FileKind::of(e) {
                        FileKind::Dim => p.file_dim,
                        FileKind::Directory => p.file_dir,
                        FileKind::Executable => p.file_exe,
                        FileKind::Archive => p.file_archive,
                        FileKind::Temporary => p.file_temp,
                        FileKind::Plain => p.file_plain,
                    }
                };
                buf.fill(Rect::new(x, y, colw - 1, 1), b' ', a, clip);
                buf.text(x, y, &crate::files::fit(&e.name, colw - 2), a, clip);
            }
        }

        // The foot: two rows, the left of which says what the cursor is on and
        // the right of which is covered by the buttons that are drawn after
        // this. They share the height because a button needs a row for its
        // shadow and a caption does not, so stacking them would cost three
        // rows to say two things.
        let w = (abs.w - 2 - foot_taken).max(1);
        let h = f.foot_rows();
        let foot = Rect::new(abs.x + 1, abs.bottom() - h, w, h);
        // The foot sits on the window, so it takes the window's ground: the
        // measured black-on-grey of a dialog, and on a blue document window
        // white on blue, with the error in light red. A grey strip on a blue
        // panel was a piece of dialog that had wandered in.
        let ground = attr_bg(wc.body);
        let (info, error) = if ground == crate::cell::Color::LightGray {
            (p.file_info, p.file_error)
        } else {
            (
                attr(crate::cell::Color::White, ground),
                attr(crate::cell::Color::LightRed, ground),
            )
        };
        match &f.error {
            Some(_) => {
                buf.fill(foot, b' ', error, clip);
                for (i, line) in f.error_lines(w).iter().take(h as usize).enumerate() {
                    buf.text(foot.x, foot.y + i as i16, line, error, clip);
                }
            }
            None if f.details_only => {
                buf.fill(foot, b' ', info, clip);
                buf.text(foot.x, foot.y, &trim(&f.details(), w as usize), info, clip);
            }
            None => {
                buf.fill(foot, b' ', info, clip);
                buf.text(foot.x, foot.y, &trim(f.path_text(), w as usize), info, clip);
                buf.text(foot.x, foot.y + 1, &trim(&f.info(), w as usize), info, clip);
            }
        }
    }

    /// `OOOOOOOO  xx xx .. xx  |................|`
    ///
    /// The three parts are coloured differently because the eye uses them
    /// differently: the offsets to find a place, the bytes to read a value,
    /// the text beside them to recognise what the file is.
    fn draw_hex(&self, h: &HexView, abs: Rect, buf: &mut Buffer, clip: Rect) {
        use crate::hex::{byte_x, text_bar_x, PER_ROW};
        let p = &self.palette;
        buf.fill(abs, b' ', p.hex_byte, clip);
        let n = PER_ROW;

        for row in 0..abs.h {
            let base = (h.top + row) as usize * n as usize;
            if base >= h.bytes.len() {
                break;
            }
            let y = abs.y + row;
            buf.text(abs.x, y, &format!("{:08X}", base), p.hex_offset, clip);

            for i in 0..n as usize {
                let at = base + i;
                let Some(b) = h.bytes.get(at) else { break };
                let hx = abs.x + byte_x(i as i16);
                let tx = abs.x + text_bar_x() + 1 + i as i16;
                let here = at == h.cursor;

                let a = if here { p.hex_cursor } else { p.hex_byte };
                buf.text(hx, y, &format!("{:02X}", b), a, clip);

                let a = if here { p.hex_cursor } else { p.hex_text };
                // Anything unprintable becomes a dot. A control character sent
                // to the screen as itself moves the cursor or clears the line,
                // and a viewer that a file can reformat is not a viewer.
                let ch = if b.is_ascii_graphic() || *b == b' ' {
                    *b
                } else {
                    b'.'
                };
                buf.put(tx, y, ch, a, clip);
            }
            let bar = abs.x + text_bar_x();
            buf.put(bar, y, b'|', p.hex_offset, clip);
            buf.put(bar + 1 + n, y, b'|', p.hex_offset, clip);
        }
    }

    /// `[X] Label` or `(•) Label`, one to a row.
    ///
    /// The bracket shape is the whole visual difference between the two kinds,
    /// and it is enough: a square means "and", a round one means "or", and
    /// people read that without being told once.
    fn draw_cluster(&self, c: &Cluster, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let p = &self.palette;
        buf.fill(abs, b' ', p.ctl, clip);
        for (i, _) in c.items.iter().enumerate() {
            if i as i16 >= abs.h {
                break;
            }
            let y = abs.y + i as i16;
            let here = c.focused && i == c.current;
            let (a, ka) = match (here, c.enabled) {
                (true, true) => (p.ctl_focus, p.ctl_focus_key),
                (_, false) => (p.ctl_disabled, p.ctl_disabled),
                _ => (p.ctl, p.ctl_key),
            };
            let m: Vec<Glyph> = c.marker(i).iter().map(|&b| b as Glyph).collect();
            buf.raw(abs.x, y, &m, a, clip);
            let label = c.label(i);
            buf.fill(Rect::new(abs.x + 4, y, abs.w - 4, 1), b' ', a, clip);
            buf.text(abs.x + 4, y, &label, a, clip);
            if c.enabled {
                if let Some(h) = c.hotkey_at(i) {
                    if let Some(ch) = label.chars().nth(h) {
                        buf.put(abs.x + 4 + h as i16, y, ch as u8, ka, clip);
                    }
                }
            }
        }
    }

    /// A one-column bar down the right of a control that holds more than it
    /// can show, and the width left over for the content.
    ///
    /// Drawn by the control and not by a window frame, because a list in a
    /// dialog has no frame to hang one on. A list with more in it than is
    /// visible and nothing saying so is a list people believe they have read
    /// to the end of.
    fn draw_inner_scroll(
        &self,
        abs: Rect,
        top: i16,
        total: i16,
        page: i16,
        buf: &mut Buffer,
        clip: Rect,
    ) -> i16 {
        let Some((x, thumb_y)) = inner_bar(abs, top, total, page) else {
            return abs.w;
        };
        let p = &self.palette;
        buf.put(x, abs.y, glyph::ARROW_UP, p.list_selected, clip);
        buf.put(x, abs.bottom() - 1, glyph::ARROW_DOWN, p.list_selected, clip);
        buf.vline(x, abs.y + 1, abs.h - 2, glyph::MEDIUM_SHADE, p.list, clip);
        buf.put(x, thumb_y, glyph::SQUARE, p.list_selected, clip);
        abs.w - 1
    }

    fn draw_list(&self, l: &ListBox, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let p = &self.palette;
        buf.fill(abs, SP, p.list, clip);
        let w = self.draw_inner_scroll(abs, l.top, l.items.len() as i16, abs.h, buf, clip);
        for row in 0..abs.h {
            let Some(ix) = l.at_row(row) else { break };
            let marked = l.is_marked(ix);
            let a = if ix == l.current {
                let sel = if l.focused { p.list_selected } else { p.list_selected_passive };
                // A marked item under the cursor keeps its yellow: the mark
                // is the more important thing to see.
                if marked { attr(attr_fg(p.list_marked), attr_bg(sel)) } else { sel }
            } else if marked {
                p.list_marked
            } else {
                p.list
            };
            let y = abs.y + row;
            buf.fill(Rect::new(abs.x, y, w, 1), SP, a, clip);
            buf.text(abs.x, y, &crate::files::fit(&l.items[ix], w), a, clip);
        }
    }

    fn draw_tree(&self, t: &crate::tree::TreeView, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let p = &self.palette;
        buf.fill(abs, SP, p.list, clip);
        let rows = t.flatten();
        let w = self.draw_inner_scroll(abs, t.top, rows.len() as i16, abs.h, buf, clip);

        for row in 0..abs.h {
            let ix = (t.top + row) as usize;
            let Some(r) = rows.get(ix) else { break };
            let a = if ix == t.current {
                if t.focused {
                    p.list_selected
                } else {
                    p.list_selected_passive
                }
            } else {
                p.list
            };
            let y = abs.y + row;
            buf.fill(Rect::new(abs.x, y, w, 1), SP, a, clip);
            let pre = crate::tree::TreeView::prefix(r);
            buf.raw(abs.x, y, &pre, a, clip);
            let left = w - pre.len() as i16;
            if left > 0 {
                buf.text(
                    abs.x + pre.len() as i16,
                    y,
                    &crate::files::fit(&r.text, left),
                    a,
                    clip,
                );
            }
        }
    }

    fn draw_static(&self, t: &StaticText, abs: Rect, buf: &mut Buffer, clip: Rect, wc: &WinColors) {
        // Words take the window's text colour: black on the grey of a
        // dialog, yellow on the blue of a document. Drawn in the dialog's
        // colour everywhere, a caption on a blue window was a grey bar.
        let a = wc.text;
        buf.fill(abs, b' ', a, clip);
        for (i, line) in t.lines(abs.w).iter().enumerate() {
            if i as i16 >= abs.h {
                break;
            }
            buf.text(abs.x, abs.y + i as i16, line, a, clip);
        }
    }

    /// A row of buttons, centred.
    ///
    /// The default one wears arrows. "Which button does Enter press" is a
    /// question people ask of every dialog they meet, and a dialog that
    /// answers it without being asked is one they can use without stopping.
    fn draw_buttons(&self, r: &ButtonRow, abs: Rect, buf: &mut Buffer, clip: Rect, wc: &WinColors) {
        let p = &self.palette;
        // The shadow falls on whatever the buttons stand on - the window's
        // body - and takes its colour from that.
        let shadow = Palette::shadow_on(wc.body);
        for (i, b) in r.buttons.iter().enumerate() {
            // A held button loses its shadow and stays exactly where it was.
            // Moving it as well is the obvious thing and the wrong one: what
            // makes a button look pressed is that it has stopped standing
            // above the surface, and a button that also jumps is a button that
            // moved out from under the finger holding it.
            let held = r.down == Some(i);
            let x = abs.x + r.x_of(i, abs.w);
            let y = abs.y;
            let w = b.width();

            let sel = r.focused && i == r.current;
            let (a, ka) = match (b.style, sel, b.enabled) {
                (_, _, false) => (p.button_disabled, p.button_disabled),
                (crate::button::ButtonStyle::Normal, true, _) => (p.button_focus, p.button_focus_key),
                (crate::button::ButtonStyle::Normal, false, _) => (p.button, p.button_key),
                (crate::button::ButtonStyle::Accent, true, _) => (p.button_accent_focus, p.button_accent_key),
                (crate::button::ButtonStyle::Accent, false, _) => (p.button_accent, p.button_accent_key),
                (crate::button::ButtonStyle::Danger, true, _) => (p.button_danger_focus, p.button_danger_key),
                (crate::button::ButtonStyle::Danger, false, _) => (p.button_danger, p.button_danger_key),
            };

            if !held {
                // Half blocks, not whole cells. A cell-tall shadow under a
                // one-cell-tall button is as thick as the button and reads as
                // a second button; `▀` puts the dark at the top of the row
                // below, hugging the edge, which is what a shadow looks like.
                buf.hline(x + 1, y + 1, w, 0xDF as Glyph, shadow, clip);
                buf.vline(x + w, y, 1, 0xDD as Glyph, shadow, clip);
            }

            buf.fill(Rect::new(x, y, w, 1), b' ', a, clip);
            let label = b.label();
            let lx = x + 2;
            buf.text(lx, y, &label, a, clip);
            if b.enabled {
                if let Some(h) = b.hotkey_at() {
                    if let Some(c) = label.chars().nth(h) {
                        buf.put(lx + h as i16, y, c as u8, ka, clip);
                    }
                }
            }
            if b.default {
                buf.put(x, y, 0x10 as Glyph, a, clip); // >
                buf.put(x + w - 1, y, 0x11 as Glyph, a, clip); // <
            }
        }
    }

    /// The hint of the menu item the cursor stands on, if a panel is open
    /// and the item has one.
    fn menu_hint(&self) -> Option<String> {
        let mb = self.menu_box_id()?;
        match &self.nodes[mb.ix()].kind {
            Kind::MenuBox(m) => m.items.get(m.current).filter(|it| !it.hint.is_empty()).map(|it| it.hint.clone()),
            _ => None,
        }
    }

    fn draw_status(&self, s: &crate::status::StatusLine, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let p = &self.palette;
        buf.fill(abs, b' ', p.status, clip);
        // While a menu item with a hint is under the cursor, the line says
        // what the item does instead of what the keys do - as the
        // classic menus did. The keys come back the moment the menu closes.
        if let Some(hint) = self.menu_hint() {
            buf.text(abs.x + 1, abs.y, &hint, p.status, clip);
            return;
        }
        for (i, it) in s.items.iter().enumerate() {
            let x = abs.x + s.item_x(i);
            let a = if it.enabled { p.status } else { p.status_disabled };
            let label = it.label();
            buf.text(x, abs.y, &label, a, clip);
            if it.enabled {
                if let Some((start, len)) = it.key_span() {
                    let part: String = label.chars().skip(start).take(len).collect();
                    buf.text(x + start as i16, abs.y, &part, p.status_key, clip);
                }
            }
        }
        // The words at the right end - an editor's line and column, or a
        // window's own - where they cover nothing. A part between tildes is
        // lit: green, as a drive's light was.
        let shown: String = s.right.chars().filter(|&c| c != '~').collect();
        let len = shown.chars().count() as i16;
        let mut x = abs.right() - len;
        if len > 0 && x >= abs.x + s.items_end() + 2 {
            let lit = (p.status & 0xF0) | crate::cell::Color::Green as u8;
            for (i, part) in s.right.split('~').enumerate() {
                x += buf.text(x, abs.y, part, if i % 2 == 1 { lit } else { p.status }, clip);
            }
        }
    }

    fn draw_label(&self, l: &crate::controls::Label, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let p = &self.palette;
        // Lit when the control it names has the focus - the eye then finds
        // the caret by finding the bright words.
        let lit = l.target.is_some() && self.focused() == l.target;
        let a = if lit { p.label_active } else { p.label };
        let label = l.label();
        buf.fill(abs, b' ', a, clip);
        buf.text(abs.x, abs.y, &label, a, clip);
        if let Some(h) = l.hotkey_at() {
            if let Some(c) = label.chars().nth(h) {
                buf.put(abs.x + h as i16, abs.y, c as u8, p.label_key, clip);
            }
        }
    }

    fn draw_progress(&self, pr: &crate::controls::Progress, abs: Rect, buf: &mut Buffer, clip: Rect, wc: &WinColors) {
        let a = Palette::progress_on(wc.body);
        // Room for ` 100%` at the right, if asked for and if it fits.
        let text = if pr.percent { pr.percent_text() } else { String::new() };
        let tw = if text.is_empty() { 0 } else { text.chars().count() as i16 + 1 };
        let bar_w = if abs.w - tw >= 4 { abs.w - tw } else { abs.w };
        let done = pr.filled(bar_w);
        buf.hline(abs.x, abs.y, done, glyph::FULL_BLOCK, a, clip);
        buf.hline(abs.x + done, abs.y, bar_w - done, glyph::LIGHT_SHADE, a, clip);
        if bar_w < abs.w {
            buf.text(abs.x + bar_w + 1, abs.y, &text, wc.body, clip);
        }
    }

    /// The bar. Two spaces in front of the first label and two between each
    /// pair — measured, not chosen.
    fn draw_menu_bar(&self, m: &MenuBar, abs: Rect, buf: &mut Buffer, clip: Rect) {
        let p = &self.palette;
        buf.fill(abs, b' ', p.menu, clip);
        for (i, it) in m.items.iter().enumerate() {
            let open = m.open == Some(i);
            let (a, ka) = match (open, it.enabled) {
                (true, _) => (p.menu_selected, p.menu_selected_key),
                (_, false) => (p.menu_disabled, p.menu_disabled),
                _ => (p.menu, p.menu_key),
            };
            let x = abs.x + m.item_x(i);
            let label = it.label();
            buf.text(x, abs.y, &label, a, clip);
            if it.enabled {
                if let Some(h) = it.hotkey_at() {
                    if let Some(c) = label.chars().nth(h) {
                        buf.put(x + h as i16, abs.y, c as u8, ka, clip);
                    }
                }
            }
        }
    }

    /// A panel. One column of padding, then a single-line frame, then the
    /// items: label at the left, shortcut against the right edge.
    fn draw_menu_box(
        &self,
        m: &MenuBox,
        abs: Rect,
        buf: &mut Buffer,
        clip: Rect,
        parent_clip: Rect,
    ) {
        let p = &self.palette;
        // The shadow goes outside the panel, so it is drawn against the clip
        // the panel was handed rather than its own.
        self.draw_shadow(abs, buf, parent_clip);

        buf.fill(abs, b' ', p.menu, clip);
        let fx = abs.x + 1;
        let fw = abs.w - 2;
        buf.hline(fx + 1, abs.y, fw - 2, glyph::SL_H, p.menu, clip);
        buf.hline(fx + 1, abs.bottom() - 1, fw - 2, glyph::SL_H, p.menu, clip);
        buf.vline(fx, abs.y + 1, abs.h - 2, glyph::SL_V, p.menu, clip);
        buf.vline(fx + fw - 1, abs.y + 1, abs.h - 2, glyph::SL_V, p.menu, clip);
        buf.put(fx, abs.y, glyph::SL_TL, p.menu, clip);
        buf.put(fx + fw - 1, abs.y, glyph::SL_TR, p.menu, clip);
        buf.put(fx, abs.bottom() - 1, glyph::SL_BL, p.menu, clip);
        buf.put(fx + fw - 1, abs.bottom() - 1, glyph::SL_BR, p.menu, clip);

        let ix0 = fx + 1;
        let iw = fw - 2;
        for (i, it) in m.items.iter().enumerate() {
            let y = abs.y + 1 + i as i16;
            if it.separator {
                buf.hline(ix0, y, iw, glyph::SL_H, p.menu, clip);
                buf.put(fx, y, 0xC3 as Glyph, p.menu, clip); // ├
                buf.put(fx + fw - 1, y, 0xB4 as Glyph, p.menu, clip); // ┤
                continue;
            }
            let sel = i == m.current && it.enabled;
            let (a, ka) = match (sel, it.enabled) {
                (true, _) => (p.menu_selected, p.menu_selected_key),
                (_, false) => (p.menu_disabled, p.menu_disabled),
                _ => (p.menu, p.menu_key),
            };
            buf.fill(Rect::new(ix0, y, iw, 1), b' ', a, clip);
            if !it.items.is_empty() {
                buf.put(ix0 + iw - 1, y, 0x10 as Glyph, a, clip); // ► there is more
            }
            let label = it.label();
            buf.text(ix0 + 1, y, &label, a, clip);
            if it.enabled {
                if let Some(h) = it.hotkey_at() {
                    if let Some(c) = label.chars().nth(h) {
                        buf.put(ix0 + 1 + h as i16, y, c as u8, ka, clip);
                    }
                }
            }
            if it.checked {
                buf.put(ix0, y, 0xFB as Glyph, a, clip); // √ in the column before the label
            }
            if !it.shortcut.is_empty() {
                let n = it.shortcut.chars().count() as i16;
                buf.text(ix0 + iw - 1 - n, y, &it.shortcut, a, clip);
            }
        }
    }

    fn draw_html(&self, h: &Html, abs: Rect, buf: &mut Buffer, clip: Rect, p: &WinColors) {
        buf.fill(abs, b' ', p.text, clip);
        for row in 0..abs.h {
            let Some(line) = h.lines().get((h.top + row) as usize) else {
                break;
            };
            for (col, c) in line.iter().enumerate() {
                if col as i16 >= abs.w {
                    break;
                }
                let a = match c.style {
                    Style::Text => p.text,
                    // The classic help viewers had no bold and no headings of their
                    // own; both borrow the selected-text colour, which on a
                    // cyan body is the brightest thing available.
                    Style::Bold | Style::Heading => p.text_selected,
                    Style::Link if c.link as usize == h.focus => p.link_focus,
                    Style::Link => p.link,
                };
                buf.put(abs.x + col as i16, abs.y + row, c.ch, a, clip);
            }
        }
    }

    fn draw_text(&self, t: &TextView, abs: Rect, buf: &mut Buffer, clip: Rect, p: &WinColors) {
        // A memo is a box sunk into a dialog; a document is the window it
        // fills. The first needs a colour of its own or nobody sees that it is
        // something to type into, and the second must not have one, because it
        // *is* the window.
        let body = if t.boxed {
            if t.focused {
                self.palette.memo_focus
            } else {
                self.palette.memo
            }
        } else {
            p.body
        };

        // Blank space keeps the window’s own colour rather than the text’s.
        // On screen the two are the same — a space shows only its background —
        // but in the bytes they are not, and the bytes are what we check
        // ourselves against.
        buf.fill(abs, SP, body, clip);
        if t.wrapping() {
            return self.draw_folded(t, abs, buf, clip, p, body);
        }
        for row in 0..abs.h {
            let li = t.top as usize + row as usize;
            let Some(line) = t.lines.get(li) else { break };
            let a = if t.highlight_line && li as i16 == t.cur.y {
                let a = p.text_selected;
                buf.fill(Rect::new(abs.x, abs.y + row, abs.w, 1), b' ', a, clip);
                a
            } else if t.boxed {
                body
            } else {
                p.text
            };
            let skip = t.left.max(0) as usize;
            if skip < line.len() {
                let take = (abs.w as usize).min(line.len() - skip);
                buf.raw(abs.x, abs.y + row, &line[skip..skip + take], a, clip);
                self.paint_syntax(t, li, skip, skip + take, abs.x, abs.y + row, a, buf, clip);
            }

            // The selection is painted after the text and recolours it, so a
            // selected blank at the end of a line still shows as selected —
            // which is how you can see that the line break is included.
            if t.anchor.is_some() {
                for col in 0..abs.w {
                    if t.is_selected(li as i16, t.left + col) {
                        buf.recolor(abs.x + col, abs.y + row, p.text_selected, clip);
                    }
                }
            }
        }
    }

    /// Recolour characters `from..to` of line `li`, drawn from column `x`,
    /// by what they are in the text's language. Over the colour the line
    /// was drawn in, so a barred caret line keeps its bar.
    #[allow(clippy::too_many_arguments)]
    fn paint_syntax(&self, t: &TextView, li: usize, from: usize, to: usize, x: i16, y: i16, base: u8, buf: &mut Buffer, clip: Rect) {
        let Some(s) = t.syntax.and_then(|s| self.syntaxes.get(s as usize)) else { return };
        let Some(line) = t.lines.get(li) else { return };
        let state = if li < t.states_valid { t.states[li] } else { 0 };
        let mut roles = Vec::new();
        s.paint(line, state, Some(&mut roles));
        for (k, r) in roles[from.min(roles.len())..to.min(roles.len())].iter().enumerate() {
            if *r != crate::syntax::Role::Text {
                buf.recolor(x + k as i16, y, self.palette.syntax_on(*r, base), clip);
            }
        }
    }

    /// Text with its lines folded: row after row from the first on the
    /// screen, each a piece of a line. Everything else is as `draw_text`
    /// does it - the caret's line barred if asked, the selection over it.
    fn draw_folded(&self, t: &TextView, abs: Rect, buf: &mut Buffer, clip: Rect, p: &WinColors, body: u8) {
        let mut at = Some(t.first_row());
        for row in 0..abs.h {
            let Some((li, r)) = at else { break };
            at = t.next_row(li, r);
            let line = &t.lines[li as usize];
            let starts = t.rows_of(li);
            let s = starts[r as usize];
            let next = starts.get(r as usize + 1).copied();
            let e = next.unwrap_or(line.len());
            let y = abs.y + row;
            let a = if t.highlight_line && li == t.cur.y {
                buf.fill(Rect::new(abs.x, y, abs.w, 1), b' ', p.text_selected, clip);
                p.text_selected
            } else if t.boxed {
                body
            } else {
                p.text
            };
            buf.raw(abs.x, y, &line[s..e], a, clip);
            self.paint_syntax(t, li as usize, s, e, abs.x, y, a, buf, clip);
            if t.anchor.is_some() {
                // A row that goes on in the next one ends at its last
                // character; only the line's last row shows the line break
                // selected, as an unfolded line does.
                let cols = match next {
                    Some(n) => (n - s) as i16,
                    None => abs.w,
                };
                for col in 0..cols.min(abs.w) {
                    if t.is_selected(li, s as i16 + col) {
                        buf.recolor(abs.x + col, y, p.text_selected, clip);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ events

    pub fn handle(&mut self, ev: Event) {
        self.compose_bars();
        match ev {
            Event::Resize(w, h) => self.resize(w, h),
            Event::Mouse(m) => self.handle_mouse(m),
            Event::Key(k) => self.handle_key(k),
            Event::Tick => {}
        }
    }

    /// Topmost window under a point, if any.
    fn window_at(&self, p: Point) -> Option<ViewId> {
        // While a modal is up it is the only window there is, as far as the
        // pointer is concerned.
        if let Some(m) = self.modal() {
            return self.abs_rect(m).contains(p).then_some(m);
        }
        self.nodes[self.root.ix()]
            .children
            .iter()
            .rev()
            .copied()
            .find(|id| !self.is_minimized(*id) && self.abs_rect(*id).contains(p))
    }

    /// The second meaning of what is under the pointer. The `Down` that
    /// came with the double click has already been handled: the window
    /// is active and the cursor is on the entry.
    fn double_click(&mut self, p: Point) {
        if self.menu_box_id().is_some() {
            return;
        }
        let Some(id) = self.window_at(p) else { return };
        let abs = self.abs_rect(id);
        if p.y == abs.y {
            // The title bar: zoom, or back. Not on the close box and not
            // on the zoom box, which have meanings of their own and got
            // them from the click already. And no drag: the press that
            // came before this started one, and a zoomed window that is
            // also being dragged is two answers to one gesture.
            let (zoomable, on_box) = match &self.nodes[id.ix()].kind {
                Kind::Window(w) => (w.zoomable, title_boxes(w, abs).iter().any(|&(_, x)| p.x >= x && p.x <= x + 2)),
                _ => (false, false),
            };
            if zoomable && !on_box {
                self.drag = None;
                self.toggle_zoom(id);
            }
            return;
        }
        let Some(hit) = self.nodes[id.ix()].children.iter().copied().find(|c| self.abs_rect(*c).contains(p)) else {
            return;
        };
        let r = self.abs_rect(hit);
        let (row, col) = (p.y - r.y, p.x - r.x);
        let press = match &mut self.nodes[hit.ix()].kind {
            // A name: what Enter does to it. A folder is entered; a file is
            // chosen and the dialog's default button pressed.
            Kind::Files(f) => {
                if f.at_point(col, row).is_none() {
                    return;
                }
                let is_dir = f.selected().map(|e| e.is_dir()).unwrap_or(false);
                f.choose();
                !is_dir
            }
            // An item in a list: chosen, which is the default button.
            Kind::List(l) => l.at_row(row).is_some(),
            _ => false,
        };
        if press {
            self.press_default(id);
        }
    }

    fn handle_mouse(&mut self, m: Mouse) {
        let p = Point::new(m.x, m.y);

        match m.kind {
            MouseKind::Double(b) => {
                // A double click is a click first - it puts the cursor on
                // the thing - and then that thing's second meaning.
                self.handle_mouse(Mouse { x: m.x, y: m.y, kind: MouseKind::Down(b) });
                if b == Button::Left {
                    self.double_click(p);
                }
            }
            MouseKind::Down(Button::Left) => {
                // A click outside a modal window does nothing at all. Not
                // "activates what is under it quietly" — nothing, because the
                // person is being asked a question and the answer is not over
                // there.
                if let Some(m) = self.modal() {
                    if !self.abs_rect(m).contains(p) && self.menu_box_id().is_none() {
                        return;
                    }
                }

                // An open panel gets the click, wherever it landed: inside it
                // picks an item, outside it puts the menu away. A click that
                // both closed a menu and did something behind it would be one
                // click doing two things the user only asked for once.
                let boxes = self.menu_boxes();
                if !boxes.is_empty() {
                    // The topmost panel under the pointer gets the click. A
                    // click on a parent panel with a submenu open closes the
                    // submenu and chooses there, as it would in the original.
                    let hit_box = boxes.iter().rev().copied().find(|b| self.abs_rect(*b).contains(p));
                    match hit_box {
                        Some(mb) => {
                            while self.menu_box_id().is_some_and(|top| top != mb) {
                                self.close_top_menu();
                            }
                            let abs = self.abs_rect(mb);
                            let hit = match &self.nodes[mb.ix()].kind {
                                Kind::MenuBox(m) => m.item_at(p.y - abs.y),
                                _ => None,
                            };
                            if let Some(ix) = hit {
                                if let Kind::MenuBox(m) = &mut self.nodes[mb.ix()].kind {
                                    m.current = ix;
                                }
                                self.pick_menu_slowly(mb);
                            }
                        }
                        None => self.close_menu(),
                    }
                    return;
                }

                if let Some(bar) = self.menu_bar_id() {
                    let abs = self.abs_rect(bar);
                    if abs.contains(p) {
                        let hit = match &self.nodes[bar.ix()].kind {
                            Kind::MenuBar(m) => m.item_at(p.x - abs.x),
                            _ => None,
                        };
                        if let Some(ix) = hit {
                            self.open_menu(ix);
                        }
                        return;
                    }
                }

                // The status line: a click on an item is its command.
                if let Some(sid) = self.status_id() {
                    let abs = self.abs_rect(sid);
                    if abs.contains(p) {
                        if let Kind::Status(s) = &self.nodes[sid.ix()].kind {
                            if let Some(ix) = s.item_at(p.x - abs.x) {
                                let it = &s.items[ix];
                                if it.enabled && it.cmd != 0 {
                                    let cmd = it.cmd;
                                    self.emit(cmd);
                                }
                            }
                        }
                        return;
                    }
                }

                // The bars of minimized windows lie on the desktop, under
                // the windows: only a press no window took reaches them.
                let Some(id) = self.window_at(p) else {
                    if let Some(b) = self.bar_at(p) {
                        self.bar_click(b, p);
                    }
                    return;
                };
                // The close and zoom boxes, and the resize grip, are drawn
                // on the active window only, so on an inactive one there is
                // nothing there to click: the click activates it, and the
                // next click is the one that reaches a box. Deciding by the
                // coordinates alone closed windows nobody had looked at yet.
                let was_active = self.active_window() == Some(id);
                self.activate(id);
                let abs = self.abs_rect(id);
                let Kind::Window(w) = &self.nodes[id.ix()].kind else {
                    return;
                };
                let (movable, resizable) = (w.movable, w.resizable && was_active);
                let boxes = if was_active { title_boxes(w, abs) } else { Vec::new() };

                // Frame hits first, body second.
                if p.y == abs.y {
                    match boxes.iter().find(|&&(_, x)| p.x >= x && p.x <= x + 2).map(|&(b, _)| b) {
                        Some(TitleBox::Close) => {
                            let cmd = match &self.nodes[id.ix()].kind {
                                Kind::Window(w) => w.close_cmd,
                                _ => 0,
                            };
                            if cmd != 0 {
                                self.command = Some(cmd);
                            } else {
                                self.close(id);
                            }
                            return;
                        }
                        Some(TitleBox::Zoom) => return self.toggle_zoom(id),
                        Some(TitleBox::Minimize) => return self.minimize(id),
                        None => {}
                    }
                    if movable {
                        self.drag = Some(Drag {
                            id,
                            mode: DragMode::Move,
                            from: p,
                            orig: self.nodes[id.ix()].rect,
                        });
                    }
                    return;
                }
                if resizable && p.x >= abs.right() - 2 && p.y == abs.bottom() - 1 {
                    self.drag = Some(Drag {
                        id,
                        mode: DragMode::Resize,
                        from: p,
                        orig: self.nodes[id.ix()].rect,
                    });
                    return;
                }

                // Scrollbars. They sit on the frame, so this has to come after
                // the frame's own controls and before anything decides the
                // click landed on the body.
                let (v, h) = self.bars(id);
                if let Some(v) = v {
                    if p.x == abs.right() - 1 && p.y >= v.start && p.y <= v.end() {
                        self.click_bar(id, Axis::Vertical, v, p.y, p);
                        return;
                    }
                }
                if let Some(h) = h {
                    if p.y == abs.bottom() - 1 && p.x >= h.start && p.x <= h.end() {
                        self.click_bar(id, Axis::Horizontal, h, p.x, p);
                        return;
                    }
                }

                // A button: it goes down now and does its work on release.
                // The focus stays where it was - the classic buttons did
                // not take it on a click either - so a keypad pressed with
                // the mouse leaves the caret in the display it types into.
                for row in self.button_rows(id) {
                    let r = self.abs_rect(row);
                    if !r.contains(p) {
                        continue;
                    }
                    let hit = match &self.nodes[row.ix()].kind {
                        Kind::Buttons(b) => b.at(p.x - r.x, r.w),
                        _ => None,
                    };
                    if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                        b.down = hit;
                        if let Some(i) = hit {
                            b.current = i;
                        }
                    }
                    return;
                }

                // A canvas: the click is remembered, in its own cells, for
                // the program to ask about. The canvas itself does nothing
                // with it - it does not know what its cells are.
                let on_canvas = self.nodes[id.ix()].children.iter().copied().find(|c| {
                    matches!(self.nodes[c.ix()].kind, Kind::Canvas(_)) && self.abs_rect(*c).contains(p)
                });
                if let Some(cv) = on_canvas {
                    let r = self.abs_rect(cv);
                    if let Kind::Canvas(c) = &mut self.nodes[cv.ix()].kind {
                        c.clicked = Some((p.x - r.x, p.y - r.y));
                    }
                    return;
                }

                // A label: the click goes to the control it names.
                let labelled = self.nodes[id.ix()].children.iter().copied().find(|c| {
                    matches!(self.nodes[c.ix()].kind, Kind::Label(_)) && self.abs_rect(*c).contains(p)
                });
                if let Some(l) = labelled {
                    if let Kind::Label(lb) = &self.nodes[l.ix()].kind {
                        if let Some(target) = lb.target {
                            self.focus_on(id, target);
                        }
                    }
                    return;
                }

                // Anything in the focus ring that the pointer landed on. The
                // click both moves the focus there and does whatever a click
                // means to that control, because those are one action as far
                // as the hand is concerned: nobody clicks a list meaning only
                // to look at it.
                let chain = self.focus_chain(id);
                if let Some(hit) = chain.iter().copied().find(|c| self.abs_rect(*c).contains(p)) {
                    for c in &chain {
                        self.set_view_focus(*c, *c == hit);
                    }
                    self.control_click(hit, p);
                    return;
                }

                // A link under the pointer. One press both focuses and
                // follows: a help page is read with one hand, and making
                // people click twice to go somewhere is how you find out
                // nobody uses the help.
                if let Some(first) = self.scrolling_child(id) {
                    let inner = self.abs_rect(first);
                    if inner.contains(p) {
                        let (row, col) = (p.y - inner.y, p.x - inner.x);
                        let page = inner.h;
                        if let Kind::Html(hv) = &mut self.nodes[first.ix()].kind {
                            if let Some(ix) = hv.link_at(hv.top + row, col) {
                                hv.focus = ix;
                                hv.follow(page);
                            }
                        }
                    }
                }
            }
            MouseKind::Drag => {
                if let Some(id) = self.selecting.filter(|id| self.is_alive(*id)) {
                    return self.drag_selection(id, p);
                }
                if let Some(id) = self.bar_drag.filter(|id| self.is_alive(*id)) {
                    return self.inner_bar_drag(id, p);
                }
                // Dragging along the bar with a panel open switches between
                // them. The classic menus did this and it is how a menu is actually
                // used: press, slide, release.
                if let (Some(mb), Some(bar)) = (self.menu_box_id(), self.menu_bar_id()) {
                    let babs = self.abs_rect(bar);
                    if babs.contains(p) {
                        let hit = match &self.nodes[bar.ix()].kind {
                            Kind::MenuBar(m) => m.item_at(p.x - babs.x),
                            _ => None,
                        };
                        let cur = match &self.nodes[bar.ix()].kind {
                            Kind::MenuBar(m) => m.open,
                            _ => None,
                        };
                        if let Some(ix) = hit {
                            if cur != Some(ix) {
                                self.open_menu(ix);
                            }
                        }
                        return;
                    }
                    let mabs = self.abs_rect(mb);
                    if mabs.contains(p) {
                        let hit = match &self.nodes[mb.ix()].kind {
                            Kind::MenuBox(m) => m.item_at(p.y - mabs.y),
                            _ => None,
                        };
                        if let Some(ix) = hit {
                            if let Kind::MenuBox(m) = &mut self.nodes[mb.ix()].kind {
                                m.current = ix;
                            }
                        }
                        return;
                    }
                }
                self.continue_drag(p)
            }
            MouseKind::Down(Button::Right) => self.open_context(p),
            MouseKind::Up(_) => {
                self.drag = None;
                self.bar_drag = None;
                self.end_selection();
                // Release over the button that went down does the thing;
                // release anywhere else is how you change your mind.
                if let Some(win) = self.active_window() {
                    for row in self.button_rows(win) {
                        let r = self.abs_rect(row);
                        let over = r.contains(p);
                        if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                            if let Some(i) = b.down.take() {
                                if over && b.at(p.x - r.x, r.w) == Some(i) {
                                    b.press(i);
                                }
                            }
                        }
                    }
                }
            }
            MouseKind::ScrollUp => self.scroll_at(p, -3),
            MouseKind::ScrollDown => self.scroll_at(p, 3),
            _ => {}
        }
    }

    /// An arrow steps one, the track either side of the marker steps a page,
    /// and the marker itself starts a drag. That is the whole contract of a
    /// scrollbar and it has not needed changing since 1983.
    fn click_bar(&mut self, id: ViewId, axis: Axis, b: Bar, c: i16, at: Point) {
        let page = self.page(id, axis).max(1);
        let thumb = b.thumb_at();

        let delta = if c == b.start {
            -1
        } else if c == b.end() {
            1
        } else if c < thumb {
            -page
        } else if c > thumb {
            page
        } else {
            self.drag = Some(Drag {
                id,
                mode: DragMode::Thumb(axis),
                from: at,
                orig: self.nodes[id.ix()].rect,
            });
            return;
        };

        let pos = self.scroll_pos(id, axis) + delta;
        self.set_scroll(id, axis, pos);
    }

    fn page(&self, id: ViewId, axis: Axis) -> i16 {
        if let Some(tid) = self.scrolling_child(id) {
            if let Kind::Files(f) = &self.nodes[tid.ix()].kind {
                return f.cols();
            }
        }
        let inner = self.abs_rect(id).inset(1);
        match axis {
            Axis::Vertical => inner.h,
            Axis::Horizontal => inner.w,
        }
    }

    fn scroll_pos(&self, id: ViewId, axis: Axis) -> i16 {
        let Some(tid) = self.scrolling_child(id) else {
            return 0;
        };
        match (&self.nodes[tid.ix()].kind, axis) {
            (Kind::Text(t), Axis::Vertical) => t.top,
            (Kind::Text(t), Axis::Horizontal) => t.left,
            (Kind::Html(h), Axis::Vertical) => h.top,
            (Kind::Console(c), Axis::Vertical) => c.top,
            (Kind::Hex(h), Axis::Vertical) => h.top,
            (Kind::Files(f), Axis::Horizontal) => f.overflow().0,
            _ => 0,
        }
    }

    fn set_scroll(&mut self, id: ViewId, axis: Axis, pos: i16) {
        let Some(tid) = self.scrolling_child(id) else {
            return;
        };
        let page = self.page(id, axis);
        if let Kind::Files(f) = &mut self.nodes[tid.ix()].kind {
            if axis == Axis::Horizontal {
                f.scroll_columns(pos);
            }
            return;
        }
        if let Kind::Html(h) = &mut self.nodes[tid.ix()].kind {
            if axis == Axis::Vertical {
                // Through `scroll`, which knows where the page ends.
                let d = pos.max(0) - h.top;
                h.scroll(d, page);
            }
            return;
        }
        if let Kind::Hex(h) = &mut self.nodes[tid.ix()].kind {
            if axis == Axis::Vertical {
                let d = pos.max(0) - h.top;
                h.scroll(d);
            }
            return;
        }
        if let Kind::Console(c) = &mut self.nodes[tid.ix()].kind {
            if axis == Axis::Vertical {
                let d = pos.max(0) - c.top;
                c.scroll(d);
            }
            return;
        }
        let Kind::Text(t) = &mut self.nodes[tid.ix()].kind else {
            return;
        };
        match axis {
            // The bar of folded text counts lines: the square is on a line
            // and the view starts at that line's first row.
            Axis::Vertical if t.wrapping() => {
                t.top = pos.clamp(0, (t.line_count() - 1).max(0));
                t.top_row = 0;
            }
            Axis::Horizontal if t.wrapping() => {}
            Axis::Vertical => {
                let max = (t.line_count() - page).max(0);
                t.top = pos.clamp(0, max);
            }
            Axis::Horizontal => {
                let max = (t.longest() - page).max(0);
                t.left = pos.clamp(0, max);
            }
        }
    }

    fn continue_drag(&mut self, p: Point) {
        let Some(d) = &self.drag else { return };
        let (id, mode, from, orig) = (d.id, d.mode, d.from, d.orig);

        // Dragging a marker moves the content, not the window.
        if let DragMode::Thumb(axis) = mode {
            let (v, h) = self.bars(id);
            let bar = match axis {
                Axis::Vertical => v,
                Axis::Horizontal => h,
            };
            if let Some(b) = bar {
                let c = match axis {
                    Axis::Vertical => p.y,
                    Axis::Horizontal => p.x,
                };
                let pos = b.pos_for(c);
                self.set_scroll(id, axis, pos);
            }
            return;
        }

        let dx = p.x - from.x;
        let dy = p.y - from.y;
        let screen = self.nodes[self.root.ix()].rect;

        // The floor is structural: two cells of frame either way, and room
        // for something between them. What a window asks for on top of that is
        // its own business.
        let (min_w, min_h, max_w, max_h) = match &self.nodes[id.ix()].kind {
            Kind::Window(w) => (
                w.min_w.max(6),
                w.min_h.max(3),
                if w.max_w > 0 { w.max_w } else { i16::MAX },
                if w.max_h > 0 { w.max_h } else { i16::MAX },
            ),
            _ => (4, 3, i16::MAX, i16::MAX),
        };

        let r = match mode {
            DragMode::Move => Rect::new(
                (orig.x + dx).clamp(1 - orig.w, screen.w - 1),
                (orig.y + dy).clamp(0, screen.h - 1),
                orig.w,
                orig.h,
            ),
            DragMode::Resize => Rect::new(
                orig.x,
                orig.y,
                (orig.w + dx).clamp(min_w, max_w),
                (orig.h + dy).clamp(min_h, max_h),
            ),
            DragMode::Thumb(_) => return, // dealt with above
        };
        self.nodes[id.ix()].rect = r;
        if let Kind::Window(w) = &mut self.nodes[id.ix()].kind {
            w.centred = false;
        }
    }

    /// The wheel: whatever scrolls under the pointer - a list or a tree in a
    /// dialog, a memo, a console - and, over the rest of a window, the
    /// window's own text. People turn the wheel before they read anything.
    fn scroll_at(&mut self, p: Point, delta: i16) {
        if !self.menu_boxes().is_empty() {
            return;
        }
        let Some(id) = self.window_at(p) else { return };
        let under = self.nodes[id.ix()].children.iter().copied().find(|c| {
            matches!(
                self.nodes[c.ix()].kind,
                Kind::List(_) | Kind::Tree(_) | Kind::Text(_) | Kind::Console(_) | Kind::Html(_) | Kind::Hex(_) | Kind::Files(_)
            ) && self.abs_rect(*c).contains(p)
        });
        let Some(target) = under.or_else(|| self.nodes[id.ix()].children.first().copied()) else {
            return;
        };
        let page = self.abs_rect(target).h;
        match &mut self.nodes[target.ix()].kind {
            Kind::Text(t) if t.wrapping() => t.scroll_rows(delta, page),
            Kind::Text(t) => {
                let max = (t.lines.len() as i16 - page).max(0);
                t.top = (t.top + delta).clamp(0, max);
            }
            Kind::Html(h) => h.scroll(delta, page),
            Kind::Hex(h) => h.scroll(delta),
            Kind::Console(c) => c.scroll(delta),
            Kind::List(l) => l.scroll(delta),
            Kind::Tree(t) => t.scroll(delta),
            // A file panel's names flow down and then across: the wheel moves
            // the cursor through them, as a file manager's does.
            Kind::Files(f) => f.step(delta),
            _ => {}
        }
    }

    fn handle_key(&mut self, k: Key) {
        use crate::event::KeyCode as K;

        // An open menu takes every key. That is what makes it a menu rather
        // than a decoration: while it is up, nothing behind it is listening.
        if let Some(mb) = self.menu_box_id() {
            return self.menu_key(mb, k);
        }

        // F10 opens the bar, and Alt with a letter opens the one it belongs
        // to. These are the only keys the bar owns while it is closed.
        if self.menu_bar_id().is_some() && self.modal().is_none() {
            if k.code == K::F(10) {
                self.open_menu(0);
                return;
            }
            if k.mods.alt {
                if let K::Char(c) = k.code {
                    if let Some(ix) = self.bar_hotkey(c) {
                        self.open_menu(ix);
                        return;
                    }
                }
            }
        }

        // A window being moved from the keyboard has every key until Enter
        // or Escape lets go of it.
        if self.sizing.is_some() {
            return self.sizing_key(k);
        }

        // The desktop's own keys, the ones the classic DOS desktops had:
        // Alt+1..9, Alt+0, Shift+F6, Ctrl+F5. Before the status line, so a
        // program's F6 does not shadow Shift+F6.
        if self.desktop_keys && self.modal().is_none() {
            match (k.code, k.mods.alt, k.mods.shift, k.mods.ctrl) {
                (K::Char(c), true, false, false) if c.is_ascii_digit() => {
                    if c == '0' {
                        self.window_list();
                    } else {
                        self.activate_numbered(c as u8 - b'0');
                    }
                    return;
                }
                (K::F(6), false, true, false) => {
                    self.cycle_windows_back();
                    return;
                }
                (K::F(5), false, false, true) => {
                    self.begin_size_move();
                    return;
                }
                _ => {}
            }
        }

        // The status line binds keys of its own - F1, Alt-X - and turns
        // them into commands. Not while a modal dialog is up: it is asking
        // something, and Help over an unanswered question is the mess
        // modality exists to prevent.
        if self.modal().is_none() {
            if let Some(cmd) = self.status_command(k) {
                self.emit(cmd);
                return;
            }
        }

        let Some(win) = self.active_window() else {
            return;
        };

        // A label's hotkey puts the focus on the control it names. Before
        // the buttons, so that ~N~ame beside a field wins over a ~N~ext
        // button: the field is what the person is looking at.
        if k.mods.alt {
            if let crate::event::KeyCode::Char(c) = k.code {
                if let Some(target) = self.label_target(win, c) {
                    self.focus_on(win, target);
                    return;
                }
            }
        }

        // A button's hotkey is Alt and its letter, and it works wherever the
        // focus happens to be. That is what makes it a shortcut rather than
        // another way to press a button you are already standing on.
        //
        // It is tried after the menu bar and not before. A dialog with a
        // ~F~ind button would otherwise swallow Alt+F and leave the File menu
        // unreachable, and an unreachable menu is the worse of the two
        // failures by a long way.
        if k.mods.alt {
            if let crate::event::KeyCode::Char(c) = k.code {
                for row in self.button_rows(win) {
                    let hit = match &self.nodes[row.ix()].kind {
                        Kind::Buttons(b) => b.by_hotkey(c),
                        _ => None,
                    };
                    if let Some(ix) = hit {
                        if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                            b.press_by_key(ix);
                        }
                        return;
                    }
                }
            }
        }

        let Some(first) = self.nodes[win.ix()].children.first().copied() else {
            return;
        };
        if matches!(self.nodes[first.ix()].kind, Kind::Html(_)) {
            return self.html_key(first, k);
        }
        if matches!(self.nodes[first.ix()].kind, Kind::Files(_)) {
            return self.files_key(win, first, k);
        }

        // An ordinary dialog: Tab walks the ring, Escape presses the last
        // button, and everything else goes to whatever is holding the focus.
        // Any window with something focusable in it, not only one with
        // several: a window holding a single list and no buttons still has
        // to give that list its keys, and it used to give them to nobody.
        if !self.focus_chain(win).is_empty() || self.button_row(win).is_some() {
            use crate::event::KeyCode as K;
            if k.code == K::Tab || k.code == K::BackTab {
                self.advance_focus(k.code == K::BackTab);
                return;
            }
            if k.code == K::Esc {
                // A console's selection is let go of first; the next
                // Escape is the window's.
                if let Some(f) = self.focused_or_first() {
                    if let Kind::Console(c) = &mut self.nodes[f.ix()].kind {
                        if c.selection().is_some() {
                            c.select_none();
                            return;
                        }
                    }
                }
                self.press_cancel(win);
                return;
            }
            if let Some(f) = self.focused_or_first() {
                if !matches!(self.nodes[f.ix()].kind, Kind::Text(_)) {
                    return self.control_key(win, f, k);
                }
                return self.control_key(win, f, k);
            }
        }
        if matches!(self.nodes[first.ix()].kind, Kind::Hex(_)) {
            return self.hex_key(first, k);
        }

        // The key never reaches the editor. It is turned into a named command
        // here, by a table, and the editor is handed the command — which is
        // why there can be two keymaps and why a test can drive the editor
        // without pretending to be a keyboard.
        self.text_key(first, k)
    }

    /// A help page has no caret, so it answers to a different, much shorter
    /// set of keys: Tab walks the links, Enter follows one, and the rest
    /// scrolls. The classic help browsers worked exactly so.
    /// Walk the dialog's focus ring: the path field, the list, the buttons,
    /// and round again.
    ///
    /// One function and one ring. It was two before — the panel toggled
    /// between its field and its list, and the buttons took the focus from the
    /// side — and the two disagreed about where Tab went from the list. The
    /// field then could not be reached at all, which is the kind of fault that
    /// only shows up when somebody actually uses the thing.
    fn step_focus(&mut self, win: ViewId, files: ViewId, back: bool) {
        use crate::files::Focus;

        let has_buttons = self.button_row(win).is_some();
        let on_buttons = match self.button_row(win) {
            Some(row) => matches!(&self.nodes[row.ix()].kind, Kind::Buttons(b) if b.focused),
            None => false,
        };
        let (here, has_path) = if on_buttons {
            (2, true)
        } else {
            match &self.nodes[files.ix()].kind {
                Kind::Files(f) if f.focus == Focus::Path => (0, true),
                Kind::Files(f) => (1, f.path_line),
                _ => (1, true),
            }
        };

        // The stops: path line (if there is one), names, buttons (if any).
        // Tab walks them in that order and wraps.
        let mut stops: Vec<usize> = Vec::new();
        if has_path {
            stops.push(0);
        }
        stops.push(1);
        if has_buttons {
            stops.push(2);
        }
        let at = stops.iter().position(|&s| s == here).unwrap_or(0);
        let n = stops.len();
        let next = stops[if back { (at + n - 1) % n } else { (at + 1) % n }];

        if let Some(row) = self.button_row(win) {
            if let Kind::Buttons(b) = &mut self.nodes[row.ix()].kind {
                b.focused = next == 2;
            }
        }
        if let Kind::Files(f) = &mut self.nodes[files.ix()].kind {
            f.focus = if next == 0 { Focus::Path } else { Focus::List };
            f.path.focused = next == 0;
        }
    }

    // ------------------------------------------------------------ the focus
    //
    // One ring per window, walked by Tab. A view is in it if there is
    // something to do there — text that can be typed into, a choice that can
    // be made, a button that can be pressed. Static text is not, and neither
    // is a view that has been disabled, because a stop that does nothing is a
    // stop people learn to Tab past.

    fn is_focusable(&self, id: ViewId) -> bool {
        match &self.nodes[id.ix()].kind {
            Kind::Text(t) => !t.readonly,
            Kind::Tree(_) => true,
            Kind::Input(_) | Kind::Files(_) | Kind::Html(_) | Kind::Hex(_) | Kind::List(_) | Kind::Console(_) => true,
            Kind::Cluster(c) => c.enabled,
            Kind::Buttons(b) => b.selectable && !b.buttons.is_empty(),
            _ => false,
        }
    }

    fn focus_chain(&self, win: ViewId) -> Vec<ViewId> {
        self.nodes[win.ix()]
            .children
            .iter()
            .copied()
            .filter(|c| self.is_focusable(*c))
            .collect()
    }

    fn set_view_focus(&mut self, id: ViewId, on: bool) {
        match &mut self.nodes[id.ix()].kind {
            Kind::Input(i) => i.focused = on,
            Kind::Cluster(c) => c.focused = on,
            Kind::List(l) => l.focused = on,
            Kind::Tree(t) => t.focused = on,
            Kind::Text(t) => t.focused = on,
            Kind::Buttons(b) => b.focused = on,
            Kind::Files(f) => {
                f.path.focused = false;
                f.focus = crate::files::Focus::List;
            }
            _ => {}
        }
    }

    /// Which view in this window actually holds the focus, and `None` when
    /// none of them does yet.
    ///
    /// It used to answer "the first one" in that case, which reads as
    /// helpfulness and is a trap: `advance_focus` then believed it was already
    /// on the first and moved to the second, so opening a dialog and pressing
    /// Tab skipped the control the eye had started on.
    pub fn focused(&self) -> Option<ViewId> {
        let win = self.active_window()?;
        self.focus_chain(win)
            .iter()
            .copied()
            .find(|id| match &self.nodes[id.ix()].kind {
                Kind::Input(i) => i.focused,
                Kind::Cluster(c) => c.focused,
                Kind::List(l) => l.focused,
                Kind::Tree(t) => t.focused,
                Kind::Text(t) => t.focused,
                Kind::Buttons(b) => b.focused,
                _ => false,
            })
    }

    /// The one holding the focus, or the one that would hold it first. For
    /// routing a key, where there has to be an answer.
    fn focused_or_first(&self) -> Option<ViewId> {
        let win = self.active_window()?;
        self.focused().or_else(|| self.focus_chain(win).first().copied())
    }

    /// The commonest dialog there is: some words and a row of buttons.
    ///
    /// In the library rather than in every application, because every
    /// application needs it and none of them should spell it differently.
    /// The classic toolkits had a message box call for the same reason.
    ///
    /// Returns the window, so a caller can move it or add to it. The answer
    /// comes back through `take_pressed`, like any other button.
    pub fn message_box(&mut self, title: &str, text: &str, buttons: crate::button::ButtonRow) -> ViewId {
        use crate::controls::StaticText;

        let work = self.work_area();
        let body = StaticText::new(text);
        let w = 52.min(work.w - 4).max(24);
        let lines = body.lines(w - 6).len() as i16;
        let h = lines + 6;

        let mut win = Window::new(title);
        win.palette = crate::palette::WinPalette::Gray;
        win.modal = true;
        // Nothing to resize: it is as big as the words in it.
        win.resizable = false;
        win.zoomable = false;
        // And no close box. The classic one meant Cancel, but nothing here
        // knows which of these buttons is the cancel one — so a close box
        // would be a way of dismissing the question without answering it,
        // which is the one thing a modal dialog must not have.
        win.closable = false;
        win.min_w = w;
        win.max_w = w;
        win.min_h = h;
        win.max_h = h;

        let r = Rect::new(
            work.x + (work.w - w) / 2,
            work.y + (work.h - h) / 2,
            w,
            h,
        );
        let root = self.root;
        let wid = self.insert(root, r, Kind::Window(win));

        let txt = self.insert(wid, Rect::new(2, 1, w - 6, lines), Kind::Static(body));
        self.set_dock(txt, Dock::Manual);

        let row = self.insert(wid, Rect::default(), Kind::Buttons(buttons));
        self.set_dock(row, Dock::BottomRight(w - 2, 2));
        self.focus_first();
        wid
    }

    /// Put the focus on the first stop. What a dialog does when it opens.
    pub fn focus_first(&mut self) {
        let Some(win) = self.active_window() else {
            return;
        };
        let chain = self.focus_chain(win);
        for (i, id) in chain.iter().enumerate() {
            self.set_view_focus(*id, i == 0);
        }
    }

    /// Move the focus on by one stop, wrapping.
    pub fn advance_focus(&mut self, back: bool) {
        let Some(win) = self.active_window() else {
            return;
        };
        let chain = self.focus_chain(win);
        if chain.is_empty() {
            return;
        }
        let here = self.focused().and_then(|f| chain.iter().position(|c| *c == f));
        let n = chain.len();
        let next = match here {
            Some(i) if back => (i + n - 1) % n,
            Some(i) => (i + 1) % n,
            None => 0,
        };
        for (i, id) in chain.iter().enumerate() {
            self.set_view_focus(*id, i == next);
        }
    }

    fn bar_hotkey(&self, c: char) -> Option<usize> {
        let bar = self.menu_bar_id()?;
        let Kind::MenuBar(m) = &self.nodes[bar.ix()].kind else {
            return None;
        };
        let c = c.to_ascii_lowercase();
        m.items
            .iter()
            .position(|i| i.enabled && !i.items.is_empty() && i.hotkey() == Some(c))
    }

    /// Keys while a panel is open.
    fn menu_key(&mut self, mb: ViewId, k: Key) {
        use crate::event::KeyCode as K;

        // Left and Right walk the bar, but only for a panel that came from
        // one. A local menu has no bar to walk and should not pretend.
        let parent = match &self.nodes[mb.ix()].kind {
            Kind::MenuBox(m) => m.parent,
            _ => None,
        };

        match k.code {
            K::Esc => self.close_top_menu(),
            K::Enter => self.pick_menu(mb),
            K::Up | K::Down => {
                let d = if k.code == K::Up { -1 } else { 1 };
                if let Kind::MenuBox(m) = &mut self.nodes[mb.ix()].kind {
                    m.step(d);
                }
            }
            K::Left | K::Right => {
                // Right on a submenu item opens it; Left in a submenu
                // closes it. Otherwise both walk the bar - from whichever
                // panel in the chain came from the bar, so Right at the end
                // of a submenu still reaches the next menu along.
                let is_sub = parent.is_none() && self.menu_boxes().len() > 1;
                let (cur, has_sub) = match &self.nodes[mb.ix()].kind {
                    Kind::MenuBox(m) => (
                        m.current,
                        m.items.get(m.current).is_some_and(|i| !i.items.is_empty()),
                    ),
                    _ => (0, false),
                };
                if k.code == K::Right && has_sub {
                    self.open_submenu(mb, cur);
                } else if k.code == K::Left && is_sub {
                    self.close_top_menu();
                } else {
                    let bottom = self.menu_boxes().first().copied();
                    let from_bar = bottom.and_then(|b| match &self.nodes[b.ix()].kind {
                        Kind::MenuBox(m) => m.parent,
                        _ => None,
                    });
                    if let (Some(p), n) = (from_bar, self.bar_len()) {
                        if n > 0 {
                            let d = if k.code == K::Left { n - 1 } else { 1 };
                            self.open_menu((p + d) % n);
                        }
                    }
                }
            }
            K::Char(c) => {
                let hit = match &self.nodes[mb.ix()].kind {
                    Kind::MenuBox(m) => m.by_hotkey(c),
                    _ => None,
                };
                if let Some(ix) = hit {
                    if let Kind::MenuBox(m) = &mut self.nodes[mb.ix()].kind {
                        m.current = ix;
                    }
                    self.pick_menu(mb);
                }
            }
            _ => {}
        }
    }

    /// What a click means to the control it landed on.
    ///
    /// Separate from the focus it also gives, because the two are one action
    /// to the hand and two to the program: the focus moves for every control,
    /// and what happens next is different for each.
    /// Where a list or a tree is scrolled to and how many rows it has.
    fn inner_scroll_of(&self, id: ViewId) -> Option<(i16, i16)> {
        match &self.nodes[id.ix()].kind {
            Kind::List(l) => Some((l.top, l.items.len().min(i16::MAX as usize) as i16)),
            Kind::Tree(t) => Some((t.top, t.len().min(i16::MAX as usize) as i16)),
            _ => None,
        }
    }

    /// A list's or a tree's view by `delta` rows, its cursor left where it is.
    fn inner_scroll(&mut self, id: ViewId, delta: i16) {
        match &mut self.nodes[id.ix()].kind {
            Kind::List(l) => l.scroll(delta),
            Kind::Tree(t) => t.scroll(delta),
            _ => {}
        }
    }

    /// A press on the scroll bar a list or a tree draws in its last column:
    /// an arrow is a row, the track either side of the marker a page, and
    /// the marker itself is taken hold of. True if the press was on it.
    fn inner_bar_click(&mut self, id: ViewId, at: Point) -> bool {
        let r = self.abs_rect(id);
        let Some((top, total)) = self.inner_scroll_of(id) else { return false };
        let Some((x, thumb_y)) = inner_bar(r, top, total, r.h) else { return false };
        if at.x != x {
            return false;
        }
        let page = (r.h - 1).max(1);
        if at.y == r.y {
            self.inner_scroll(id, -1);
        } else if at.y == r.bottom() - 1 {
            self.inner_scroll(id, 1);
        } else if at.y < thumb_y {
            self.inner_scroll(id, -page);
        } else if at.y > thumb_y {
            self.inner_scroll(id, page);
        } else {
            self.bar_drag = Some(id);
        }
        true
    }

    /// The marker of a list's or a tree's bar, dragged: the view goes where
    /// the marker would be under the pointer.
    fn inner_bar_drag(&mut self, id: ViewId, p: Point) {
        let r = self.abs_rect(id);
        let Some((top, total)) = self.inner_scroll_of(id) else { return };
        let track = (r.h - 2).max(1);
        let span = (total - r.h).max(0);
        let off = (p.y - r.y - 1).clamp(0, track - 1);
        let pos = if track > 1 { (off as i32 * span as i32 / (track - 1) as i32) as i16 } else { 0 };
        self.inner_scroll(id, pos - top);
    }

    fn control_click(&mut self, id: ViewId, at: Point) {
        if self.inner_bar_click(id, at) {
            return;
        }
        let r = self.abs_rect(id);
        let (row, col) = (at.y - r.y, at.x - r.x);

        match &mut self.nodes[id.ix()].kind {
            Kind::Cluster(c) => {
                let i = row.max(0) as usize;
                if i < c.items.len() {
                    c.current = i;
                    // On the bracket, not on the label: clicking the words of
                    // an option to read them should not change the answer.
                    if col < 3 {
                        c.toggle();
                    }
                }
            }
            Kind::List(l) => {
                if let Some(i) = l.at_row(row) {
                    l.current = i;
                }
            }
            Kind::Tree(t) => {
                if let Some(i) = t.at_row(row) {
                    t.current = i;
                    // The sign is the hinge. Clicking the name selects;
                    // clicking the `+` or `-` opens or closes, which is what
                    // it looks like it would do.
                    let rows = t.flatten();
                    if let Some(rw) = rows.get(i) {
                        let pre = crate::tree::TreeView::prefix(rw).len() as i16;
                        if col == pre - 2 {
                            t.toggle();
                        }
                    }
                }
            }
            Kind::Input(i) => {
                // The ▼ at the end opens the history; anywhere else puts
                // the caret into the text - not into the field: the field
                // may be scrolled, and the column under the pointer is
                // that many characters past the first one showing.
                if !i.history.is_empty() && col == r.w - 1 {
                    self.open_history(id);
                    return;
                }
                let x = i.left(r.w) + (col - i.field_x()).max(0) as usize;
                i.set_cursor(x);
            }
            Kind::Files(f) => {
                // Row 0 is the path line, one column in from the panel's
                // edge (`field_rect`); the names start on row 2 - or on row
                // 0 in a panel without a path line.
                if f.path_line && row == 0 {
                    f.focus = crate::files::Focus::Path;
                    f.path.focused = true;
                    let field_w = (r.w - 2).max(1);
                    let x = f.path.left(field_w) + (col - 1 - f.path.field_x()).max(0) as usize;
                    f.path.set_cursor(x);
                } else if let Some(ix) = f.at_point(col, row) {
                    f.current = ix;
                    f.focus = crate::files::Focus::List;
                    f.path.focused = false;
                }
            }
            // The caret goes where the pointer is, and a selection starts
            // there: dragging with the button down is how a hand selects.
            Kind::Text(t) => {
                t.cur = text_point(t, row, col);
                t.anchor = Some(t.cur);
                self.selecting = Some(id);
            }
            Kind::Console(c) => {
                let p = c.point(row, col);
                c.select_from(p);
                self.selecting = Some(id);
            }
            _ => {}
        }
    }

    /// The mouse dragged with its button down over a text or a console
    /// being selected: the selection's far end follows it, and past the top
    /// or the bottom the view scrolls a row towards it.
    fn drag_selection(&mut self, id: ViewId, p: Point) {
        let r = self.abs_rect(id);
        let (row, col) = (p.y - r.y, p.x - r.x);
        let step = if row < 0 { -1 } else if row >= r.h { 1 } else { 0 };
        match &mut self.nodes[id.ix()].kind {
            Kind::Text(t) => {
                if step != 0 {
                    if t.wrapping() {
                        t.scroll_rows(step, r.h);
                    } else {
                        t.top = (t.top + step).clamp(0, (t.line_count() - r.h).max(0));
                    }
                }
                t.cur = text_point(t, row.clamp(0, (r.h - 1).max(0)), col.clamp(0, (r.w - 1).max(0)));
            }
            Kind::Console(c) => {
                if step != 0 {
                    c.scroll(step);
                }
                let q = c.point(row.clamp(0, (r.h - 1).max(0)), col.clamp(0, r.w.max(0)));
                c.select_to(q);
            }
            _ => {}
        }
    }

    /// A right click over a text or a console: its context menu, at the
    /// pointer - what every program a person has used since does there.
    /// A click outside the selection moves the caret there first, so what
    /// is cut or copied is what was clicked on; inside it, the selection
    /// stays to be acted on.
    fn open_context(&mut self, p: Point) {
        use crate::menu::MenuItem;
        if !self.menu_boxes().is_empty() {
            return self.close_menu();
        }
        if let Some(m) = self.modal() {
            if !self.abs_rect(m).contains(p) {
                return;
            }
        }
        let Some(win) = self.window_at(p) else { return };
        let target = self.nodes[win.ix()].children.iter().copied().find(|c| {
            matches!(self.nodes[c.ix()].kind, Kind::Text(_) | Kind::Console(_)) && self.abs_rect(*c).contains(p)
        });
        let Some(id) = target else { return };
        self.activate(win);
        let chain = self.focus_chain(win);
        if chain.contains(&id) {
            for c in &chain {
                self.set_view_focus(*c, *c == id);
            }
        }
        let r = self.abs_rect(id);
        let (row, col) = (p.y - r.y, p.x - r.x);
        let host = self.host_clip.on;
        let clip_full = !self.clipboard.is_empty();
        let item = |label: &str, key: &str, cmd: Cmd, hint: &str, on: bool| {
            let mut m = MenuItem::new(label, key, cmd).hint(hint);
            m.enabled = on;
            m
        };
        let items = match &mut self.nodes[id.ix()].kind {
            Kind::Text(t) => {
                let at = text_point(t, row, col);
                let inside = t.selection().is_some_and(|(a, b)| !(at.y, at.x).lt(&(a.y, a.x)) && (at.y, at.x).lt(&(b.y, b.x)));
                if !inside {
                    t.cur = at;
                    t.anchor = None;
                }
                let (ro, sel, text) = (t.readonly, t.selection().is_some(), t.hex.is_none());
                vec![
                    item("~U~ndo", "Ctrl+Z", CM_CTX_UNDO, "Take back the last change", text && !ro && !t.undo_stack.is_empty()),
                    MenuItem::line(),
                    item("Cu~t~", "Ctrl+X", CM_CTX_CUT, "The selected text to the clipboard, and out of the text", text && !ro && sel),
                    item("~C~opy", "Ctrl+C", CM_CTX_COPY, "The selected text to the clipboard", text && sel),
                    item("~P~aste", "Ctrl+V", CM_CTX_PASTE, "What is on the clipboard, into the text here", text && !ro && (host || clip_full)),
                    MenuItem::line(),
                    item("Select ~a~ll", "Ctrl+A", CM_CTX_SELECT_ALL, "The whole text", text),
                ]
            }
            Kind::Console(c) => {
                let sel = c.selection().is_some();
                vec![
                    item(if sel { "~C~opy" } else { "~C~opy all" }, "Ctrl+C", CM_CTX_COPY,
                        if sel { "The selected lines to the clipboard" } else { "Everything in the console to the clipboard" }, true),
                    item("Select ~a~ll", "Ctrl+A", CM_CTX_SELECT_ALL, "Everything in the console", true),
                ]
            }
            _ => return,
        };
        self.close_menu();
        let mut b = MenuBox::new(items);
        b.parent = None;
        let (w, h) = (b.width(), b.height());
        let screen = self.nodes[self.root.ix()].rect;
        let x = p.x.min((screen.w - w).max(0));
        // Below the pointer when there is room, above it when there is not.
        let y = if p.y + 1 + h <= screen.h { p.y + 1 } else { (p.y - h).max(0) };
        let root = self.root;
        self.insert(root, Rect::new(x, y, w, h), Kind::MenuBox(b));
        self.context_for = Some(id);
    }

    /// A context menu's command, on the view it was opened over.
    fn context_command(&mut self, cmd: Cmd) {
        use crate::edit::Cmd as E;
        let Some(id) = self.context_for.take().filter(|id| self.is_alive(*id)) else { return };
        match (&self.nodes[id.ix()].kind, cmd) {
            (Kind::Text(_), CM_CTX_UNDO) => self.edit(id, E::Undo, false),
            (Kind::Text(_), CM_CTX_CUT) => self.edit(id, E::Cut, false),
            (Kind::Text(_), CM_CTX_COPY) => self.edit(id, E::Copy, false),
            (Kind::Text(_), CM_CTX_PASTE) => self.edit(id, E::Paste, false),
            (Kind::Text(_), CM_CTX_SELECT_ALL) => self.edit(id, E::SelectAll, false),
            (Kind::Console(_), CM_CTX_COPY) => self.console_copy(id),
            (Kind::Console(_), CM_CTX_SELECT_ALL) => {
                if let Kind::Console(c) = &mut self.nodes[id.ix()].kind {
                    c.select_all();
                }
            }
            _ => {}
        }
    }

    /// The console's selection to the clipboard - or, with nothing
    /// selected, everything in it: what a bug report wants.
    fn console_copy(&mut self, id: ViewId) {
        let Kind::Console(c) = &self.nodes[id.ix()].kind else { return };
        let lines = if c.selection().is_some() { c.selected() } else { c.all() };
        self.clipboard = lines;
        self.host_clip.copied = self.host_clip.copied.wrapping_add(1);
    }

    /// The button came up: a press that never moved selected nothing.
    fn end_selection(&mut self) {
        let Some(id) = self.selecting.take() else { return };
        if let Kind::Text(t) = &mut self.nodes[id.ix()].kind {
            if t.anchor == Some(t.cur) {
                t.anchor = None;
            }
        }
    }

    /// Keys for whatever holds the focus in an ordinary dialog.
    fn control_key(&mut self, win: ViewId, id: ViewId, k: Key) {
        use crate::event::KeyCode as K;
        let page = self.abs_rect(id).h;

        match &mut self.nodes[id.ix()].kind {
            Kind::Cluster(c) => match k.code {
                // In radio buttons the arrows choose, not merely point:
                // the classic ones did, and a dot that stays behind while
                // the cursor moves shows one answer and means another.
                K::Up => {
                    c.step(-1);
                    if c.mode == crate::controls::Choice::One {
                        c.toggle();
                    }
                }
                K::Down => {
                    c.step(1);
                    if c.mode == crate::controls::Choice::One {
                        c.toggle();
                    }
                }
                // Space marks, Enter does not: Enter belongs to the dialog's
                // default button, and a control that takes it makes the one
                // key everybody presses mean something different here.
                K::Char(' ') => c.toggle(),
                K::Char(ch) => {
                    if let Some(i) = c.by_hotkey(ch) {
                        c.current = i;
                        c.toggle();
                    }
                }
                _ => {
                    if k.code == K::Enter {
                        self.press_default(win);
                    }
                }
            },
            Kind::List(l) => match k.code {
                K::Up => l.step(-1),
                K::Down => l.step(1),
                K::Insert => l.toggle_mark(),
                K::PageUp => l.step(-page),
                K::PageDown => l.step(page),
                K::Home => l.step(-(l.items.len() as i16)),
                K::End => l.step(l.items.len() as i16),
                _ => {
                    if k.code == K::Enter {
                        self.press_default(win);
                    }
                }
            },
            Kind::Input(i) => {
                if k.code == K::Enter {
                    // What was entered is worth remembering; then the
                    // dialog's default button, as Enter always is.
                    i.remember();
                    self.press_default(win);
                } else if k.code == K::Down && !i.history.is_empty() {
                    // Out of the borrow first: the panel is a new node.
                    let id = id;
                    self.open_history(id);
                } else if (k.code == K::Char('v') && k.mods.ctrl && !k.mods.alt) || (k.code == K::Insert && k.mods.shift) {
                    // Paste: the first line of the clipboard, at the caret -
                    // the host's clipboard first, when it shares one.
                    if self.host_clip.on {
                        self.host_clip.paste_wanted = Some(id);
                    } else if let Some(l) = self.clipboard.first() {
                        i.insert(&glyph_string(l));
                    }
                } else {
                    i.key(k);
                }
            }
            Kind::Buttons(b) => match k.code {
                K::Left => b.step(-1),
                K::Right => b.step(1),
                K::Enter | K::Char(' ') => {
                    let c = b.current;
                    b.press_by_key(c);
                }
                _ => {}
            },
            Kind::Text(_) => self.text_key(id, k),
            Kind::Tree(t) => match k.code {
                K::Up => t.step(-1),
                K::Down => t.step(1),
                // Right opens a branch and steps into one already open; Left
                // closes it and, when it is closed already, goes up to the
                // parent. That second half is what makes Left usable: in a
                // deep tree the thing wanted after closing a branch is nearly
                // always the branch above it.
                K::Right => t.expand(),
                K::Left => t.collapse(),
                K::Char(SP_CH) => t.toggle(),
                K::PageUp => t.step(-page),
                K::PageDown => t.step(page),
                _ => {
                    if k.code == K::Enter {
                        self.press_default(win);
                    }
                }
            },
            Kind::Hex(_) => self.hex_key(id, k),
            Kind::Html(_) => self.html_key(id, k),
            Kind::Console(_) => self.console_key(id, k),
            _ => {}
        }
    }

    /// Up, down and the page keys through the record; Home and End to its
    /// two ends, and End is how following the newest line starts again.
    fn console_key(&mut self, id: ViewId, k: Key) {
        use crate::event::KeyCode as K;
        let page = self.abs_rect(id).h.max(1);
        let Kind::Console(c) = &mut self.nodes[id.ix()].kind else {
            return;
        };
        match k.code {
            K::Up => c.scroll(-1),
            K::Down => c.scroll(1),
            K::PageUp => c.scroll(-page),
            K::PageDown => c.scroll(page),
            K::Home => c.home(),
            K::End => c.end(),
            K::Char(ch) if k.mods.ctrl && !k.mods.alt && ch.eq_ignore_ascii_case(&'a') => c.select_all(),
            K::Char(ch) if k.mods.ctrl && !k.mods.alt && ch.eq_ignore_ascii_case(&'c') => self.console_copy(id),
            // Ctrl+Insert, the older key for the same.
            K::Insert if k.mods.ctrl => self.console_copy(id),
            K::Esc => c.select_none(),
            _ => {}
        }
    }

    fn hex_key(&mut self, id: ViewId, k: Key) {
        use crate::event::KeyCode as K;
        let page = self.abs_rect(id).h;
        let Kind::Hex(h) = &mut self.nodes[id.ix()].kind else {
            return;
        };
        let row = h.per_row() as i64;
        match k.code {
            K::Left => h.step(-1),
            K::Right => h.step(1),
            K::Up => h.step(-row),
            K::Down => h.step(row),
            K::PageUp => h.step(-row * page as i64),
            K::PageDown => h.step(row * page as i64),
            K::Home if k.mods.ctrl => h.home(),
            K::End if k.mods.ctrl => h.end(),
            K::Home => h.step(-(h.cursor as i64 % row)),
            K::End => h.step(row - 1 - (h.cursor as i64 % row)),
            _ => {}
        }
    }

    fn html_key(&mut self, id: ViewId, k: Key) {
        use crate::event::KeyCode as K;
        let page = self.abs_rect(id).h;
        let Kind::Html(h) = &mut self.nodes[id.ix()].kind else {
            return;
        };
        match k.code {
            K::Tab => h.next_link(page),
            K::BackTab => h.prev_link(page),
            K::Enter => h.follow(page),
            K::Up => h.scroll(-1, page),
            K::Down => h.scroll(1, page),
            K::PageUp => h.scroll(-page, page),
            K::PageDown => h.scroll(page, page),
            K::Home => h.top = 0,
            K::End => h.top = (h.line_count() - page).max(0),
            _ => {}
        }
    }
}
