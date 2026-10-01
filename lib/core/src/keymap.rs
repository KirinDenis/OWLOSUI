//! Keys to commands.
//!
//! The editor does not contain a single key code. This file is the whole of
//! the mapping, and there are two of them because there are two kinds of
//! person who will use this.
//!
//! `classic` is the old DOS editors' arrangement — WordStar control keys, `Ctrl+Y` to
//! delete a line, `Ctrl+K B` to mark a block. `modern` is what someone who has
//! never seen a DOS editor will try first: `Ctrl+C`, `Ctrl+V`, `Ctrl+Z`,
//! Shift and an arrow to select.
//!
//! Modern is the default, and that is a deliberate choice rather than a
//! concession. Somebody who presses Ctrl+C, gets something else and loses
//! their work does not conclude that the program is authentic.
//!
//! One group belongs to both: **Ctrl+Ins, Shift+Ins and Shift+Del**. They are
//! CUA — IBM's, older than Ctrl+C/V and still working in Windows today — and
//! in a terminal they are not merely traditional but necessary, because Ctrl+C
//! is taken by the signal.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::edit::Cmd;
use crate::event::{Key, KeyCode, Mods};

/// A command, and whether the key was holding Shift to extend a selection.
pub type Bound = (Cmd, bool);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Keymap {
    Classic,
    Modern,
}

impl Keymap {
    pub fn lookup(self, k: Key) -> Option<Bound> {
        shared(k).or(match self {
            Keymap::Classic => classic(k),
            Keymap::Modern => modern(k),
        })
    }
}

/// What both arrangements agree on: the arrows, the editing keys, and the CUA
/// clipboard that outlived everything else.
fn shared(k: Key) -> Option<Bound> {
    let s = k.mods.shift;
    let c = k.mods.ctrl;

    let cmd = match k.code {
        KeyCode::Left if c => Cmd::WordLeft,
        KeyCode::Right if c => Cmd::WordRight,
        KeyCode::Left => Cmd::CharLeft,
        KeyCode::Right => Cmd::CharRight,
        KeyCode::Up => Cmd::LineUp,
        KeyCode::Down => Cmd::LineDown,
        KeyCode::Home if c => Cmd::TextStart,
        KeyCode::End if c => Cmd::TextEnd,
        KeyCode::Home => Cmd::LineStart,
        KeyCode::End => Cmd::LineEnd,
        KeyCode::PageUp => Cmd::PageUp,
        KeyCode::PageDown => Cmd::PageDown,

        KeyCode::Enter => Cmd::NewLine,
        KeyCode::Backspace => Cmd::DeleteLeft,

        // The CUA clipboard. Shift+Del must be tested before plain Delete or
        // it is read as a delete that happens to have Shift held.
        KeyCode::Delete if s => Cmd::Cut,
        KeyCode::Insert if c => Cmd::Copy,
        KeyCode::Insert if s => Cmd::Paste,
        KeyCode::Delete => Cmd::DeleteRight,

        KeyCode::Char(ch) if k.mods.is_none() => Cmd::Insert(ch),
        // Shift is already in the character the terminal gave us; letting it
        // through here is what makes a capital letter typeable.
        KeyCode::Char(ch) if s && !c && !k.mods.alt => Cmd::Insert(ch),

        _ => return None,
    };

    Some((cmd, s && cmd.is_movement()))
}

/// The old DOS editors'. WordStar underneath, which is why it looks arbitrary until you
/// know that Ctrl+E/S/D/X are a diamond under the left hand.
fn classic(k: Key) -> Option<Bound> {
    if !k.mods.ctrl {
        return None;
    }
    let cmd = match k.code {
        KeyCode::Char('s') => Cmd::CharLeft,
        KeyCode::Char('d') => Cmd::CharRight,
        KeyCode::Char('e') => Cmd::LineUp,
        KeyCode::Char('x') => Cmd::LineDown,
        KeyCode::Char('a') => Cmd::WordLeft,
        KeyCode::Char('f') => Cmd::WordRight,
        KeyCode::Char('r') => Cmd::PageUp,
        KeyCode::Char('c') => Cmd::PageDown,
        KeyCode::Char('g') => Cmd::DeleteRight,
        KeyCode::Char('h') => Cmd::DeleteLeft,
        KeyCode::Char('y') => Cmd::DeleteLine,
        KeyCode::Char('u') => Cmd::Undo,
        _ => return None,
    };
    Some((cmd, false))
}

/// What anyone under fifty will try without being told.
fn modern(k: Key) -> Option<Bound> {
    if !k.mods.ctrl {
        return None;
    }
    let cmd = match k.code {
        KeyCode::Char('c') => Cmd::Copy,
        KeyCode::Char('x') => Cmd::Cut,
        KeyCode::Char('v') => Cmd::Paste,
        KeyCode::Char('z') => Cmd::Undo,
        KeyCode::Char('y') => Cmd::Redo,
        KeyCode::Char('a') => Cmd::SelectAll,
        _ => return None,
    };
    Some((cmd, false))
}

/// Every command by name, for scripts and for anyone building a keymap editor.
pub fn by_name(name: &str) -> Option<Cmd> {
    use Cmd::*;
    Some(match name {
        "CharLeft" => CharLeft,
        "CharRight" => CharRight,
        "LineUp" => LineUp,
        "LineDown" => LineDown,
        "WordLeft" => WordLeft,
        "WordRight" => WordRight,
        "LineStart" => LineStart,
        "LineEnd" => LineEnd,
        "PageUp" => PageUp,
        "PageDown" => PageDown,
        "TextStart" => TextStart,
        "TextEnd" => TextEnd,
        "NewLine" => NewLine,
        "DeleteLeft" => DeleteLeft,
        "DeleteRight" => DeleteRight,
        "DeleteLine" => DeleteLine,
        "SelectAll" => SelectAll,
        "SelectNone" => SelectNone,
        "Cut" => Cut,
        "Copy" => Copy,
        "Paste" => Paste,
        "Undo" => Undo,
        "Redo" => Redo,
        _ => return None,
    })
}

/// A key written the way a script writes it: `Right`, `C-Right`, `S-Down`.
pub fn key_by_name(name: &str) -> Option<Key> {
    let mut mods = Mods::NONE;
    let mut rest = name;
    loop {
        match rest.split_once('-') {
            Some(("C", r)) => {
                mods.ctrl = true;
                rest = r;
            }
            Some(("S", r)) => {
                mods.shift = true;
                rest = r;
            }
            Some(("A", r)) => {
                mods.alt = true;
                rest = r;
            }
            _ => break,
        }
    }
    let code = match rest {
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PgUp" => KeyCode::PageUp,
        "PgDn" => KeyCode::PageDown,
        "Enter" => KeyCode::Enter,
        "BS" => KeyCode::Backspace,
        "Del" => KeyCode::Delete,
        "Ins" => KeyCode::Insert,
        "Tab" => KeyCode::Tab,
        "Esc" => KeyCode::Esc,
        "Space" => KeyCode::Char(' '),
        s if s.chars().count() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => return None,
    };
    Some(Key::new(code, mods))
}
