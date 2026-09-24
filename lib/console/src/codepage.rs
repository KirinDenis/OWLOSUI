//! Code pages: the one place a byte becomes a character.
//!
//! The core never sees Unicode. A cell holds a glyph index, a title is a
//! string of glyph indices, an editor line is a `Vec<u8>` - the screen of a
//! DOS machine, where a byte in video memory *is* the picture the card draws
//! for it. Which picture depends on the code page the card was loaded with,
//! and that is the whole of what this module knows: 437 (the IBM PC's own),
//! 866 (Cyrillic, with the same box-drawing at the same places), and room
//! for more.
//!
//! Text crosses the wire as UTF-8 and is turned into bytes here on the way
//! in and back on the way out. A character the page has no glyph for becomes
//! `?` - not a defect: 256 glyphs cannot hold Unicode, and pretending
//! otherwise is how frames end up with holes in them. What *can* be done is
//! to know beforehand (`fits`), which is how an editor refuses to save a
//! file it would ruin.
//!
//! Inside the core a `String` is a byte string: every `char` is in
//! `0..=255` and is the glyph index itself. `Buffer::text` writes such a
//! char as that byte. So a title in code page 866 is encoded here to bytes
//! and carried in a `String` as chars `U+0000..U+00FF`; nothing in the core
//! has to know, and `chars().count()` still counts cells.

use std::collections::HashMap;
use std::sync::OnceLock;

pub struct CodePage {
    pub number: u16,
    pub table: [char; 256],
    reverse: HashMap<char, u8>,
}

impl CodePage {
    fn new(number: u16, table: [char; 256]) -> Self {
        let mut reverse = HashMap::with_capacity(256);
        // Skip 0x00..0x20: those positions hold pictures (☺, ♦, ↑), and
        // matching them would turn a stray arrow in a document into a
        // control code. Later entries win over earlier ones, so a glyph
        // that appears twice maps to the higher position - which for `■`
        // and the blanks is the one a program means.
        for (i, &c) in table.iter().enumerate().skip(0x20) {
            reverse.insert(c, i as u8);
        }
        CodePage { number, table, reverse }
    }

    /// The pages this build knows. `None` for a number it does not.
    pub fn by_number(n: u16) -> Option<CodePage> {
        match n {
            437 => Some(CodePage::new(437, crate::cp437::TABLE)),
            866 => Some(CodePage::new(866, cp866_table())),
            _ => None,
        }
    }

    pub fn to_char(&self, b: u8) -> char {
        self.table[b as usize]
    }

    /// Unicode to a glyph index, the direction that can fail.
    pub fn from_char(&self, c: char) -> u8 {
        if (c as u32) < 128 {
            return c as u8;
        }
        self.reverse.get(&c).copied().unwrap_or(b'?')
    }

    /// Whether every character of a text has a glyph on this page.
    pub fn fits(&self, s: &str) -> bool {
        s.chars()
            .all(|c| (c as u32) < 128 || c == '\n' || self.reverse.contains_key(&c))
    }

    pub fn encode(&self, s: &str) -> Vec<u8> {
        s.chars().map(|c| self.from_char(c)).collect()
    }

    pub fn decode(&self, b: &[u8]) -> String {
        b.iter().map(|&x| self.to_char(x)).collect()
    }

    /// A text as the core carries it: each character replaced by its glyph
    /// index, held as a char `U+0000..U+00FF`. See the module note.
    pub fn to_core(&self, s: &str) -> String {
        s.chars().map(|c| self.from_char(c) as char).collect()
    }

    /// The other way: a core string of glyph indices back to text.
    pub fn from_core(&self, s: &str) -> String {
        s.chars()
            .map(|c| if (c as u32) < 256 { self.to_char(c as u8) } else { '?' })
            .collect()
    }
}

/// The code page this process shows, from `OWLOSUI_CODEPAGE`; 437 unless
/// told otherwise. For the terminal backend, which draws for one console.
pub fn current() -> &'static CodePage {
    static CP: OnceLock<CodePage> = OnceLock::new();
    CP.get_or_init(|| {
        std::env::var("OWLOSUI_CODEPAGE")
            .ok()
            .and_then(|s| s.trim().parse::<u16>().ok())
            .and_then(CodePage::by_number)
            .unwrap_or_else(|| CodePage::by_number(437).unwrap())
    })
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
