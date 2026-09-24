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

/// A glyph index: which picture in the font a cell shows.
///
/// On DOS (`feature = "dos"`) it is a byte, because that is what video
/// memory holds, and the font is the 256 glyphs loaded into the card.
/// Everywhere else it is a `u16` into a font that starts as a code page
/// and grows as characters arrive, so a Windows or browser client can show
/// Russian and Slovak names on one screen without the core knowing either.
/// The first 256 entries are the same in both: the frames, the shades and
/// the arrows are drawn by number, and those numbers do not move.
#[cfg(feature = "dos")]
pub type Glyph = u8;
#[cfg(not(feature = "dos"))]
pub type Glyph = u16;

/// The largest glyph index, as a `u32` for comparing against a `char`.
pub const GLYPH_MAX: u32 = Glyph::MAX as u32;

/// A string as the core carries it, turned into glyph indices. Every char
/// of a core string *is* a glyph index (the backend put it there), so this
/// is a widening and never a conversion; a char beyond the font becomes `?`.
pub fn glyphs(s: &str) -> Vec<Glyph> {
    s.chars().map(glyph_of).collect()
}

/// One char of a core string as a glyph index.
pub fn glyph_of(c: char) -> Glyph {
    if (c as u32) <= GLYPH_MAX {
        c as u32 as Glyph
    } else {
        b'?' as Glyph
    }
}

/// One character cell: a glyph index plus its attribute.
///
/// `ch` is an index into the current font, not a Unicode scalar. The default
/// font is CP437; swapping the font swaps the code page with it, which is how
/// CP866 (Cyrillic) or a Spectrum character set slot in without the core
/// knowing anything about them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: Glyph,
    pub attr: u8,
}

impl Cell {
    pub const fn new(ch: Glyph, attr: u8) -> Self {
        Cell { ch, attr }
    }

    /// The glyph as the char that stands for it in a core string. For
    /// tests and dumps; a backend draws through its font instead.
    pub fn to_char(&self) -> char {
        char::from_u32(self.ch as u32).unwrap_or('?')
    }
}

/// Glyph indices we reach for by name instead of by magic number.
pub mod glyph {
    use super::Glyph;

    pub const SPACE: Glyph = 0x20;

    // Shading, used for the desktop background and scrollbar tracks.
    pub const LIGHT_SHADE: Glyph = 0xB0; // ░
    pub const MEDIUM_SHADE: Glyph = 0xB1; // ▒
    pub const DARK_SHADE: Glyph = 0xB2; // ▓
    pub const FULL_BLOCK: Glyph = 0xDB; // █
    /// A small filled square that leaves a margin inside its cell — the
    /// scrollbar marker and the close box are both this.
    pub const SQUARE: Glyph = 0xFE; // ■

    // Single line box drawing.
    pub const SL_H: Glyph = 0xC4; // ─
    pub const SL_V: Glyph = 0xB3; // │
    pub const SL_TL: Glyph = 0xDA; // ┌
    pub const SL_TR: Glyph = 0xBF; // ┐
    pub const SL_BL: Glyph = 0xC0; // └
    pub const SL_BR: Glyph = 0xD9; // ┘

    // Double line box drawing, the frame of an active window.
    pub const DL_H: Glyph = 0xCD; // ═
    pub const DL_V: Glyph = 0xBA; // ║
    pub const DL_TL: Glyph = 0xC9; // ╔
    pub const DL_TR: Glyph = 0xBB; // ╗
    pub const DL_BL: Glyph = 0xC8; // ╚
    pub const DL_BR: Glyph = 0xBC; // ╝

    // Arrows, used on scrollbar ends.
    pub const ARROW_UP: Glyph = 0x1E; // ▲
    pub const ARROW_DOWN: Glyph = 0x1F; // ▼
    pub const ARROW_LEFT: Glyph = 0x11; // ◄
    pub const ARROW_RIGHT: Glyph = 0x10; // ►
}
