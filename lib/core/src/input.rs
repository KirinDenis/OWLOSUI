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
}

impl InputLine {
    pub fn new(label: &str, text: &str) -> Self {
        InputLine {
            label: label.to_string(),
            text: text.to_string(),
            cursor: text.chars().count(),
            focused: false,
            max: 255,
            entered: false,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.chars().count();
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, c: usize) {
        self.cursor = c.min(self.text.chars().count());
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

    /// Handle a key. Returns true if it was used, so a caller can pass on what
    /// was not.
    pub fn key(&mut self, k: Key) -> bool {
        let n = self.text.chars().count();
        match k.code {
            KeyCode::Char(c) if !k.mods.ctrl && !k.mods.alt => {
                if n >= self.max {
                    return true;
                }
                let at = byte_at(&self.text, self.cursor);
                self.text.insert(at, c);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                let at = byte_at(&self.text, self.cursor - 1);
                self.text.remove(at);
                self.cursor -= 1;
            }
            KeyCode::Delete if self.cursor < n => {
                let at = byte_at(&self.text, self.cursor);
                self.text.remove(at);
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(n),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = n,
            KeyCode::Enter => self.entered = true,
            _ => return false,
        }
        true
    }
}

fn byte_at(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map_or(s.len(), |(i, _)| i)
}
