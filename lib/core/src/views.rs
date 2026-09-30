//! The views themselves — data only. How they draw lives in `ui.rs`, because
//! drawing a view needs to walk its children, and the tree belongs to `Ui`.
//!
//! Note what is *not* here: no colours, no coordinates on screen, no back
//! pointer to a parent. A view knows what it is and what it contains. Where it
//! ends up and what colour it takes are decided above it.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::geom::Rect;
use crate::menu::MenuItem;
use crate::status::StatusItem;

/// The patterned background everything else sits on.
pub struct Desktop {
    /// The fill character. `░` is what Turbo Vision used; `▒` and `▓` are the
    /// other two shades, and any glyph is legal.
    pub glyph: crate::cell::Glyph,
}

impl Default for Desktop {
    fn default() -> Self {
        Desktop {
            glyph: crate::cell::glyph::LIGHT_SHADE,
        }
    }
}

/// A framed, movable, resizable window.
///
/// The frame is not decoration: its four edges carry the close box, the title,
/// the zoom box, both scrollbars, a footer slot and the resize grip. Turbo
/// Vision got more use out of one character of border than most toolkits get
/// out of a title bar.
pub struct Window {
    pub title: String,
    pub closable: bool,
    pub zoomable: bool,
    pub movable: bool,
    pub resizable: bool,
    /// Casts a shadow down and to the right. Two columns wide and one row
    /// deep, because a character cell is taller than it is wide and an equal
    /// offset would look lopsided.
    pub shadow: bool,
    /// Nothing behind it answers until it is gone.
    ///
    /// It can still be moved, because a dialog covering the thing it is asking
    /// about is a dialog you cannot answer. What it holds is the *attention*:
    /// keys and clicks outside it do nothing at all, rather than quietly doing
    /// something in a window nobody is looking at.
    pub modal: bool,
    /// Shown near the right end of the top edge. Borland numbered windows so
    /// Alt+1..Alt+9 could reach them; the number in the frame is what makes
    /// that shortcut discoverable instead of secret.
    pub number: Option<u8>,
    /// Which family of colours this window and everything in it belongs to.
    pub palette: crate::palette::WinPalette,
    /// Shown in the bottom-left of the frame — a position indicator, a byte
    /// count, whatever the application wants there.
    pub footer: String,
    /// Smallest size the window may be dragged down to, frame included.
    ///
    /// Zero means no preference, which is the default: most windows do not
    /// care and should not have to say so. A dialog does care — its contents
    /// stop making sense below a size it knows and the toolkit does not — and
    /// says what that size is.
    ///
    /// There is a floor underneath either way. A window narrower than its own
    /// frame is not a small window, it is a drawing bug.
    pub min_w: i16,
    pub min_h: i16,
    /// Largest it may be dragged up to. Zero means no limit, which is the
    /// default.
    ///
    /// Not the same thing as `resizable`, and deliberately not derived from
    /// it: a window that may grow but only so far is an ordinary thing, and a
    /// reader who finds no grip on a window should be able to see *why* in one
    /// field rather than by comparing two others.
    pub max_w: i16,
    pub max_h: i16,
    /// Keep it in the middle of the work area, whatever size that becomes.
    ///
    /// A dialog that was centred when it opened and is left in the top-left
    /// corner after the console grows was centred by accident. Dragging it
    /// anywhere turns this off: from then on it is where the hand put it.
    pub centred: bool,
    /// What the close box sends instead of closing, if not zero.
    ///
    /// Turbo Vision's `[■]` sent `cmClose`, and a program could refuse it -
    /// which is how "save changes?" got asked. With zero here the box just
    /// closes the window; with a command the program hears about it and
    /// decides. An editor with unsaved text wants the second.
    pub close_cmd: u16,
    /// Set while zoomed; holds the rectangle to restore.
    pub(crate) unzoomed: Option<Rect>,
    /// What this window adds to the status line while it is the active
    /// one: its own keys, bound and shown, and gone when it is not.
    /// Turbo Vision changed the status line by help context; a window
    /// that carries its keys with it is the same idea with less
    /// machinery.
    pub status: Vec<StatusItem>,
    /// What this window adds to the menu bar while it is active - see
    /// `menu::merge_items` for where an item lands.
    pub menu: Vec<MenuItem>,
}

