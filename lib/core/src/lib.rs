//! OWLOSUI core — the part that is the same everywhere.
//!
//! The contract with a backend is three things and nothing else:
//!
//!   * events go in   (`Event`)
//!   * a grid of cells comes out  (`Buffer`)
//!   * the backend decides how to show it
//!
//! No I/O, no allocation of anything the caller cannot see, no knowledge of
//! terminals, browsers or video memory. This crate has no dependencies and
//! must never acquire one: it has to be buildable for a machine where the
//! whole program lives in 64K.
//!
//! Which is why it is `no_std` unless the `std` feature says otherwise:
//! `core` and `alloc` are all it uses, and a DOS build has no more.

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

pub mod buffer;
pub mod button;
pub mod cell;
pub mod console;
pub mod edit;
pub mod controls;
pub mod event;
pub mod files;
pub mod geom;
pub mod hex;
pub mod html;
pub mod input;
pub mod keymap;
pub mod menu;
pub mod palette;
pub mod status;
pub mod syntax;
pub mod tree;
pub mod ui;
pub mod views;

pub use buffer::Buffer;
pub use console::Console;
pub use button::{Align, Button as PushButton, ButtonRow, ButtonStyle};
pub use cell::{attr, glyph_of, glyphs, Cell, Color, Glyph, GLYPH_MAX};
pub use event::{Button, Event, Key, KeyCode, Mods, Mouse, MouseKind};
pub use files::{FileEntry, FileKind, FileList};
pub use geom::{Point, Rect};
pub use palette::{Palette, WinColors, WinPalette};
pub use controls::{Canvas, Choice, Cluster, Label, ListBox, Progress, StaticText};
pub use status::{StatusItem, StatusLine};
pub use edit::Cmd;
pub use hex::HexView;
pub use html::Html;
pub use input::InputLine;
pub use keymap::Keymap;
pub use menu::{MenuBar, MenuBox, MenuItem};
pub use tree::{TreeNode, TreeView};
pub use ui::{Ui, ViewId, CM_INTERNAL};
pub use views::{Desktop, Dock, Kind, TextView, Window};
