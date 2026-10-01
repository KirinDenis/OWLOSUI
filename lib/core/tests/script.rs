//! Editing tests written as scripts.
//!
//! Every `.txt` under `tests/scripts` is a case, and every line of it is one
//! directive:
//!
//! ```text
//! keymap  modern | classic     which arrangement of keys is in force
//! text    <content>            load the buffer; \n starts a new line
//! keys    <sequence>           feed keys: literal characters, or <Right>,
//!                              <C-Right>, <S-Down>, <Del>, <Enter> …
//! cmd     <Name> …             run named commands, no keyboard involved
//! expect  <content>            the whole buffer must now be this
//! cursor  <line> <column>      the caret must now be here
//! sel     <content>            the selection must now be this
//! clip    <content>            the clipboard must now be this
//! # anything                   a comment
//! ```
//!
//! This is the shape the classic DOS editors had: a set of named primitives with the
//! keyboard bolted on top as data. That made four keymaps possible for them,
//! and it makes these tests possible for us — `cmd` drives the editor with no
//! keyboard at all, and `keys` then checks that a particular arrangement of
//! keys reaches the same commands.
//!
//! A failure prints the script line that failed, because a test that says
//! `assertion failed: left == right` about the fourteenth keystroke of a
//! sequence is not evidence, it is a puzzle.

use owlosui_core::{keymap, Cmd, Keymap, TextView};
use owlosui_core::cell::{glyphs, Glyph};

struct World {
    t: TextView,
    clip: Vec<Vec<Glyph>>,
    keymap: Keymap,
    page: i16,
}

impl World {
    fn new() -> Self {
        World {
            t: TextView::from_ascii(""),
            clip: Vec::new(),
            keymap: Keymap::Modern,
            page: 10,
        }
    }

    fn load(&mut self, s: &str) {
        self.t = TextView::from_ascii(&unescape(s));
    }

    fn key(&mut self, k: owlosui_core::Key) {
        if let Some((cmd, extend)) = self.keymap.lookup(k) {
            self.t.exec(cmd, extend, self.page, &mut self.clip);
        }
    }

    fn cmd(&mut self, c: Cmd) {
        self.t.exec(c, false, self.page, &mut self.clip);
    }
}

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
}

fn lines_to_string(v: &[Vec<Glyph>]) -> String {
    v.iter()
        .map(|l| l.iter().map(|&g| char::from_u32(g as u32).unwrap_or('?')).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Turn `abc<Right><C-Left>` into keys.
fn parse_keys(s: &str, line_no: usize, file: &str) -> Vec<owlosui_core::Key> {
    let mut out = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '<' {
            let mut name = String::new();
            for c in it.by_ref() {
                if c == '>' {
                    break;
                }
                name.push(c);
            }
            match keymap::key_by_name(&name) {
                Some(k) => out.push(k),
                None => panic!("{file}:{line_no}: no such key <{name}>"),
            }
        } else {
            out.push(owlosui_core::Key::new(
                owlosui_core::KeyCode::Char(c),
                owlosui_core::Mods::NONE,
            ));
        }
    }
    out
}

fn run_script(file: &str, src: &str) {
    let mut w = World::new();

    for (i, raw) in src.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (word, rest) = match line.split_once(char::is_whitespace) {
            Some((a, b)) => (a, b.trim_start()),
            None => (line, ""),
        };

        let fail = |what: &str, got: String, want: &str| -> ! {
            panic!("{file}:{line_no}: {what}\n     got: {got:?}\nexpected: {want:?}");
        };

        match word {
            "keymap" => {
                w.keymap = match rest {
                    "classic" => Keymap::Classic,
                    "modern" => Keymap::Modern,
                    _ => panic!("{file}:{line_no}: keymap must be classic or modern"),
                }
            }
            "page" => w.page = rest.parse().expect("page wants a number"),
            "text" => w.load(rest),
            "keys" => {
                for k in parse_keys(rest, line_no, file) {
                    w.key(k);
                }
            }
            "cmd" => {
                for name in rest.split_whitespace() {
                    match keymap::by_name(name) {
                        Some(c) => w.cmd(c),
                        None => panic!("{file}:{line_no}: no such command {name}"),
                    }
                }
            }
            "expect" => {
                let got = w.t.text();
                let want = unescape(rest);
                if got != want {
                    fail("buffer", got, &want);
                }
            }
            "cursor" => {
                let n: Vec<i16> = rest
                    .split_whitespace()
                    .map(|v| v.parse().expect("cursor wants two numbers"))
                    .collect();
                let got = (w.t.cur.y, w.t.cur.x);
                if got != (n[0], n[1]) {
                    fail("caret", format!("{},{}", got.0, got.1), rest);
                }
            }
            "sel" => {
                let got = lines_to_string(&w.t.selected_text());
                let want = unescape(rest);
                if got != want {
                    fail("selection", got, &want);
                }
            }
            "clip" => {
                let got = lines_to_string(&w.clip);
                let want = unescape(rest);
                if got != want {
                    fail("clipboard", got, &want);
                }
            }
            _ => panic!("{file}:{line_no}: unknown directive {word:?}"),
        }
    }
}

#[test]
fn scripts() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/scripts");
    let mut ran = 0;
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |x| x == "txt"))
        .collect();
    files.sort();

    for p in files {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(&p).unwrap();
        run_script(&name, &src);
        ran += 1;
    }
    assert!(ran > 0, "no scripts found in {}", dir.display());
    println!("{ran} scripts");
}
