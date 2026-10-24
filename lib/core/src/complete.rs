//! Words offered while typing: the first letters of a word, and a short
//! list under the caret of the words it may become.
//!
//! Two kinds of word are offered. The language's own - an assembler's
//! mnemonics, its registers, its directives, from `syntax.ini` - and the
//! program's: its labels and its names, from the text itself, which no
//! list could know. The program's come first the more often they are
//! used, because a label written forty times is the one wanted; then the
//! shorter before the longer, so `mo` offers `mov` before `movsb`.
//!
//! This is what makes code writable on a phone, where every letter is a
//! tap - and it is what any code editor does on a desktop. It is not a
//! grammar: nothing here knows where a mnemonic may stand. Like the
//! colours, it is small enough to run on DOS.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::cell::Glyph;
use crate::syntax::Syntax;

/// How many letters are typed before anything is offered: one letter
/// matches half the language.
pub const MIN_PREFIX: usize = 2;
/// How many words the list shows.
pub const MAX_ITEMS: usize = 8;
/// The program's own words are looked for in this many lines around the
/// caret, so a big file costs no more than a small one.
const SCAN_LINES: usize = 4000;

/// The words `prefix` may become, best first, at most `MAX_ITEMS`. `here`
/// is the line and column where the typed word starts, so the word being
/// typed is not offered to itself.
///
/// In a language that ignores case, its words come in the case being
/// typed: `MO` offers `MOV`, `mo` offers `mov`. The program's own words
/// come as they are written.
pub fn candidates(syn: &Syntax, lines: &[Vec<Glyph>], here: (usize, usize), prefix: &[Glyph]) -> Vec<Vec<Glyph>> {
    if prefix.len() < MIN_PREFIX || prefix.iter().any(|&g| g >= 128) {
        return Vec::new();
    }
    let ic = syn.ignores_case();
    let fold = |g: Glyph| if ic && g < 128 { (g as u8).to_ascii_lowercase() as Glyph } else { g };
    let key = |w: &[Glyph]| w.iter().map(|&g| fold(g)).collect::<Vec<Glyph>>();
    let typed = key(prefix);
    let upper = prefix.iter().any(|&g| (g as u8).is_ascii_alphabetic()) && !prefix.iter().any(|&g| (g as u8).is_ascii_lowercase());

    // word (as shown), how often the program uses it, and whether the
    // language has it - by its folded spelling.
    let mut found: Vec<(Vec<Glyph>, Vec<Glyph>, usize, bool)> = Vec::new();
    let mut add = |shown: Vec<Glyph>, uses: usize, lang: bool| {
        let k = key(&shown);
        if k == typed || !k.starts_with(&typed) {
            return;
        }
        if let Some(f) = found.iter_mut().find(|f| f.1 == k) {
            f.2 += uses;
            f.3 |= lang;
        } else {
            found.push((shown, k, uses, lang));
        }
    };

    let (row, col) = here;
    let from = row.saturating_sub(SCAN_LINES / 2);
    let to = (from + SCAN_LINES).min(lines.len());
    for (r, line) in lines.iter().enumerate().take(to).skip(from) {
        let mut i = 0;
        while i < line.len() {
            if !syn.word_char(line[i]) {
                i += 1;
                continue;
            }
            let start = i;
            while i < line.len() && syn.word_char(line[i]) {
                i += 1;
            }
            // Not the word being typed, and not a number: 0FFh is not a name.
            let w = &line[start..i];
            if (r, start) != (row, col) && w.len() > typed.len() && !(w[0] as u8).is_ascii_digit() {
                add(w.to_vec(), 1, false);
            }
        }
    }
    let lang: Vec<Vec<u8>> = syn.words_from(&typed.iter().map(|&g| g as u8).collect::<Vec<u8>>()).iter().map(|w| w.to_vec()).collect();
    for w in lang {
        let shown = w.iter().map(|&b| (if ic && upper { b.to_ascii_uppercase() } else { b }) as Glyph).collect();
        add(shown, 0, true);
    }
    // A language word the program also uses is shown in the case being
    // typed, not as some line happened to write it.
    for f in &mut found {
        if f.3 && ic {
            f.0 = f.1.iter().map(|&g| if upper { (g as u8).to_ascii_uppercase() as Glyph } else { g }).collect();
        }
    }
    found.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.len().cmp(&b.0.len())).then(a.1.cmp(&b.1)));
    found.into_iter().take(MAX_ITEMS).map(|f| f.0).collect()
}

/// The list while it is up: over which text, where the typed word starts,
/// what it offers, which one is lit, and whether a hand chose it - Up or
/// Down - which is when Enter takes it instead of starting a new line.
#[derive(Clone, Debug)]
pub struct Completion {
    pub row: usize,
    pub start: usize,
    pub items: Vec<Vec<Glyph>>,
    pub current: usize,
    pub chosen: bool,
}
