//! The status line.
//!
//! The bottom row of the screen, grey, with the keys that work right now and
//! what they do: `F1 Help  F10 Menu  Alt-X Exit`. Turbo Vision's
//! `TStatusLine` did two things at once and so does this: it *shows* the
//! keys, and it *is* where they are bound - press F1 and the status line,
//! not the application, turns it into the Help command. That is why the
//! picture and the behaviour cannot drift apart: they are one list.
//!
//! An item with no key is still clickable, which is how a mouse-only person
//! reaches Exit.

use crate::event::Key;

pub struct StatusItem {
    /// The label, with the key's name between tildes: `~F1~ Help`. What is
    /// between the tildes is drawn in the key colour; the rest in the text
    /// colour. Nothing about the tildes is a hotkey - the key itself is in
    /// `key`, and the two are allowed to differ, though it would be odd.
    pub text: String,
    /// The key that sends `cmd`, if one does.
    pub key: Option<Key>,
    pub cmd: u16,
    pub enabled: bool,
}

impl StatusItem {
    pub fn new(text: &str, key: Option<Key>, cmd: u16) -> Self {
        StatusItem {
            text: text.into(),
            key,
            cmd,
            enabled: true,
        }
    }

    pub fn label(&self) -> String {
        self.text.replace('~', "")
    }

    /// Where the key's name is inside the label: `(start, len)` in
    /// characters, or `None` when the text has no tildes.
    pub fn key_span(&self) -> Option<(usize, usize)> {
        let mut it = self.text.split('~');
        let before = it.next()?.chars().count();
        let inside = it.next()?.chars().count();
        Some((before, inside))
    }

    pub fn width(&self) -> i16 {
        self.label().chars().count() as i16
    }
}

pub struct StatusLine {
    pub items: Vec<StatusItem>,
}

impl StatusLine {
    pub fn new(items: Vec<StatusItem>) -> Self {
        StatusLine { items }
    }

    /// Where an item starts. One column in from the left, two between items
    /// - the spacing the hand-drawn status line had before this existed.
    pub fn item_x(&self, ix: usize) -> i16 {
        let mut x = 1;
        for it in &self.items[..ix] {
            x += it.width() + 2;
        }
        x
    }

    /// Which item a column is on, for a click.
    pub fn item_at(&self, x: i16) -> Option<usize> {
        (0..self.items.len()).find(|&i| {
            let s = self.item_x(i);
            x >= s && x < s + self.items[i].width()
        })
    }

    /// The command a key sends, if the status line binds it.
    pub fn command_for(&self, k: Key) -> Option<u16> {
        self.items
            .iter()
            .find(|it| it.enabled && it.cmd != 0 && it.key == Some(k))
            .map(|it| it.cmd)
    }
}
