//! The font: the one place a glyph index becomes a character.
//!
//! The core never sees Unicode. A cell holds a glyph index, a title is a
//! string of glyph indices, an editor line is a `Vec<Glyph>` - the screen of
//! a DOS machine, where a byte in video memory *is* the picture the card
//! draws for it. Which picture depends on the font the card was loaded
//! with, and that is what this module knows.
//!
//! A `Font` starts as a code page - 437 (the IBM PC's own) or 866
//! (Cyrillic, with the same box-drawing at the same places) - and is one of
//! two kinds:
//!
//!  * **fixed**, 256 glyphs and no more: a DOS card, or the terminal
//!    backend, which draws for one console. A character with no glyph
//!    becomes `?`, and `fits` says so beforehand;
//!  * **growing**: the same 256 to begin with, and every new character that
//!    arrives gets the next free index. That is how a Windows or browser
//!    client shows Russian and Slovak on one screen: the core still holds
//!    numbers, the client asks for the table (`GET_GLYPHS`) and draws.
//!
//! Text crosses the wire as UTF-8 and is turned into glyph indices here on
//! the way in and back on the way out. Inside the core a `String` is a
//! string of glyph indices: every `char` is in `0..=Glyph::MAX` and is the
//! index itself. `Buffer::text` writes such a char as that glyph.

use alloc::collections::BTreeMap;
#[allow(unused_imports)]
use alloc::{string::String, vec::Vec};

use owlosui_core::{Glyph, GLYPH_MAX};

pub use owlosui_core::cell::GLYPH_MAX as MAX_INDEX;

pub struct Font {
    /// The code page it started from.
    pub number: u16,
    /// Glyph index to character. The first 256 are the code page.
    pub table: Vec<char>,
    reverse: BTreeMap<char, Glyph>,
    /// Whether new characters may be added, or must become `?`.
    pub growing: bool,
}

impl Font {
    fn new(number: u16, page: [char; 256], growing: bool) -> Self {
        let mut reverse = BTreeMap::new();
        // Skip 0x00..0x20: those positions hold pictures (☺, ♦, ↑), and
        // matching them would turn a stray arrow in a document into a
        // control code. Later entries win over earlier ones, so a glyph
        // that appears twice maps to the higher position - which for `■`
        // and the blanks is the one a program means.
        for (i, &c) in page.iter().enumerate().skip(0x20) {
            reverse.insert(c, i as Glyph);
        }
        // A growing font can only grow if a glyph index has room past 255.
        let growing = growing && GLYPH_MAX > 255;
        Font { number, table: page.to_vec(), reverse, growing }
    }

    /// A font that is exactly a code page, 256 glyphs, for a screen that
    /// holds bytes. `None` for a page this build does not know.
    pub fn fixed(number: u16) -> Option<Font> {
        page(number).map(|p| Font::new(number, p, false))
    }

