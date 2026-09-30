//! The parts HELLO and DEMO are built from, made from the core's own types.
//!
//! This is what the server (`lib/serve`) does for a request, done here
//! directly: a Rust program links the core and calls it, so there is no
//! wire - but the parts are the same ones the assembler, C and Pascal
//! programs ask for over INT 60h, with the same flags, so the Rust DEMO
//! reads part for part like theirs.

// HELLO takes a few of these, DEMO nearly all.
#![allow(dead_code)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use owlosui_core::files::Focus;
use owlosui_core::{
    Align, ButtonRow, ButtonStyle, Canvas, Cell, Dock, FileList, Glyph, Key, KeyCode, Kind, Mods, PushButton, Rect,
    StaticText, StatusItem, TextView, Ui, ViewId, WinPalette, Window,
};

// Window flags, as the wire has them.
pub const MODAL: u8 = 0x01;
pub const FIXED: u8 = 0x02;
pub const NO_ZOOM: u8 = 0x04;
pub const BLUE: u8 = 0x00;
pub const GREY: u8 = 0x20;
pub const DIALOG: u8 = GREY | FIXED | NO_ZOOM;

// A button's flags, likewise.
pub const DEFAULT: u8 = 0x01;
pub const ACCENT: u8 = 0x02;
pub const DANGER: u8 = 0x04;
pub const CANCEL: u8 = 0x10;

/// A window on the desktop. `x` or `y` of -1 centres it in the work area.
pub fn window(ui: &mut Ui, title: &str, x: i16, y: i16, w: i16, h: i16, flags: u8, close_cmd: u16) -> ViewId {
    let work = ui.work_area();
    let centred = x < 0 || y < 0;
    let x = if x < 0 { work.x + (work.w - w) / 2 } else { x };
    let y = if y < 0 { work.y + (work.h - h) / 2 } else { y };
    let mut win = Window::new(title);
    win.centred = centred;
    win.close_cmd = close_cmd;
    win.modal = flags & MODAL != 0;
    win.resizable = flags & FIXED == 0;
    win.zoomable = flags & NO_ZOOM == 0;
    win.palette = if flags & GREY != 0 { WinPalette::Gray } else { WinPalette::Blue };
    if !win.resizable {
        win.min_w = w;
        win.max_w = w;
        win.min_h = h;
        win.max_h = h;
    }
    let root = ui.root();
    ui.insert(root, Rect::new(x, y, w, h), Kind::Window(win))
}

/// Words, where they are put; a static line folds runs of spaces.
pub fn words(ui: &mut Ui, parent: ViewId, x: i16, y: i16, w: i16, h: i16, text: &str) -> ViewId {
    let id = ui.insert(parent, Rect::new(x, y, w, h), Kind::Static(StaticText::new(text)));
    ui.set_dock(id, Dock::Manual);
    id
}

pub fn set_words(ui: &mut Ui, id: ViewId, text: &str) {
    if let Kind::Static(t) = ui.kind_mut(id) {
        t.text = text.to_string();
    }
}

fn row_of(buttons: &[(&str, u16, u8)]) -> ButtonRow {
    ButtonRow::new(
        buttons
            .iter()
            .map(|&(label, cmd, flags)| {
                let mut b = PushButton::new(label, cmd);
                if flags & DEFAULT != 0 {
                    b = b.default();
                }
                if flags & CANCEL != 0 {
                    b = b.cancel();
                }
                b.style(if flags & DANGER != 0 {
                    ButtonStyle::Danger
                } else if flags & ACCENT != 0 {
                    ButtonStyle::Accent
                } else {
                    ButtonStyle::Normal
                })
            })
            .collect(),
    )
}

/// Buttons docked bottom right, as buttons are.
pub fn buttons(ui: &mut Ui, parent: ViewId, buttons: &[(&str, u16, u8)]) -> ViewId {
    let row = row_of(buttons);
    let w = row.width() + 3;
    let id = ui.insert(parent, Rect::default(), Kind::Buttons(row));
    ui.set_dock(id, Dock::BottomRight(w, 2));
    ui.focus_first();
    id
}

/// A grey modal box sized to its words, with buttons.
pub fn message_box(ui: &mut Ui, title: &str, text: &str, buttons: &[(&str, u16, u8)]) -> ViewId {
    ui.message_box(title, text, row_of(buttons))
}

/// A row placed by hand, out of the Tab ring: a keypad.
pub fn button_row(ui: &mut Ui, parent: ViewId, x: i16, y: i16, buttons: &[(&str, u16, u8)]) -> ViewId {
    let mut row = row_of(buttons);
    row.align = Align::Left;
    row.selectable = false;
    let w = row.width() + 1;
    let id = ui.insert(parent, Rect::new(x, y, w, 2), Kind::Buttons(row));
    ui.set_dock(id, Dock::Manual);
    settle_focus(ui);
    id
}

/// A dialog with nothing focused is one whose first key goes nowhere.
fn settle_focus(ui: &mut Ui) {
    if ui.focused().is_none() {
        ui.focus_first();
    }
}

