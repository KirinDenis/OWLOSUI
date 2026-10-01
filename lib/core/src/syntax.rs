//! Syntax colours.
//!
//! A language is a few lines of an INI file - its comments, its strings, its
//! keywords - and not a grammar. That is all the classic DOS editors knew, and it
//! is enough for the eye: what matters when reading code on a 16-colour
//! screen is which words are the language's, what is said to a person
//! rather than to the machine, and where a string ends. It also fits: no
//! regular expressions, no parser, a small table per language, so the same
//! code colours a file on DOS.
//!
//! The languages that come with the core are in `syntax.ini`, beside this
//! file, which also says what every line of the format means. A program can
//! add its own with `Ui::add_syntax` - the same format, in a string.
//!
//! Colouring goes a line at a time and carries one thing from a line to the
//! next: whether it ended inside a block comment, a block directive or a
//! string that may go on. That is a `u16`, kept for each line so that
//! typing on line 900 recolours from line 900 down, and only as far as the
//! window shows.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::cell::Glyph;

/// What a character is, for its colour. `Text` keeps the editor's own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Text,
    Keyword,
    Type,
    Comment,
    String,
    Number,
    Directive,
}

/// The languages the core comes with.
pub const BUILTIN: &str = include_str!("syntax.ini");

/// One language.
#[derive(Clone, Debug, Default)]
pub struct Syntax {
    pub name: String,
    /// Extensions, upper case, without the dot.
    pub files: Vec<String>,
    ignore_case: bool,
    /// Characters that belong to a word besides letters, digits and `_`.
    word: Vec<u8>,
    comments: Vec<Vec<u8>>,
    blocks: Vec<(Vec<u8>, Vec<u8>)>,
    block_directives: Vec<(Vec<u8>, Vec<u8>)>,
    directive_lines: Vec<Vec<u8>>,
    /// Quote, escape, and whether the string may go on past the line.
    strings: Vec<(u8, Option<u8>, bool)>,
    chars: Vec<(u8, Option<u8>)>,
    number_prefixes: Vec<u8>,
    keywords: Vec<Vec<u8>>,
    types: Vec<Vec<u8>>,
    directives: Vec<Vec<u8>>,
}

// A line's starting state: 0 is nothing carried; otherwise the kind in the
// high byte and which one of that kind in the low.
const IN_COMMENT: u16 = 0x100;
const IN_DIRECTIVE: u16 = 0x200;
const IN_STRING: u16 = 0x300;

/// Every language in an INI text, in the order written.
///
/// Lenient on purpose: a line it does not understand is passed over, so a
/// file written for a later version still gives what it can.
pub fn parse(ini: &str) -> Vec<Syntax> {
    let mut out: Vec<Syntax> = Vec::new();
    for raw in ini.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            out.push(Syntax { name: name.trim().to_string(), ..Syntax::default() });
            continue;
        }
        let (Some(s), Some((key, value))) = (out.last_mut(), line.split_once('=')) else { continue };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        let t: Vec<&str> = value.split_whitespace().collect();
        let byte = |s: &str| s.bytes().next().filter(|_| s.len() == 1);
        match key.as_str() {
            "files" => s.files.extend(t.iter().map(|x| x.trim_start_matches('.').to_ascii_uppercase())),
            "case" => s.ignore_case = value.eq_ignore_ascii_case("ignore"),
            "word" => s.word.extend(value.bytes().filter(|b| !b.is_ascii_whitespace())),
            "comment" => s.comments.extend(t.iter().map(|x| x.as_bytes().to_vec())),
            "block" | "block directive" if t.len() >= 2 => {
                let pair = (t[0].as_bytes().to_vec(), t[1].as_bytes().to_vec());
                if key == "block" {
                    s.blocks.push(pair);
                } else {
                    s.block_directives.push(pair);
                }
            }
            "directive" => s.directive_lines.extend(t.iter().map(|x| x.as_bytes().to_vec())),
            "string" | "char" => {
                let Some(q) = t.first().and_then(|x| byte(x)) else { continue };
                let esc = t.iter().skip(1).find_map(|x| byte(x));
                if key == "string" {
                    let multi = t.iter().any(|x| x.eq_ignore_ascii_case("multiline"));
                    s.strings.push((q, esc, multi));
                } else {
                    s.chars.push((q, esc));
                }
            }
            "number" => s.number_prefixes.extend(t.iter().filter_map(|x| byte(x))),
            "keywords" | "types" | "directives" => {
                let ic = s.ignore_case;
                let words = t.iter().map(|w| if ic { w.to_ascii_lowercase() } else { w.to_string() }.into_bytes());
                match key.as_str() {
                    "keywords" => s.keywords.extend(words),
                    "types" => s.types.extend(words),
                    _ => s.directives.extend(words),
                }
            }
            _ => {}
        }
    }
    for s in &mut out {
        for list in [&mut s.keywords, &mut s.types, &mut s.directives] {
            list.sort();
            list.dedup();
        }
    }
    out
}

