//! A screen cell, and the sixteen colours it may take.
//!
//! The layout is the IBM PC text attribute byte, unchanged: low nibble is the
//! foreground, high nibble the background. On DOS a row of these can be handed
//! to the video hardware with no conversion at all; every other backend
//! translates.
//!
//! We treat the top bit of the background nibble as *bright background*, not
//! as blink. Blinking is a property we can add later as a separate flag if a
//! backend can honour it; losing eight background colours to it cannot be
//! undone.

/// The sixteen colours. The numbering is the hardware's, not ours.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Reconstruct a colour from a nibble. Values above 15 are impossible by
    /// construction, but we mask rather than panic: a bad attribute byte
    /// should draw something wrong, never bring the program down.
    pub const fn from_index(i: u8) -> Color {
        match i & 0x0F {
            0 => Color::Black,
            1 => Color::Blue,
            2 => Color::Green,
            3 => Color::Cyan,
            4 => Color::Red,
            5 => Color::Magenta,
            6 => Color::Brown,
            7 => Color::LightGray,
            8 => Color::DarkGray,
            9 => Color::LightBlue,
            10 => Color::LightGreen,
            11 => Color::LightCyan,
            12 => Color::LightRed,
            13 => Color::LightMagenta,
            14 => Color::Yellow,
            _ => Color::White,
        }
    }
}

/// Build an attribute byte from a foreground and a background colour.
pub const fn attr(fg: Color, bg: Color) -> u8 {
    ((bg as u8) << 4) | (fg as u8)
}

pub const fn attr_fg(a: u8) -> Color {
    Color::from_index(a & 0x0F)
}

pub const fn attr_bg(a: u8) -> Color {
    Color::from_index(a >> 4)
}

/// One character cell: a code page byte plus its attribute.
///
/// `ch` is an index into the current font, not a Unicode scalar. The default
/// font is CP437; swapping the font swaps the code page with it, which is how
/// CP866 (Cyrillic) or a Spectrum character set slot in without the core
/// knowing anything about them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: u8,
    pub attr: u8,
}

impl Cell {
    pub const fn new(ch: u8, attr: u8) -> Self {
        Cell { ch, attr }
    }
}

/// Glyph indices we reach for by name instead of by magic number.
pub mod glyph {
    pub const SPACE: u8 = 0x20;

    // Shading, used for the desktop background and scrollbar tracks.
    pub const LIGHT_SHADE: u8 = 0xB0; // ░
    pub const MEDIUM_SHADE: u8 = 0xB1; // ▒
    pub const DARK_SHADE: u8 = 0xB2; // ▓
    pub const FULL_BLOCK: u8 = 0xDB; // █
    /// A small filled square that leaves a margin inside its cell — the
    /// scrollbar marker and the close box are both this.
    pub const SQUARE: u8 = 0xFE; // ■

    // Single line box drawing.
    pub const SL_H: u8 = 0xC4; // ─
    pub const SL_V: u8 = 0xB3; // │
    pub const SL_TL: u8 = 0xDA; // ┌
    pub const SL_TR: u8 = 0xBF; // ┐
    pub const SL_BL: u8 = 0xC0; // └
    pub const SL_BR: u8 = 0xD9; // ┘

    // Double line box drawing, the frame of an active window.
    pub const DL_H: u8 = 0xCD; // ═
    pub const DL_V: u8 = 0xBA; // ║
    pub const DL_TL: u8 = 0xC9; // ╔
    pub const DL_TR: u8 = 0xBB; // ╗
    pub const DL_BL: u8 = 0xC8; // ╚
    pub const DL_BR: u8 = 0xBC; // ╝

    // Arrows, used on scrollbar ends.
    pub const ARROW_UP: u8 = 0x1E; // ▲
    pub const ARROW_DOWN: u8 = 0x1F; // ▼
    pub const ARROW_LEFT: u8 = 0x11; // ◄
    pub const ARROW_RIGHT: u8 = 0x10; // ►
}
