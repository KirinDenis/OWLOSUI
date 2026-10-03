//! CP437 to Unicode.
//!
//! The core stores a glyph *index*, not a character. On DOS that index goes
//! straight to the video card. In a terminal we have to say which Unicode
//! character the font should draw instead, and that is what this table is.
//!
//! Swap this table and the same core renders CP866 Cyrillic or anything else;
//! nothing above it changes.

#[allow(unused_imports)]
use alloc::vec::Vec;

pub const TABLE: [char; 256] = [
    // 0x00
    ' ', '☺', '☻', '♥', '♦', '♣', '♠', '•', '◘', '○', '◙', '♂', '♀', '♪', '♫', '☼',
    // 0x10
    '►', '◄', '↕', '‼', '¶', '§', '▬', '↨', '↑', '↓', '→', '←', '∟', '↔', '▲', '▼',
    // 0x20
    ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    // 0x30
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    // 0x40
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    // 0x50
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
    // 0x60
    '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    // 0x70
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '⌂',
    // 0x80
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    // 0x90
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
    // 0xA0
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    // 0xC0
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    // 0xD0
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    // 0xE0
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
    // 0xF0
    // 0xFF is the no-break space, not a second space: a byte read as a
    // character and written back must come back the same byte.
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

#[inline]
pub fn to_char(b: u8) -> char {
    TABLE[b as usize]
}

/// Unicode to CP437, the direction that can fail.
///
/// Anything the code page has no glyph for becomes `?`. That is not a defect
/// to be fixed later: a font of 256 glyphs cannot represent Unicode, and
/// pretending otherwise is how a text UI ends up with holes in its frames.
pub fn from_char(c: char) -> u8 {
    if (c as u32) < 128 {
        return c as u8;
    }
    // Skip 0x00..0x20: those positions hold pictures (☺, ♦, ↑), and matching
    // them here would turn a stray arrow in a document into a control code.
    match TABLE[0x20..].iter().position(|&t| t == c) {
        Some(i) => (i + 0x20) as u8,
        None => b'?',
    }
}

pub fn encode(s: &str) -> Vec<u8> {
    s.chars().map(from_char).collect()
}