impl Window {
    pub fn new(title: impl Into<String>) -> Self {
        Window {
            title: title.into(),
            closable: true,
            zoomable: true,
            movable: true,
            resizable: true,
            shadow: true,
            modal: false,
            number: None,
            palette: crate::palette::WinPalette::Blue,
            footer: String::new(),
            min_w: 0,
            min_h: 0,
            max_w: 0,
            max_h: 0,
            centred: false,
            close_cmd: 0,
            unzoomed: None,
            status: Vec::new(),
            menu: Vec::new(),
        }
    }

    pub fn is_zoomed(&self) -> bool {
        self.unzoomed.is_some()
    }
}

/// Read-only scrolling text.
///
/// This is deliberately the first thing built and not the editor: it is most
/// of the scrolling, clipping and layout work with none of the editing work,
/// and it is already three things at once — a source viewer, a log pane and
/// the body of the help browser.
pub struct TextView {
    /// Lines of *code page bytes*, not of characters.
    ///
    /// The core never sees a Unicode string: `Cell.ch` is a glyph index, and
    /// text held for display is the same thing. Converting whatever the host
    /// uses — UTF-8 here, whatever DOS hands us there — is the backend's job,
    /// done once on the way in. Doing it per frame, or storing both, is how a
    /// core stops being portable.
    pub lines: Vec<Vec<crate::cell::Glyph>>,
    /// First visible line.
    pub top: i16,
    /// First visible column.
    pub left: i16,
    /// The caret: `x` is the column, `y` the line.
    pub cur: crate::geom::Point,
    /// Where a selection began, if one is being made. The selection is
    /// whatever lies between this and the caret — no start/end pair to keep
    /// in order, and dragging backwards works without a special case.
    pub anchor: Option<crate::geom::Point>,
    /// A viewer rather than an editor. The same view does both jobs; the log
    /// pane and the help browser are this with one flag set.
    pub readonly: bool,
    /// Drawn as a box of its own rather than as part of whatever is behind it.
    /// A memo in a dialog wants this; a document filling a window does not,
    /// because there it *is* the window.
    pub boxed: bool,
    pub focused: bool,
    /// Something has been typed since this was loaded or saved.
    ///
    /// Set in one place — `splice`, the single point every change goes
    /// through — so it cannot drift out of step with the text. Cleared by
    /// whoever saves, because only they know it happened.
    pub modified: bool,
    /// Draw a bar across the caret line. Off by default: Borland's editors
    /// never did this, and a full-width highlight is the single thing that
    /// makes a screen stop looking like Turbo Vision.
    pub highlight_line: bool,
    /// Long lines folded at the view's right edge - at a space when the row
    /// has one, in the middle of the word when it has none. Only the picture
    /// folds: the lines, the caret's line and column, and the saved file are
    /// what they were, and there is no sideways scrolling while it is on.
    pub wrap: bool,
    /// Which folded row of line `top` the view starts on. Always 0 without
    /// `wrap`.
    pub top_row: i16,
    /// How wide the view was last laid out: where `wrap` folds. Zero until
    /// the first layout, and folding waits for it.
    pub width: i16,
    /// What this editor offers on the menu bar and the status line while
    /// its window is active: the bits of `edit::offer`. Zero, the default,
    /// offers nothing and the bars are the program's alone.
    pub offers: u8,
    /// This editor's own arrangement of keys, or the desktop's when `None`.
    pub keymap: Option<crate::keymap::Keymap>,
    /// The hex view standing in for this text while it is shown.
    pub(crate) hex: Option<crate::ui::ViewId>,

    pub(crate) undo_stack: Vec<crate::edit::Edit>,
    pub(crate) redo_stack: Vec<crate::edit::Edit>,
}