pub fn canvas(ui: &mut Ui, parent: ViewId, x: i16, y: i16, w: i16, h: i16) -> ViewId {
    let id = ui.insert(parent, Rect::new(x, y, w, h), Kind::Canvas(Canvas::new(w, h)));
    ui.set_dock(id, Dock::Manual);
    id
}

/// Cells into a canvas: a glyph and a colour each. On DOS a glyph is the
/// byte the card draws, so a program puts glyph 1 in a cell and gets the
/// face - no character set in between.
pub fn blit(ui: &mut Ui, canvas: ViewId, w: i16, h: i16, cells: &[Cell]) {
    if let Kind::Canvas(c) = ui.kind_mut(canvas) {
        c.blit(0, 0, w, h, cells);
    }
}

/// Where the mouse last went down on a canvas, once.
pub fn clicked(ui: &mut Ui, canvas: ViewId) -> Option<(i16, i16)> {
    match ui.kind_mut(canvas) {
        Kind::Canvas(c) => c.clicked.take(),
        _ => None,
    }
}

/// An editor filling its window; read-only makes it a viewer, `[view]`.
/// An editor offers everything it has on an Edit menu of its own while its
/// window is in front; a viewer offers Find, Word wrap, the bytes in hex
/// and the language to colour it as. The core runs all of it - nothing
/// comes back to the program.
pub fn editor(ui: &mut Ui, parent: ViewId, lines: Vec<Vec<Glyph>>, readonly: bool) -> ViewId {
    use owlosui_core::edit::{offer, state};
    let id = ui.insert(parent, Rect::default(), Kind::Text(TextView::new(lines)));
    ui.set_dock(id, Dock::Fill);
    if readonly {
        ui.set_editor(id, offer::FIND | offer::WRAP | offer::HEX | offer::SYNTAX, state::READONLY);
    } else {
        ui.set_editor(id, offer::ALL, 0);
    }
    settle_focus(ui);
    id
}

/// Text as the core holds it: lines of glyphs. On DOS a byte of a file
/// is its glyph already.
pub fn lines_of(bytes: &[u8]) -> Vec<Vec<Glyph>> {
    let mut lines: Vec<Vec<Glyph>> = bytes
        .split(|&b| b == b'\n')
        .map(|l| l.iter().filter(|&&b| b != b'\r').map(|&b| if b == b'\t' { b' ' } else { b } as Glyph).collect())
        .collect();
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

// Keys, for a status line or the keys a window carries.
pub fn key(code: KeyCode) -> Option<Key> {
    Some(Key { code, mods: Mods::default() })
}

pub fn alt(code: KeyCode) -> Option<Key> {
    Some(Key { code, mods: Mods { alt: true, ..Mods::default() } })
}

pub fn item(label: &str, cmd: u16, k: Option<Key>) -> StatusItem {
    StatusItem::new(label, k, cmd)
}

// ------------------------------------------------------------------- files

/// A folder made into the panel's path line: C:\DIR\*.*
fn pattern_of(dir: &str) -> String {
    let mut p = String::from(dir);
    if !p.ends_with('\\') {
        p.push('\\');
    }
    p.push_str("*.*");
    p
}

/// A file panel filling its window, `top` rows left free above it, the
/// folder read through DOS. A folder DOS cannot read is an empty panel.
pub fn files(ui: &mut Ui, parent: ViewId, top: i16, dir: &str) -> ViewId {
    let entries = owlosui_dos::read_dir(dir).unwrap_or_default();
    let mut f = FileList::new(entries, "*.*");
    f.set_path(&pattern_of(dir));
    f.focus = Focus::List;
    let id = ui.insert(parent, Rect::new(0, top, 0, 0), Kind::Files(f));
    ui.set_dock(id, Dock::FillFrom(top));
    id
}

/// Another folder in the panel; false if DOS cannot read it.
pub fn set_files(ui: &mut Ui, panel: ViewId, dir: &str) -> bool {
    let Some(entries) = owlosui_dos::read_dir(dir) else { return false };
    if let Kind::Files(f) = ui.kind_mut(panel) {
        f.set_mask("*.*");
        f.set_entries(entries);
        f.set_path(&pattern_of(dir));
        f.error = None;
    }
    true
}

pub fn files_error(ui: &mut Ui, panel: ViewId, text: &str) {
    if let Kind::Files(f) = ui.kind_mut(panel) {
        f.error = Some(text.to_string());
    }
}

/// What the panel reports, once: 1 a name entered, 2 a path typed, 0 nothing.
pub fn take_files(ui: &mut Ui, panel: ViewId) -> (u8, String) {
    match ui.kind_mut(panel) {
        Kind::Files(f) => {
            if let Some(p) = f.pending_path.take() {
                (2, p)
            } else if let Some(n) = f.chosen.take() {
                (1, n)
            } else {
                (0, String::new())
            }
        }
        _ => (0, String::new()),
    }
}

/// The name under the cursor.
pub fn current_name(ui: &mut Ui, panel: ViewId) -> Option<String> {
    match ui.kind(panel) {
        Kind::Files(f) => f.selected().map(|e| e.name.clone()),
        _ => None,
    }
}
