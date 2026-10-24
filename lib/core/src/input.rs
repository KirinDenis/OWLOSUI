//! A single line of text with a label in front of it.
//!
//! This is the composite we said an input field is: a piece of static text and
//! an editor one line tall. It is one control and not two because the two
//! always travel together — a field with no label is a box nobody can name,
//! and a label with no field is a caption.
//!
//! It is used in two places at once, which is the whole reason it lives in a
//! file of its own: the file panel owns one for its path, and it is also a
//! view in its own right for the day dialogs need fields. One implementation,
//! two uses — copy it and the second copy is where the bugs will be.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::event::{Key, KeyCode};

pub struct InputLine {
    /// Shown in front, with its colon: `Path:`. Written here rather than
    /// composed by the caller so every field in a program lines up the same
    /// way without anyone having to remember.
    pub label: String,
    pub text: String,
    /// Caret position, counted in characters.
    pub cursor: usize,
    pub focused: bool,
    pub max: usize,
    /// Set when Enter was pressed. What that means is the owner's business.
    pub entered: bool,
    /// What has been entered here before, newest first. Down opens it as
    /// a list under the field, and so does the `▼` at the field's end.
    /// The classic history list, without the separate view.
    pub history: Vec<String>,
    /// The other end of the selection, in characters; the caret is the
    /// near end. Shift with the arrows, Ctrl+A or the mouse dragged over
    /// the field makes one; typing replaces it, Ctrl+C and Ctrl+X take it.
    pub anchor: Option<usize>,
    /// The text and the caret before each change, newest last: Ctrl+Z.
    undo: Vec<(String, usize)>,
}

/// How many changes Ctrl+Z can take back in a field.
const UNDO_DEPTH: usize = 50;

impl InputLine {
    pub fn new(label: &str, text: &str) -> Self {
        InputLine {
            label: label.to_string(),
            text: text.to_string(),
            cursor: text.chars().count(),
            focused: false,
            max: 255,
            entered: false,
            history: Vec::new(),
            anchor: None,
            undo: Vec::new(),
        }
    }

    /// Put the current text at the top of the history: once, and only
    /// if it says something.
    pub fn remember(&mut self) {
        let t = self.text.trim();
        if t.is_empty() {
            return;
        }
        let t = t.to_string();
        self.history.retain(|h| *h != t);
        self.history.insert(0, t);
        self.history.truncate(20);
    }

    /// New words from the program: a new text, so nothing to take back.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.chars().count();
        self.anchor = None;
        self.undo.clear();
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The caret to `c`; any selection goes.
    pub fn set_cursor(&mut self, c: usize) {
        self.cursor = c.min(self.text.chars().count());
        self.anchor = None;
    }