impl Syntax {
    /// Whether a language is meant by `what`: its name, an extension, or a
    /// file name, whose extension then decides.
    pub fn answers_to(&self, what: &str) -> bool {
        let what = what.trim();
        if what.is_empty() {
            return false;
        }
        if self.name.eq_ignore_ascii_case(what) {
            return true;
        }
        let ext = what.rsplit_once('.').map_or(what, |(_, e)| e);
        self.files.iter().any(|f| f.eq_ignore_ascii_case(ext))
    }

    fn is_word(&self, g: Glyph) -> bool {
        // Beyond ASCII every glyph is a letter, as the editor's word moves
        // have it: a Cyrillic identifier is still one word.
        g >= 128 || (g as u8).is_ascii_alphanumeric() || g == b'_' as Glyph || self.word.contains(&(g as u8))
    }

    fn same(&self, g: Glyph, b: u8) -> bool {
        if g >= 128 {
            return false;
        }
        let c = g as u8;
        c == b || (self.ignore_case && c.eq_ignore_ascii_case(&b))
    }

    /// Whether `m` is at `i`. A marker made of letters - `rem` - must be a
    /// whole word, or "remember" would start a comment.
    fn at(&self, line: &[Glyph], i: usize, m: &[u8]) -> bool {
        if m.is_empty() || i + m.len() > line.len() {
            return false;
        }
        if !m.iter().enumerate().all(|(k, &b)| self.same(line[i + k], b)) {
            return false;
        }
        if m.iter().all(|b| b.is_ascii_alphabetic()) {
            let before = i > 0 && self.is_word(line[i - 1]);
            let after = line.get(i + m.len()).is_some_and(|&g| self.is_word(g));
            return !before && !after;
        }
        true
    }

    fn find(&self, line: &[Glyph], from: usize, m: &[u8]) -> Option<usize> {
        (from..line.len()).find(|&i| self.at(line, i, m))
    }

    /// Where a string opened by quote number `k` ends, past its closing
    /// quote, starting to look at `from`.
    fn string_end(&self, line: &[Glyph], from: usize, k: usize) -> Option<usize> {
        let (q, esc, _) = self.strings[k];
        let mut j = from;
        while j < line.len() {
            if esc.is_some_and(|e| self.same(line[j], e)) && Some(q) != esc {
                j += 2;
            } else if self.same(line[j], q) {
                return Some(j + 1);
            } else {
                j += 1;
            }
        }
        None
    }

    fn role_of_word(&self, w: &[Glyph]) -> Role {
        if w.iter().any(|&g| g >= 128) {
            return Role::Text;
        }
        let bytes: Vec<u8> = w
            .iter()
            .map(|&g| if self.ignore_case { (g as u8).to_ascii_lowercase() } else { g as u8 })
            .collect();
        if self.keywords.binary_search(&bytes).is_ok() {
            Role::Keyword
        } else if self.types.binary_search(&bytes).is_ok() {
            Role::Type
        } else if self.directives.binary_search(&bytes).is_ok() {
            Role::Directive
        } else {
            Role::Text
        }
    }