impl TextView {
    pub fn new(lines: Vec<Vec<crate::cell::Glyph>>) -> Self {
        TextView {
            lines,
            top: 0,
            left: 0,
            cur: crate::geom::Point::new(0, 0),
            anchor: None,
            readonly: false,
            boxed: false,
            focused: false,
            modified: false,
            highlight_line: false,
            wrap: false,
            top_row: 0,
            width: 0,
            offers: 0,
            keymap: None,
            hex: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }

    /// Convenience for ASCII-only text. Anything else must be encoded to the
    /// active code page by the caller first.
    pub fn from_ascii(text: &str) -> Self {
        TextView::new(
            text.lines()
                .map(|l| crate::cell::glyphs(l))
                .collect(),
        )
    }

    pub fn line_count(&self) -> i16 {
        self.lines.len().min(i16::MAX as usize) as i16
    }

    pub fn longest(&self) -> i16 {
        self.lines
            .iter()
            .map(|l| l.len())
            .max()
            .unwrap_or(0)
            .min(i16::MAX as usize) as i16
    }
}

/// What a view is. An enum, not a trait object and not a generic: every
/// generic instantiation is another copy of the code, and on the platform this
/// has to reach one day the whole program lives in 64K.
pub enum Kind {
    Desktop(Desktop),
    Window(Window),
    Text(TextView),
    Html(crate::html::Html),
    MenuBar(crate::menu::MenuBar),
    MenuBox(crate::menu::MenuBox),
    Files(crate::files::FileList),
    Input(crate::input::InputLine),
    Buttons(crate::button::ButtonRow),
    Hex(crate::hex::HexView),
    Cluster(crate::controls::Cluster),
    List(crate::controls::ListBox),
    Static(crate::controls::StaticText),
    Tree(crate::tree::TreeView),
    /// The bottom row: keys and what they do. One per desktop, like the bar.
    Status(crate::status::StatusLine),
    Label(crate::controls::Label),
    Progress(crate::controls::Progress),
    /// Cells the program drew itself; shown as they are.
    Canvas(crate::controls::Canvas),
}

/// Where a child sits inside its parent.
///
/// This is the whole layout system so far, and it is enough for a dialog: one
/// view fills what is left and the rest are strips against an edge. Turbo
/// Vision had nothing like it - every dialog worked out its own rectangles by
/// hand - which is precisely why resizing one was not something you did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dock {
    Fill,
    /// Fills what is left, but leaves that many rows free above itself:
    /// a panel that wants a line of air under the title bar.
    FillFrom(i16),
    Bottom(i16),
    Top(i16),
    /// A box in the bottom-right corner that takes no room from anyone.
    ///
    /// The strips above take a whole row each, which is right for a toolbar
    /// and wrong for a pair of buttons: the buttons want to sit beside the
    /// information pane, not below it. This one is positioned rather than
    /// subtracted, and whatever it covers was drawn first.
    BottomRight(i16, i16),
    /// Left exactly where it was put, relative to the parent's client area.
    ///
    /// The escape hatch, and the one every Turbo Vision dialog used for
    /// everything: its parts had their rectangles worked out by hand. It is
    /// honest for a dialog whose contents are known and fixed, and it is the
    /// wrong tool the moment anything has to grow.
    Manual,
}

impl Kind {
    /// How far the children's coordinate origin sits inside this view's own
    /// rectangle. A window's frame takes one cell on every side; everything
    /// else has no frame.
    pub fn client_inset(&self) -> i16 {
        match self {
            Kind::Window(_) => 1,
            _ => 0,
        }
    }

    /// What goes in front of what.
    ///
    /// Windows shuffle among themselves as they are activated; the bar and the
    /// status line sit above all of them and never move; an open menu panel is
    /// above even those. Without this, activating a window would put it over
    /// the menu bar, which is both wrong and very confusing to look at.
    pub fn layer(&self) -> u8 {
        match self {
            Kind::MenuBox(_) => 3,
            // A modal dialog goes above the menu bar as well. While one is up
            // the menu is not a thing you may reach, and a bar drawn over the
            // dialog would say the opposite.
            Kind::Window(w) if w.modal => 2,
            Kind::MenuBar(_) | Kind::Status(_) => 1,
            _ => 0,
        }
    }
}