    /// A font seeded with a code page that grows as characters arrive.
    pub fn growing(number: u16) -> Option<Font> {
        page(number).map(|p| Font::new(number, p, true))
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub fn to_char(&self, g: Glyph) -> char {
        self.table.get(g as usize).copied().unwrap_or('?')
    }

    /// A character's glyph index, if it has one.
    pub fn lookup(&self, c: char) -> Option<Glyph> {
        if (c as u32) < 128 {
            return Some(c as u32 as Glyph);
        }
        self.reverse.get(&c).copied()
    }

    /// A character's glyph index: found, or - in a growing font - made.
    /// `?` when there is none and none can be made.
    pub fn from_char(&mut self, c: char) -> Glyph {
        if let Some(g) = self.lookup(c) {
            return g;
        }
        if self.growing && (self.table.len() as u32) <= GLYPH_MAX {
            let g = self.table.len() as Glyph;
            self.table.push(c);
            self.reverse.insert(c, g);
            return g;
        }
        b'?' as Glyph
    }

    /// Whether every character of a text has a glyph, or could be given
    /// one. On a fixed font this is the question "will it show or turn to
    /// `?`"; on a growing one it is almost always yes.
    pub fn fits(&self, s: &str) -> bool {
        s.chars().all(|c| {
            c == '\n' || c == '\r' || c == '\t' || self.lookup(c).is_some() || self.growing
        })
    }

    /// A character for a cell of a canvas, where a picture is a picture.
    /// Text never maps to the positions below 0x20 (a stray arrow in a
    /// document must not become a control code), but a canvas cell is
    /// not text: ☺ asked for there is glyph 1, the face the card draws.
    /// That is how an ASCII table shows every glyph on a fixed font, where
    /// growing past 255 is not possible.
    pub fn cell_glyph(&mut self, c: char) -> Glyph {
        if let Some(g) = self.lookup(c) {
            return g;
        }
        if let Some(i) = self.table.iter().take(0x20).position(|&t| t == c) {
            if i > 0 {
                return i as Glyph;
            }
        }
        self.from_char(c)
    }

    pub fn encode(&mut self, s: &str) -> Vec<Glyph> {
        s.chars().map(|c| self.from_char(c)).collect()
    }

    /// `encode` for a font that may not grow - the terminal's, shared and
    /// fixed: what has no glyph becomes `?`.
    pub fn encode_known(&self, s: &str) -> Vec<Glyph> {
        s.chars().map(|c| self.lookup(c).unwrap_or(b'?' as Glyph)).collect()
    }

    pub fn decode(&self, g: &[Glyph]) -> String {
        g.iter().map(|&x| self.to_char(x)).collect()
    }

    /// A text as the core carries it: each character replaced by its glyph
    /// index, held as a char. See the module note.
    pub fn to_core(&mut self, s: &str) -> String {
        s.chars()
            .map(|c| char::from_u32(self.from_char(c) as u32).unwrap_or('?'))
            .collect()
    }

    /// The other way: a core string of glyph indices back to text.
    pub fn from_core(&self, s: &str) -> String {
        s.chars()
            .map(|c| if (c as u32) <= GLYPH_MAX { self.to_char(c as u32 as Glyph) } else { '?' })
            .collect()
    }
}

/// The font this process draws with, from `OWLOSUI_CODEPAGE`; 437 unless
/// told otherwise. Fixed, because the terminal backend draws for one
/// console with one code page - the DOS case, on a bigger machine.
#[cfg(feature = "std")]
pub fn current() -> &'static Font {
    static CP: std::sync::OnceLock<Font> = std::sync::OnceLock::new();
    CP.get_or_init(|| {
        std::env::var("OWLOSUI_CODEPAGE")
            .ok()
            .and_then(|s| s.trim().parse::<u16>().ok())
            .and_then(Font::fixed)
            .unwrap_or_else(|| Font::fixed(437).unwrap())
    })
}

fn page(number: u16) -> Option<[char; 256]> {
    match number {
        437 => Some(crate::cp437::TABLE),
        866 => Some(cp866_table()),
        _ => None,
    }
}

/// CP866: the IBM PC's Cyrillic. The lower half and the box-drawing block
/// `0xB0..0xDF` are 437's, which is why a Russian program's frames looked
/// the same as an American one's; the letters take the two blocks either
/// side of the boxes, and the top row holds Ё, the Ukrainian and
/// Belarusian letters, and a few signs.
fn cp866_table() -> [char; 256] {
    let mut t = crate::cp437::TABLE;
    for i in 0..0x20u32 {
        t[0x80 + i as usize] = char::from_u32(0x0410 + i).unwrap(); // А..Я
    }
    for i in 0..0x10u32 {
        t[0xA0 + i as usize] = char::from_u32(0x0430 + i).unwrap(); // а..п
        t[0xE0 + i as usize] = char::from_u32(0x0440 + i).unwrap(); // р..я
    }
    let top: [char; 16] = [
        '\u{0401}', '\u{0451}', '\u{0404}', '\u{0454}', '\u{0407}', '\u{0457}', '\u{040E}', '\u{045E}',
        '\u{00B0}', '\u{2219}', '\u{00B7}', '\u{221A}', '\u{2116}', '\u{00A4}', '\u{25A0}', '\u{00A0}',
    ];
    t[0xF0..].copy_from_slice(&top);
    t
}
