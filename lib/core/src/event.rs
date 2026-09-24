//! What comes in.
//!
//! This is the second seam, and the one that gets forgotten: DOS scan codes,
//! browser key events and terminal escape sequences have nothing in common,
//! so the core refuses to see any of them. A backend translates into these
//! types and the core never learns which platform it is on.

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