    /// The caret to `c` with the selection's far end staying where it is -
    /// or, with none yet, where the caret was: the mouse dragging.
    pub fn extend_to(&mut self, c: usize) {
        if self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        }
        self.cursor = c.min(self.text.chars().count());
    }

    /// The selection as (start, end) in characters, end not included;
    /// None when nothing is selected.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    /// The selected words, if any.
    pub fn selected(&self) -> Option<String> {
        let (a, b) = self.selection()?;
        Some(self.text.chars().skip(a).take(b - a).collect())
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.text.chars().count();
    }

    /// The selection out of the text and returned - Ctrl+X; the clipboard
    /// is the owner's.
    pub fn cut(&mut self) -> Option<String> {
        let s = self.selected()?;
        self.remember_undo();
        self.delete_selection();
        Some(s)
    }

    /// Ctrl+Z: the text as it was before the last change.
    pub fn undo(&mut self) -> bool {
        let Some((text, cursor)) = self.undo.pop() else { return false };
        self.text = text;
        self.cursor = cursor.min(self.text.chars().count());
        self.anchor = None;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    fn remember_undo(&mut self) {
        if self.undo.len() >= UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.undo.push((self.text.clone(), self.cursor));
    }

    /// The selected characters gone, the caret where they began.
    fn delete_selection(&mut self) -> bool {
        let Some((a, b)) = self.selection() else {
            self.anchor = None;
            return false;
        };
        let (from, to) = (byte_at(&self.text, a), byte_at(&self.text, b));
        self.text.replace_range(from..to, "");
        self.cursor = a;
        self.anchor = None;
        true
    }

    /// Where Ctrl+Left lands from `c`: the start of the word before it.
    fn word_left(&self, c: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = c.min(chars.len());
        while i > 0 && !chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        while i > 0 && chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        i
    }

    /// Where Ctrl+Right lands from `c`: the start of the next word.
    fn word_right(&self, c: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = c.min(chars.len());
        while i < chars.len() && chars[i].is_alphanumeric() {
            i += 1;
        }
        while i < chars.len() && !chars[i].is_alphanumeric() {
            i += 1;
        }
        i
    }

    /// The word under character `c`: a double click.
    pub fn select_word(&mut self, c: usize) {
        let chars: Vec<char> = self.text.chars().collect();
        let c = c.min(chars.len());
        let (mut a, mut b) = (c, c);
        while a > 0 && chars[a - 1].is_alphanumeric() {
            a -= 1;
        }
        while b < chars.len() && chars[b].is_alphanumeric() {
            b += 1;
        }
        self.anchor = Some(a);
        self.cursor = b;
    }

    /// Where the field starts, relative to the control: past the label and the
    /// space after it.
    pub fn field_x(&self) -> i16 {
        if self.label.is_empty() {
            0
        } else {
            self.label.chars().count() as i16 + 1
        }
    }

    /// The first visible character, worked out rather than remembered.
    ///
    /// Keeping it as state would mean drawing had to mutate the control, and a
    /// view that changes when it is looked at is a view that draws differently
    /// depending on how often you looked. The rule is simple enough not to
    /// need memory: the caret is always in sight, and a path too long for its
    /// field shows its tail — which is the end you were typing.
    pub fn left(&self, width: i16) -> usize {
        let w = self.width(width);
        self.cursor.saturating_sub(w.saturating_sub(1))
    }

    fn width(&self, width: i16) -> usize {
        (width - self.field_x()).max(1) as usize
    }

    /// The caret's column within the control, or `None` when the field does
    /// not have the focus and so has no caret to show.
    pub fn caret_x(&self, width: i16) -> Option<i16> {
        self.focused
            .then(|| self.field_x() + (self.cursor - self.left(width)) as i16)
    }

    /// The part of the text the field can show, given its width.
    pub fn visible(&self, width: i16) -> String {
        self.text
            .chars()
            .skip(self.left(width))
            .take(self.width(width))
            .collect()
    }

    /// Words in at the caret, over the selection if there is one, as many
    /// as the field has room for - what a paste does here. A field is one
    /// line, so the paste is too: a line break becomes a space.
    pub fn insert(&mut self, s: &str) {
        self.remember_undo();
        self.delete_selection();
        let room = self.max.saturating_sub(self.text.chars().count());
        let s: String = s.chars().filter(|&c| c != '\r').map(|c| if c == '\n' { ' ' } else { c }).take(room).collect();
        let at = byte_at(&self.text, self.cursor);
        self.text.insert_str(at, &s);
        self.cursor += s.chars().count();
    }

    /// Handle a key. Returns true if it was used, so a caller can pass on what
    /// was not. Copy, cut and paste are the owner's - the clipboard is not
    /// the field's - with `selected`, `cut` and `insert`.
    pub fn key(&mut self, k: Key) -> bool {
        let n = self.text.chars().count();
        let m = k.mods;
        // A move with Shift held carries the selection with it; without,
        // it lets the selection go - from where the move started, so Left
        // on a selection lands at its start, as everywhere else.
        let go = |me: &mut Self, to: usize| {
            if m.shift {
                me.extend_to(to);
            } else {
                me.set_cursor(to);
            }
        };
        match k.code {
            KeyCode::Char('a') | KeyCode::Char('A') if m.ctrl && !m.alt => self.select_all(),
            KeyCode::Char('z') | KeyCode::Char('Z') if m.ctrl && !m.alt => {
                self.undo();
            }
            KeyCode::Char(c) if !m.ctrl && !m.alt => {
                if self.selection().is_none() && n >= self.max {
                    return true;
                }
                self.remember_undo();
                self.delete_selection();
                let at = byte_at(&self.text, self.cursor);
                self.text.insert(at, c);
                self.cursor += 1;
            }
            KeyCode::Backspace | KeyCode::Delete if self.selection().is_some() => {
                self.remember_undo();
                self.delete_selection();
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.remember_undo();
                let from = if m.ctrl { self.word_left(self.cursor) } else { self.cursor - 1 };
                let (a, b) = (byte_at(&self.text, from), byte_at(&self.text, self.cursor));
                self.text.replace_range(a..b, "");
                self.cursor = from;
                self.anchor = None;
            }
            KeyCode::Delete if self.cursor < n => {
                self.remember_undo();
                let to = if m.ctrl { self.word_right(self.cursor) } else { self.cursor + 1 };
                let (a, b) = (byte_at(&self.text, self.cursor), byte_at(&self.text, to));
                self.text.replace_range(a..b, "");
                self.anchor = None;
            }
            KeyCode::Backspace | KeyCode::Delete => self.anchor = None,
            KeyCode::Left => {
                let to = match (m.ctrl, self.selection()) {
                    (true, _) => self.word_left(self.cursor),
                    (false, Some((a, _))) if !m.shift => a,
                    _ => self.cursor.saturating_sub(1),
                };
                go(self, to);
            }
            KeyCode::Right => {
                let to = match (m.ctrl, self.selection()) {
                    (true, _) => self.word_right(self.cursor),
                    (false, Some((_, b))) if !m.shift => b,
                    _ => (self.cursor + 1).min(n),
                };
                go(self, to);
            }
            KeyCode::Home => go(self, 0),
            KeyCode::End => go(self, n),
            KeyCode::Enter => self.entered = true,
            _ => return false,
        }
        true
    }
}

fn byte_at(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map_or(s.len(), |(i, _)| i)
}
