//! The status line.
//!
//! The bottom row of the screen, grey, with the keys that work right now and
//! what they do: `F1 Help  F10 Menu  Alt-X Exit`. The classic
//! status line did two things at once and so does this: it *shows* the
//! keys, and it *is* where they are bound - press F1 and the status line,
//! not the application, turns it into the Help command. That is why the
//! picture and the behaviour cannot drift apart: they are one list.
//!
//! An item with no key is still clickable, which is how a mouse-only person
//! reaches Exit.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::event::Key;

#[derive(Clone)]
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
    /// What is shown and bound right now: the application's own items and,
    /// after them, whatever the active window brought. Composed by the
    /// desktop before every event and every frame.
    pub items: Vec<StatusItem>,
    /// The application's own items, which every composition starts from.
    pub base: Vec<StatusItem>,
    /// Words at the right end - where the caret is in the active editor,
    /// and how it is set. Composed with the items; shown only where it
    /// does not cover one.
    pub right: String,
    /// Every item in a cell this wide, from the left edge, with no gap -
    /// ten `F1 Help` keys in eighty columns, as a file manager's key bar
    /// had them. 0: each item as wide as its label, two columns apart.
    pub cell: i16,
}

impl StatusLine {
    pub fn new(items: Vec<StatusItem>) -> Self {
        StatusLine { base: items.clone(), items, right: String::new(), cell: 0 }
    }

    /// Where the items end.
    pub fn items_end(&self) -> i16 {
        match (0..self.items.len()).rev().find(|&i| self.items[i].width() > 0) {
            None => 0,
            Some(i) => self.item_x(i) + self.items[i].width(),
        }
    }

    /// Where an item starts. One column in from the left, two between items
    /// - the spacing the hand-drawn status line had before this existed.
    /// An item with no label is a key and nothing to see - Tab in a file
    /// manager, which everybody knows - and takes no room at all.
    pub fn item_x(&self, ix: usize) -> i16 {
        if self.cell > 0 {
            return ix as i16 * self.cell;
        }
        let mut x = 1;
        for it in &self.items[..ix] {
            if it.width() > 0 {
                x += it.width() + 2;
            }
        }
        x
    }

    /// Which item a column is on, for a click.
    pub fn item_at(&self, x: i16) -> Option<usize> {
        (0..self.items.len()).find(|&i| {
            let s = self.item_x(i);
            x >= s && x < s + if self.cell > 0 { self.cell } else { self.items[i].width() }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_with_no_label_is_bound_and_takes_no_room() {
        let tab = Key::new(crate::event::KeyCode::Tab, crate::event::Mods::NONE);
        let s = StatusLine::new(vec![
            StatusItem::new("~F3~ View", None, 1),
            StatusItem::new("", Some(tab), 2),
            StatusItem::new("~F5~ Copy", None, 3),
        ]);
        assert_eq!(s.item_x(2), s.item_x(0) + "F3 View".len() as i16 + 2);
        assert_eq!(s.items_end(), s.item_x(2) + "F5 Copy".len() as i16);
        assert_eq!(s.command_for(tab), Some(2));
    }

    #[test]
    fn items_in_cells_stand_a_cell_apart_and_a_click_takes_the_whole_cell() {
        let mut s = StatusLine::new(vec![StatusItem::new("~F1~ Help", None, 1), StatusItem::new("~F2~ Menu", None, 2)]);
        s.cell = 8;
        assert_eq!((s.item_x(0), s.item_x(1)), (0, 8));
        // The blank after `F1 Help` is still F1's.
        assert_eq!(s.item_at(7), Some(0));
        assert_eq!(s.item_at(8), Some(1));
    }
}
