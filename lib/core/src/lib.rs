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

pub mod buffer;
pub mod button;
pub mod cell;
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
pub mod tree;
pub mod ui;
pub mod views;

pub use buffer::Buffer;
pub use button::{Align, Button as PushButton, ButtonRow};
pub use cell::{attr, Cell, Color};
pub use event::{Button, Event, Key, KeyCode, Mods, Mouse, MouseKind};
pub use files::{FileEntry, FileKind, FileList};
pub use geom::{Point, Rect};
pub use palette::{Palette, WinColors, WinPalette};
pub use controls::{Choice, Cluster, ListBox, StaticText};
pub use edit::Cmd;
pub use hex::HexView;
pub use html::Html;
pub use input::InputLine;
pub use keymap::Keymap;
pub use menu::{MenuBar, MenuBox, MenuItem};
pub use tree::{TreeNode, TreeView};
pub use ui::{Ui, ViewId};
pub use views::{Desktop, Dock, Kind, TextView, Window};