    /// Colour one line that starts in `state`, and say what the next one
    /// starts in. With `out`, the role of every character goes there.
    pub fn paint(&self, line: &[Glyph], state: u16, mut out: Option<&mut Vec<Role>>) -> u16 {
        let n = line.len();
        if let Some(o) = out.as_deref_mut() {
            o.clear();
            o.resize(n, Role::Text);
        }
        let mut set = |a: usize, b: usize, r: Role| {
            if let Some(o) = out.as_deref_mut() {
                for x in &mut o[a.min(n)..b.min(n)] {
                    *x = r;
                }
            }
        };
        let mut i = 0;

        // What the line before left open.
        let k = (state & 0xFF) as usize;
        match state & 0xFF00 {
            IN_COMMENT | IN_DIRECTIVE => {
                let (list, role) = if state & 0xFF00 == IN_COMMENT {
                    (&self.blocks, Role::Comment)
                } else {
                    (&self.block_directives, Role::Directive)
                };
                let Some((_, close)) = list.get(k) else { return 0 };
                match self.find(line, 0, close) {
                    Some(j) => {
                        set(0, j + close.len(), role);
                        i = j + close.len();
                    }
                    None => {
                        set(0, n, role);
                        return state;
                    }
                }
            }
            IN_STRING if k < self.strings.len() => match self.string_end(line, 0, k) {
                Some(e) => {
                    set(0, e, Role::String);
                    i = e;
                }
                None => {
                    set(0, n, Role::String);
                    return state;
                }
            },
            _ => {}
        }

        let first = line.iter().position(|&g| g != b' ' as Glyph && g != b'\t' as Glyph).unwrap_or(n);
        'scan: while i < n {
            let g = line[i];
            if self.comments.iter().any(|m| self.at(line, i, m)) {
                set(i, n, Role::Comment);
                return 0;
            }
            if i == first && self.directive_lines.iter().any(|m| self.at(line, i, m)) {
                set(i, n, Role::Directive);
                return 0;
            }
            for (list, role, carry) in [
                (&self.block_directives, Role::Directive, IN_DIRECTIVE),
                (&self.blocks, Role::Comment, IN_COMMENT),
            ] {
                for (k, (open, close)) in list.iter().enumerate() {
                    if self.at(line, i, open) {
                        match self.find(line, i + open.len(), close) {
                            Some(j) => {
                                set(i, j + close.len(), role);
                                i = j + close.len();
                                continue 'scan;
                            }
                            None => {
                                set(i, n, role);
                                return carry | k as u16;
                            }
                        }
                    }
                }
            }
            if let Some(k) = self.strings.iter().position(|&(q, _, _)| self.same(g, q)) {
                match self.string_end(line, i + 1, k) {
                    Some(e) => {
                        set(i, e, Role::String);
                        i = e;
                        continue;
                    }
                    None => {
                        set(i, n, Role::String);
                        return if self.strings[k].2 { IN_STRING | k as u16 } else { 0 };
                    }
                }
            }
            if let Some(&(q, esc)) = self.chars.iter().find(|&&(q, _)| self.same(g, q)) {
                // A character: one, or an escape and what follows it, and
                // the closing quote close by. A quote with no partner near
                // it - Rust's 'a - is not the start of anything.
                let end = if esc.is_some_and(|e| line.get(i + 1).is_some_and(|&c| self.same(c, e))) {
                    (i + 3..n.min(i + 12)).find(|&j| self.same(line[j], q)).map(|j| j + 1)
                } else if line.get(i + 2).is_some_and(|&c| self.same(c, q)) {
                    Some(i + 3)
                } else {
                    None
                };
                if let Some(e) = end {
                    set(i, e, Role::String);
                    i = e;
                    continue;
                }
            }
            let digit = |c: Glyph| c < 128 && (c as u8).is_ascii_digit();
            let hex = |c: Glyph| c < 128 && (c as u8).is_ascii_hexdigit();
            let prefixed = self.number_prefixes.contains(&(g as u8)) && g < 128 && line.get(i + 1).is_some_and(|&c| hex(c));
            if digit(g) || prefixed {
                let mut j = i + 1;
                while j < n && (self.is_word(line[j]) || (line[j] == b'.' as Glyph && line.get(j + 1).is_some_and(|&c| digit(c)))) {
                    j += 1;
                }
                set(i, j, Role::Number);
                i = j;
                continue;
            }
            if self.is_word(g) {
                let mut j = i + 1;
                while j < n && self.is_word(line[j]) {
                    j += 1;
                }
                let r = self.role_of_word(&line[i..j]);
                if r != Role::Text {
                    set(i, j, r);
                }
                i = j;
                continue;
            }
            i += 1;
        }
        0
    }
}

/// Bring the remembered starting states of a text's lines up to line
/// `upto`, colouring from the first line whose state is not known.
pub(crate) fn update_states(t: &mut crate::views::TextView, syn: &Syntax, upto: usize) {
    if t.states_valid == 0 {
        t.states.clear();
        t.states.push(0);
        t.states_valid = 1;
    }
    t.states.truncate(t.states_valid);
    while t.states.len() <= upto {
        let k = t.states.len() - 1;
        let Some(line) = t.lines.get(k) else { break };
        let next = syn.paint(line, t.states[k], None);
        t.states.push(next);
    }
    t.states_valid = t.states.len();
}
