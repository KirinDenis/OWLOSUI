//! What comes in.
//!
//! This is the second seam, and the one that gets forgotten: DOS scan codes,
//! browser key events and terminal escape sequences have nothing in common,
//! so the core refuses to see any of them. A backend translates into these
//! types and the core never learns which platform it is on.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub const NONE: Mods = Mods {
        ctrl: false,
        alt: false,
        shift: false,
    };

    pub const fn ctrl() -> Mods {
        Mods {
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    pub const fn alt() -> Mods {
        Mods {
            ctrl: false,
            alt: true,
            shift: false,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Key {
    pub code: KeyCode,
    pub mods: Mods,
}

impl Key {
    pub const fn new(code: KeyCode, mods: Mods) -> Self {
        Key { code, mods }
    }

    pub fn plain(&self, code: KeyCode) -> bool {
        self.code == code && self.mods.is_none()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MouseKind {
    Down(Button),
    Up(Button),
    /// The second press of a double click, in place of its `Down`. The
    /// core has no clock, so the backend decides what "double" is - the
    /// console says so itself on Windows, and a terminal times it - and
    /// the core only says what it means: a click, and then the thing's
    /// second meaning. A title bar zooms; a file is chosen as Enter would
    /// choose it; a list presses the default button.
    Double(Button),
    Drag,
    Move,
    ScrollUp,
    ScrollDown,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mouse {
    /// Absolute screen cell the pointer is over.
    pub x: i16,
    pub y: i16,
    pub kind: MouseKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    Key(Key),
    Mouse(Mouse),
    Resize(i16, i16),
    /// A timer beat. Blinking and timeouts hang off this rather than off any
    /// notion of wall-clock time inside the core.
    Tick,
}
