//! The core behind a pipe.
//!
//! A client starts this program, writes requests to its stdin and reads one
//! reply per request from its stdout. The format is `lib/PROTOCOL.md`, and the
//! same numbers are meant to go into `AH` one day when the transport is a
//! software interrupt instead of a pipe.
//!
//! Nothing here draws. The client asks for the frame and puts the cells on
//! whatever it has. That is what makes a C# console program, a browser page
//! and a DOS `.COM` the same kind of thing: each is a renderer and an event
//! source, and neither knows what a window is.
//!
//! The one rule of this file: **bad input is a reply, never a panic.** The
//! other end may be a program somebody is still writing, and the most useful
//! thing a server can do for them is stay up and say what was wrong.
//!
//! This is the library half: `Server::call` takes one request and gives one
//! reply. `main.rs` puts it behind stdin and stdout for the C# client;
//! `lib/js/wire` puts it inside a WebAssembly module for the JavaScript one.
//! Same bytes either way.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// What `std`'s prelude would have given; the same names from `alloc`, so
// the crate reads the same with or without it.
#[allow(unused_imports)]
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use owlosui_console::codepage::Font;
use owlosui_core::{
    Align, Button, ButtonRow, ButtonStyle, Canvas, Cell, Choice, Cluster, Dock, Event, FileEntry,
    FileList, InputLine, Key, KeyCode, Kind, Label, ListBox, MenuBar, MenuItem, Mods, Mouse,
    MouseKind, Progress, PushButton, Rect, StaticText, StatusItem, StatusLine, TextView, Ui, ViewId,
    WinPalette, Window,
};

pub mod op {
    pub const QUIT: u8 = 0x00;
    pub const INIT: u8 = 0x01;
    pub const RESIZE: u8 = 0x02;
    pub const CODEPAGE: u8 = 0x03;
    pub const WINDOW: u8 = 0x10;
    pub const TEXT: u8 = 0x11;
    pub const STATIC: u8 = 0x12;
    pub const INPUT: u8 = 0x13;
    pub const BUTTONS: u8 = 0x14;
    pub const MESSAGE_BOX: u8 = 0x15;
    pub const STATUS: u8 = 0x16;
    pub const LABEL: u8 = 0x17;
    pub const PROGRESS: u8 = 0x18;
    pub const LIST: u8 = 0x19;
    pub const FILES: u8 = 0x1A;
    pub const CANVAS: u8 = 0x1B;
    pub const MENU_BAR: u8 = 0x1C;
    pub const CLUSTER: u8 = 0x1D;
    pub const BUTTON_ROW: u8 = 0x1E;
    pub const HEX: u8 = 0x1F;
    pub const CLOSE: u8 = 0x20;
    pub const GET_TEXT: u8 = 0x21;
    pub const SET_PROGRESS: u8 = 0x22;
    pub const GET_MARKED: u8 = 0x23;
    pub const GET_CURRENT: u8 = 0x24;
    pub const SET_FILES: u8 = 0x25;
    pub const TAKE_FILES: u8 = 0x26;
    pub const ACTIVATE: u8 = 0x27;
    pub const MARKED_NAMES: u8 = 0x28;
    pub const SET_FILES_ERROR: u8 = 0x29;
    pub const ADD_FILES: u8 = 0x5A;
    /// Answered by the stdio host itself (main.rs), never by `call`: the
    /// desktop moves into a native window. See window.rs.
    pub const OPEN_WINDOW: u8 = 0x5B;
    /// Likewise: the next input from that window.
    pub const WAIT: u8 = 0x5C;
    pub const ACTIVE: u8 = 0x2A;
    pub const SET_TEXT: u8 = 0x2B;
    pub const BLIT: u8 = 0x2C;
    pub const MENU_CHECK: u8 = 0x2D;
    pub const GET_CLUSTER: u8 = 0x2E;
    pub const GET_CLICK: u8 = 0x2F;
    pub const KEY: u8 = 0x30;
    pub const MOUSE: u8 = 0x31;
    pub const TICK: u8 = 0x32;
    pub const FRAME: u8 = 0x40;
    pub const TAKE: u8 = 0x41;
    pub const GET_GLYPHS: u8 = 0x42;
    pub const CYCLE: u8 = 0x43;
    pub const ZOOM: u8 = 0x44;
    pub const SET_BUTTON: u8 = 0x45;
    pub const FOCUS: u8 = 0x46;
    pub const CASCADE: u8 = 0x47;
    pub const TILE: u8 = 0x48;
    pub const SET_READONLY: u8 = 0x49;
    pub const WINDOW_STATUS: u8 = 0x4A;
    pub const WINDOW_MENU: u8 = 0x4B;
    pub const WINDOW_LIST: u8 = 0x4C;
    pub const CYCLE_BACK: u8 = 0x4D;
    pub const SIZE_MOVE: u8 = 0x4E;
    pub const SET_HISTORY: u8 = 0x4F;
    pub const GET_HISTORY: u8 = 0x50;
    pub const PALETTE: u8 = 0x51;
    pub const SET_COLOR: u8 = 0x52;
    pub const TREE: u8 = 0x53;
    pub const TREE_CHILDREN: u8 = 0x54;
    pub const TREE_EXPAND: u8 = 0x55;
    pub const TREE_PATH: u8 = 0x56;
    pub const FIND: u8 = 0x57;
    pub const REPLACE: u8 = 0x58;
    pub const REPLACE_ALL: u8 = 0x59;
    pub const EDITOR: u8 = 0x5D;
    pub const GET_EDITOR: u8 = 0x5E;
}

type Res<T> = Result<T, String>;

// ------------------------------------------------------------------ reading

/// A cursor over one request's payload. Every read says what it wanted when
/// it runs out, so a client with a field missing hears which one.
struct In<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> In<'a> {
    fn take(&mut self, n: usize, what: &str) -> Res<&'a [u8]> {
        if self.p + n > self.b.len() {
            return Err(format!("payload ends before {what}"));
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn remaining(&self) -> usize {
        self.b.len() - self.p
    }
    fn u8(&mut self, what: &str) -> Res<u8> {
        Ok(self.take(1, what)?[0])
    }
    fn u16(&mut self, what: &str) -> Res<u16> {
        let s = self.take(2, what)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn i16(&mut self, what: &str) -> Res<i16> {
        Ok(self.u16(what)? as i16)
    }
    fn u32(&mut self, what: &str) -> Res<u32> {
        let s = self.take(4, what)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    /// `kind:u8 value:u16 mods:u8`, as KEY carries it; `0xFF` means no key.
    fn key(&mut self) -> Res<Option<Key>> {
        let kind = self.u8("key.kind")?;
        let value = self.u16("key.value")?;
        let m = self.u8("key.mods")?;
        if kind == 0xFF {
            return Ok(None);
        }
        decode_key(kind, value, m).map(Some)
    }
    fn id(&mut self, what: &str) -> Res<ViewId> {
        Ok(ViewId::from_raw(self.u16(what)? as u32))
    }
    fn rect(&mut self) -> Res<Rect> {
        Ok(Rect::new(
            self.i16("rect.x")?,
            self.i16("rect.y")?,
            self.i16("rect.w")?,
            self.i16("rect.h")?,
        ))
    }
    /// Whatever is left of the payload.
    fn rest(&mut self) -> &[u8] {
        let r = &self.b[self.p..];
        self.p = self.b.len();
        r
    }
    fn str(&mut self, what: &str) -> Res<String> {
        let n = self.u16(what)? as usize;
        let s = self.take(n, what)?;
        String::from_utf8(s.to_vec()).map_err(|_| format!("{what} is not UTF-8"))
    }
    /// `n:u16` then `n ×` (`name:str size:u32 year:u16 month:u8 day:u8
    /// hour:u8 minute:u8 attrs:u8`): a directory listing, read by the
    /// client, because the server reads no files and no directories.
    fn entries(&mut self, cp: &mut Font) -> Res<Vec<FileEntry>> {
        let n = self.u16("entry count")?;
        let mut v = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let name = cp.to_core(&self.str("entry.name")?);
            let size = self.u32("entry.size")?;
            let year = self.u16("entry.year")?;
            let month = self.u8("entry.month")?;
            let day = self.u8("entry.day")?;
            let hour = self.u8("entry.hour")?;
            let minute = self.u8("entry.minute")?;
            let attrs = self.u8("entry.attrs")?;
            v.push(FileEntry {
                name,
                size,
                date: (year, month, day, hour, minute),
                attrs,
            });
        }
        Ok(v)
    }

    /// `n:u8` then `n ×` (`flags:u8 cmd:u16 text:str shortcut:str` then the
    /// item's own submenu, the same shape): a menu, as deep as it goes.
    /// `flags`: bit 0 separator, bit 1 disabled, bit 2 ticked.
    /// `n:u8` then `n ×` (`cmd:u16 key label:str`): status items.
    fn status_items(&mut self, cp: &mut Font) -> Res<Vec<StatusItem>> {
        let n = self.u8("item count")?;
        let mut items = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let cmd = self.u16("status.cmd")?;
            let key = self.key()?;
            let text = cp.to_core(&self.str("status.text")?);
            items.push(StatusItem::new(&text, key, cmd));
        }
        Ok(items)
    }

    /// `n:u8` then `n ×` (`flags:u8 text:str` then its children the same
    /// way): tree nodes. Flags: bit 0 open, bit 1 lazy - children exist
    /// but are given only when the node is opened.
    fn tree_nodes(&mut self, cp: &mut Font, depth: u8) -> Res<Vec<owlosui_core::TreeNode>> {
        if depth > 32 {
            return Err("a tree deeper than anyone can follow".into());
        }
        let n = self.u8("node count")?;
        let mut v = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let flags = self.u8("node.flags")?;
            let text = cp.to_core(&self.str("node.text")?);
            let children = self.tree_nodes(cp, depth + 1)?;
            let mut node = owlosui_core::TreeNode::leaf(&text);
            node.children = children;
            node.open = flags & 1 != 0;
            node.lazy = flags & 2 != 0;
            v.push(node);
        }
        Ok(v)
    }

    /// `depth:u8` then `depth × u16`: a path down a tree.
    fn tree_path(&mut self) -> Res<Vec<usize>> {
        let d = self.u8("path depth")?;
        let mut p = Vec::with_capacity(d as usize);
        for _ in 0..d {
            p.push(self.u16("path index")? as usize);
        }
        Ok(p)
    }

    fn menu_items(&mut self, cp: &mut Font, depth: u8) -> Res<Vec<MenuItem>> {
        if depth > 8 {
            return Err("a menu nested deeper than anyone can follow".into());
        }
        let n = self.u8("item count")?;
        let mut v = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let flags = self.u8("item.flags")?;
            let cmd = self.u16("item.cmd")?;
            let text = cp.to_core(&self.str("item.text")?);
            let shortcut = cp.to_core(&self.str("item.shortcut")?);
            let hint = cp.to_core(&self.str("item.hint")?);
            let sub = self.menu_items(cp, depth + 1)?;
            let mut it = if flags & 1 != 0 {
                MenuItem::line()
            } else if !sub.is_empty() {
                MenuItem::sub(&text, sub)
            } else {
                MenuItem::new(&text, &shortcut, cmd)
            };
            it = it.hint(&hint);
            if flags & 2 != 0 {
                it = it.disabled();
            }
            if flags & 4 != 0 {
                it = it.checked(true);
            }
            v.push(it);
        }
        Ok(v)
    }

    /// `n ×` (`cmd:u16 flags:u8 label:str`) — the shape both BUTTONS and
    /// MESSAGE_BOX carry.
    fn buttons(&mut self, cp: &mut Font) -> Res<ButtonRow> {
        let n = self.u8("button count")?;
        let mut v = Vec::with_capacity(n as usize);
        for i in 0..n {
            let cmd = self.u16("button.cmd")?;
            if cmd == 0 {
                return Err(format!("button {i} has command 0, which means none"));
            }
            let flags = self.u8("button.flags")?;
            let label = cp.to_core(&self.str("button.label")?);
            let mut b = PushButton::new(&label, cmd);
            if flags & 1 != 0 {
                b = b.default();
            }
            // Bits 1-2: 0 normal, 1 accent, 2 danger. Bit 3: disabled.
            b = b.style(match (flags >> 1) & 3 {
                1 => ButtonStyle::Accent,
                2 => ButtonStyle::Danger,
                _ => ButtonStyle::Normal,
            });
            if flags & 8 != 0 {
                b = b.disabled();
            }
            if flags & 16 != 0 {
                b = b.cancel();
            }
            v.push(b);
        }
        if v.is_empty() {
            return Err("a button row with no buttons".into());
        }
        Ok(ButtonRow::new(v))
    }
}

// ------------------------------------------------------------------ writing

#[derive(Default)]
struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i16(&mut self, v: i16) {
        self.u16(v as u16);
    }
    fn id(&mut self, v: ViewId) {
        self.u16(v.raw() as u16);
    }
    fn str(&mut self, s: &str) {
        self.u16(s.len() as u16);
        self.0.extend_from_slice(s.as_bytes());
    }
}

// ------------------------------------------------------------------- server

pub struct Server {
    ui: Option<Ui>,
    /// The status line, if one has been made. A second one replaces it.
    status: Option<ViewId>,
    /// The menu bar, likewise.
    menu: Option<ViewId>,
    /// The font every string is turned into on the way in and back on the
    /// way out. One per session, like the font in a video card - except
    /// that this one grows: a character it has never seen gets the next
    /// index, and the client fetches the table again when a frame refers
    /// past what it has.
    cp: Font,
}

impl Default for Server {
    fn default() -> Self {
        Server::new()
    }
}

impl Server {
    /// A server before INIT: no screen yet, code page 437, a font that grows.
    pub fn new() -> Server {
        Server {
            ui: None,
            status: None,
            menu: None,
            cp: Font::growing(437).expect("437 is built in"),
        }
    }

    /// One request, one reply: `(status, body, keep_going)`. Status 0 is
    /// OK and the body is the reply; 1 is an error and the body is the
    /// message as a string. `keep_going` is false after QUIT.
    pub fn call(&mut self, op: u8, payload: &[u8]) -> (u8, Vec<u8>, bool) {
        let mut out = Out::default();
        match self.handle(op, payload, &mut out) {
            Ok(going) => (0, out.0, going),
            Err(e) => {
                let mut msg = Out::default();
                msg.str(&e);
                (1, msg.0, true)
            }
        }
    }

    /// The core, once INIT has made it. For a host that draws it itself.
    pub fn ui_mut(&mut self) -> Option<&mut Ui> {
        self.ui.as_mut()
    }

    /// What a glyph index of this session's font looks like.
    pub fn glyph(&self, g: owlosui_core::Glyph) -> char {
        self.cp.to_char(g)
    }

    fn ui(&mut self) -> Res<&mut Ui> {
        self.ui.as_mut().ok_or_else(|| "INIT first".to_string())
    }

    fn alive(&mut self, id: ViewId) -> Res<ViewId> {
        if self.ui()?.is_alive(id) {
            Ok(id)
        } else {
            Err(format!("no view {}", id.raw()))
        }
    }

    /// A dialog with something focusable in it and nothing focused is a
    /// dialog whose first key goes nowhere. The first control to arrive
    /// takes the focus, as it would in a dialog built by hand.
    fn settle_focus(&mut self) -> Res<()> {
        let ui = self.ui()?;
        if ui.focused().is_none() {
            ui.focus_first();
        }
        Ok(())
    }

    /// `-1` in either coordinate means "in the middle of the work area" -
    /// now and after every resize, until it is dragged. Returns the rect and
    /// whether that was asked for.
    fn place(&mut self, r: Rect) -> Res<(Rect, bool)> {
        let ui = self.ui()?;
        let work = ui.work_area();
        let centred = r.x < 0 || r.y < 0;
        let x = if r.x < 0 { work.x + (work.w - r.w) / 2 } else { r.x };
        let y = if r.y < 0 { work.y + (work.h - r.h) / 2 } else { r.y };
        Ok((Rect::new(x, y, r.w, r.h), centred))
    }

    /// One request in, one reply out. `Ok(false)` means QUIT was received.
    fn handle(&mut self, op: u8, payload: &[u8], out: &mut Out) -> Res<bool> {
        let mut r = In { b: payload, p: 0 };
        match op {
            op::QUIT => return Ok(false),

            // Only a host with a window answers these; `call` is the core
            // behind any host, the browser's among them.
            op::OPEN_WINDOW => return Err("this host has no window".into()),
            op::WAIT => return Err("WAIT is for a window: send OPEN_WINDOW first".into()),

            op::INIT => {
                let (w, h) = (r.i16("w")?, r.i16("h")?);
                if w < 1 || h < 1 {
                    return Err(format!("cannot make a {w}x{h} screen"));
                }
                // An optional code page; absent or 0 is 437.
                if r.remaining() >= 2 {
                    let n = r.u16("codepage")?;
                    if n != 0 {
                        self.cp = Font::growing(n).ok_or_else(|| format!("no code page {n}"))?;
                    }
                }
                self.ui = Some(Ui::new(w, h));
                self.status = None;
                self.menu = None;
            }

            op::CODEPAGE => {
                let n = r.u16("codepage")?;
                self.cp = Font::growing(n).ok_or_else(|| format!("no code page {n}"))?;
            }

            op::GET_GLYPHS => {
                // What each glyph index looks like, as Unicode, so a client
                // draws with the server's table and never keeps one of its
                // own. The table grows; a FRAME says how long it is now.
                out.u8(self.cp.growing as u8);
                out.u16(self.cp.len() as u16);
                for &c in self.cp.table.iter() {
                    let u = c as u32;
                    out.u16(if u <= 0xFFFF { u as u16 } else { b'?' as u16 });
                }
            }
            op::RESIZE => {
                let (w, h) = (r.i16("w")?, r.i16("h")?);
                self.ui()?.handle(Event::Resize(w, h));
            }

            op::WINDOW => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let flags = r.u8("flags")?;
                let title = self.cp.to_core(&r.str("title")?);
                // Added after the first clients were written, so it is
                // allowed to be absent: no field, no command.
                let close_cmd = if r.remaining() >= 2 { r.u16("close_cmd")? } else { 0 };
                let parent = self.alive(parent)?;
                let (rect, centred) = self.place(rect)?;
                let mut w = Window::new(&title);
                w.centred = centred;
                w.close_cmd = close_cmd;
                w.modal = flags & 0x01 != 0;
                w.resizable = flags & 0x02 == 0;
                w.zoomable = flags & 0x04 == 0;
                w.closable = flags & 0x08 == 0;
                // No shadow: two panels side by side would each cast one
                // on the other, and a shadow on a window that is really
                // half the screen says nothing about depth.
                w.shadow = flags & 0x40 == 0;
                w.palette = match (flags >> 4) & 3 {
                    1 => WinPalette::Cyan,
                    2 => WinPalette::Gray,
                    _ => WinPalette::Blue,
                };
                if !w.resizable {
                    w.min_w = rect.w;
                    w.max_w = rect.w;
                    w.min_h = rect.h;
                    w.max_h = rect.h;
                }
                let id = self.ui()?.insert(parent, rect, Kind::Window(w));
                out.id(id);
            }

            op::TEXT => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let dock = r.u8("dock")?;
                let flags = r.u8("flags")?;
                let text = r.str("text")?;
                let parent = self.alive(parent)?;
                let mut t = TextView::new(lines_of(&mut self.cp, &text));
                t.readonly = flags & 1 != 0;
                t.boxed = flags & 2 != 0;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Text(t));
                ui.set_dock(id, if dock == 1 { Dock::Manual } else { Dock::Fill });
                self.settle_focus()?;
                out.id(id);
            }

            op::TREE => {
                // A tree filling its window. A node marked lazy is drawn as
                // a branch and, when opened, asks the program for its
                // children through TREE_EXPAND: a disk is not read whole.
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let nodes = r.tree_nodes(&mut self.cp, 0)?;
                let parent = self.alive(parent)?;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Tree(owlosui_core::TreeView::new(nodes)));
                ui.set_dock(id, Dock::Fill);
                self.settle_focus()?;
                out.id(id);
            }

            op::TREE_CHILDREN => {
                let id = r.id("id")?;
                let path = r.tree_path()?;
                let nodes = r.tree_nodes(&mut self.cp, 0)?;
                let id = self.alive(id)?;
                if !self.ui()?.tree_set_children(id, &path, nodes) {
                    return Err(format!("view {} has no node at that path", id.raw()));
                }
            }

            op::TREE_EXPAND => {
                // The node waiting for children, if one is: its path as
                // indices, each with the node's text, so the program can
                // tell which folder it is without a copy of the tree.
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let ui = self.ui()?;
                match ui.tree_take_expand(id) {
                    Some(path) => {
                        let texts = ui.tree_texts(id, &path);
                        out.u8(path.len() as u8);
                        for (i, ix) in path.iter().enumerate() {
                            out.u16(*ix as u16);
                            out.str(&self.cp.from_core(texts.get(i).map(|s| s.as_str()).unwrap_or("")));
                        }
                    }
                    None => out.u8(0),
                }
            }

            op::TREE_PATH => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let texts = self.ui()?.tree_path(id);
                out.u8(texts.len() as u8);
                for t in &texts {
                    out.str(&self.cp.from_core(t));
                }
            }

            op::FIND => {
                // Flags: bit 0 case-sensitive, bit 1 whole words. The next
                // match after the caret is selected; the reply says whether
                // there was one. No wrapping: that is a question for a
                // dialog to ask.
                let id = r.id("id")?;
                let flags = r.u8("flags")?;
                let pat = owlosui_core::glyphs(&self.cp.to_core(&r.str("pattern")?));
                let id = self.alive(id)?;
                let ui = self.ui()?;
                if !matches!(ui.kind(id), Kind::Text(_)) {
                    return Err(format!("view {} is not a text", id.raw()));
                }
                let found = ui.find_text(id, &pat, flags & 1 != 0, flags & 2 != 0);
                out.u8(found as u8);
            }

            op::REPLACE => {
                let id = r.id("id")?;
                let flags = r.u8("flags")?;
                let pat = owlosui_core::glyphs(&self.cp.to_core(&r.str("pattern")?));
                let with = owlosui_core::glyphs(&self.cp.to_core(&r.str("replacement")?));
                let id = self.alive(id)?;
                let ui = self.ui()?;
                if !matches!(ui.kind(id), Kind::Text(_)) {
                    return Err(format!("view {} is not a text", id.raw()));
                }
                let (replaced, found) = ui.replace_text(id, &pat, &with, flags & 1 != 0, flags & 2 != 0);
                out.u8(replaced as u8);
                out.u8(found as u8);
            }

            op::REPLACE_ALL => {
                let id = r.id("id")?;
                let flags = r.u8("flags")?;
                let pat = owlosui_core::glyphs(&self.cp.to_core(&r.str("pattern")?));
                let with = owlosui_core::glyphs(&self.cp.to_core(&r.str("replacement")?));
                let id = self.alive(id)?;
                let ui = self.ui()?;
                if !matches!(ui.kind(id), Kind::Text(_)) {
                    return Err(format!("view {} is not a text", id.raw()));
                }
                let n = ui.replace_all_text(id, &pat, &with, flags & 1 != 0, flags & 2 != 0);
                out.u16(n);
            }

            op::HEX => {
                // A hex dump of the bytes that follow, filling its window.
                // The wire's payload is at most 64K, so that is the most a
                // dump can hold; a program with a bigger file sends the
                // part it wants seen.
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let bytes = r.rest().to_vec();
                let parent = self.alive(parent)?;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Hex(owlosui_core::HexView::new(bytes)));
                ui.set_dock(id, Dock::Fill);
                self.settle_focus()?;
                out.id(id);
            }

            op::SET_READONLY => {
                let id = r.id("id")?;
                let on = r.u8("readonly")? != 0;
                let id = self.alive(id)?;
                if !self.ui()?.set_readonly(id, on) {
                    return Err(format!("view {} is not a text", id.raw()));
                }
                self.settle_focus()?;
            }

            op::EDITOR => {
                // What the editor offers on the bars, and how it is set:
                // the core then answers its Edit menu, its keys and its
                // Find and Replace dialogs by itself.
                let id = r.id("id")?;
                let offers = r.u8("offers")?;
                let state = r.u8("state")?;
                let id = self.alive(id)?;
                if !self.ui()?.set_editor(id, offers, state) {
                    return Err(format!("view {} is not a text", id.raw()));
                }
                self.settle_focus()?;
            }

            op::GET_EDITOR => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let Some((offers, state, line, col)) = self.ui()?.editor_state(id) else {
                    return Err(format!("view {} is not a text", id.raw()));
                };
                out.u8(offers);
                out.u8(state);
                out.u16(line.max(0) as u16);
                out.u16(col.max(0) as u16);
            }

            op::STATIC => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let text = self.cp.to_core(&r.str("text")?);
                let parent = self.alive(parent)?;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Static(StaticText::new(&text)));
                ui.set_dock(id, Dock::Manual);
                out.id(id);
            }

            op::INPUT => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let max = r.u16("max")?;
                let label = self.cp.to_core(&r.str("label")?);
                let text = self.cp.to_core(&r.str("text")?);
                let parent = self.alive(parent)?;
                let mut i = InputLine::new(&label, &text);
                if max > 0 {
                    i.max = max as usize;
                }
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Input(i));
                ui.set_dock(id, Dock::Manual);
                self.settle_focus()?;
                out.id(id);
            }

            op::BUTTONS => {
                let parent = r.id("parent")?;
                let row = r.buttons(&mut self.cp)?;
                let parent = self.alive(parent)?;
                // Bottom right, with room for the last button's shadow. The
                // row is two rows tall for the same reason.
                let w = row.width() + 3;
                let ui = self.ui()?;
                let id = ui.insert(parent, Rect::default(), Kind::Buttons(row));
                ui.set_dock(id, Dock::BottomRight(w, 2));
                ui.focus_first();
                out.id(id);
            }

            op::MESSAGE_BOX => {
                let title = self.cp.to_core(&r.str("title")?);
                let text = self.cp.to_core(&r.str("text")?);
                let row = r.buttons(&mut self.cp)?;
                let id = self.ui()?.message_box(&title, &text, row);
                out.id(id);
            }

            op::CLOSE => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                self.ui()?.close(id);
            }

            op::GET_TEXT => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let cp = &self.cp;
                let s = match self.ui.as_ref().ok_or("INIT first")?.kind(id) {
                    Kind::Text(t) => t
                        .lines
                        .iter()
                        .map(|l| cp.decode(l))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    Kind::Input(i) => cp.from_core(&i.text),
                    _ => return Err(format!("view {} has no text", id.raw())),
                };
                out.str(&s);
            }

            op::KEY => {
                let kind = r.u8("kind")?;
                let value = r.u16("value")?;
                let m = r.u8("mods")?;
                let mut key = decode_key(kind, value, m)?;
                // A typed character goes in as its glyph index, like every
                // other string; a character the page has no glyph for is
                // typed as `?`, which is what the screen would show anyway.
                if let KeyCode::Char(c) = key.code {
                    if (c as u32) >= 128 {
                        let g = self.cp.from_char(c);
                        key.code = KeyCode::Char(char::from_u32(g as u32).unwrap_or('?'));
                    }
                }
                self.ui()?.handle(Event::Key(key));
            }

            op::STATUS => {
                let items = r.status_items(&mut self.cp)?;
                let old = self.status.take();
                let ui = self.ui()?;
                if let Some(old) = old {
                    if ui.is_alive(old) {
                        ui.close(old);
                    }
                }
                let root = ui.root();
                let screen = ui.rect(root);
                let id = ui.insert(
                    root,
                    Rect::new(0, screen.h - 1, screen.w, 1),
                    Kind::Status(StatusLine::new(items)),
                );
                self.status = Some(id);
                out.id(id);
            }

            op::LABEL => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let target = r.u16("target")?;
                let text = self.cp.to_core(&r.str("text")?);
                let parent = self.alive(parent)?;
                let target = if target == 0 {
                    None
                } else {
                    Some(self.alive(ViewId::from_raw(target as u32))?)
                };
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Label(Label::new(&text, target)));
                ui.set_dock(id, Dock::Manual);
                out.id(id);
            }

            op::PROGRESS => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let max = r.u32("max")?;
                let flags = r.u8("flags")?;
                let parent = self.alive(parent)?;
                let mut p = Progress::new(max);
                p.percent = flags & 1 != 0;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Progress(p));
                ui.set_dock(id, Dock::Manual);
                out.id(id);
            }

            op::SET_PROGRESS => {
                let id = r.id("id")?;
                let value = r.u32("value")?;
                let id = self.alive(id)?;
                match self.ui()?.kind_mut(id) {
                    Kind::Progress(p) => p.set(value),
                    _ => return Err(format!("view {} is not a progress bar", id.raw())),
                }
            }

            op::LIST => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let flags = r.u8("flags")?;
                let n = r.u16("item count")?;
                let mut items = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    items.push(self.cp.to_core(&r.str("item")?));
                }
                let parent = self.alive(parent)?;
                let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                let mut l = ListBox::new(&refs);
                l.multi = flags & 1 != 0;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::List(l));
                ui.set_dock(id, Dock::Manual);
                self.settle_focus()?;
                out.id(id);
            }

            op::FILES => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let flags = r.u8("flags")?;
                let mask = self.cp.to_core(&r.str("mask")?);
                let path = self.cp.to_core(&r.str("path")?);
                let entries = r.entries(&mut self.cp)?;
                let parent = self.alive(parent)?;
                let mut f = FileList::new(entries, &mask);
                f.set_path(&path);
                f.multi = flags & 1 != 0;
                // A bare path line, without `Path:` in front of it. An Open
                // dialog wants the word; a file manager's panel is nothing
                // but paths and does not.
                if flags & 2 != 0 {
                    f.path.label = String::new();
                }
                // No path line at all: a commander's panel, where the names
                // start at the top and the foot says where you are.
                if flags & 4 != 0 {
                    f.path_line = false;
                }
                f.focus = owlosui_core::files::Focus::List;
                let ui = self.ui()?;
                // The panel fills the window; `rect.y` is how many rows to
                // leave free above it, for the line of air an Open dialog
                // wants under its title.
                let top = rect.y.max(0);
                let id = ui.insert(parent, rect, Kind::Files(f));
                ui.set_dock(id, if top > 0 { Dock::FillFrom(top) } else { Dock::Fill });
                out.id(id);
            }

            op::SET_FILES => {
                let id = r.id("id")?;
                let path = self.cp.to_core(&r.str("path")?);
                let mask = self.cp.to_core(&r.str("mask")?);
                let entries = r.entries(&mut self.cp)?;
                let id = self.alive(id)?;
                match self.ui()?.kind_mut(id) {
                    Kind::Files(f) => {
                        f.set_mask(&mask);
                        f.set_entries(entries);
                        f.set_path(&path);
                        f.error = None;
                    }
                    _ => return Err(format!("view {} is not a file panel", id.raw())),
                }
            }

            op::ADD_FILES => {
                // The rest of a listing that did not fit in one request:
                // a request is at most 64K, and a folder of a few thousand
                // names is more than that.
                let id = r.id("id")?;
                let entries = r.entries(&mut self.cp)?;
                let id = self.alive(id)?;
                match self.ui()?.kind_mut(id) {
                    Kind::Files(f) => f.add_entries(entries),
                    _ => return Err(format!("view {} is not a file panel", id.raw())),
                }
            }

            op::SET_FILES_ERROR => {
                let id = r.id("id")?;
                let text = self.cp.to_core(&r.str("text")?);
                let id = self.alive(id)?;
                match self.ui()?.kind_mut(id) {
                    Kind::Files(f) => f.error = Some(text),
                    _ => return Err(format!("view {} is not a file panel", id.raw())),
                }
            }

            op::TAKE_FILES => {
                // What the person did in the panel since last asked: typed a
                // path and pressed Enter (2), or pressed Enter on a name (1).
                // Read once, like TAKE.
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let (kind, text) = match self.ui()?.kind_mut(id) {
                    Kind::Files(f) => {
                        if let Some(p) = f.pending_path.take() {
                            (2u8, p)
                        } else if let Some(n) = f.chosen.take() {
                            (1u8, n)
                        } else {
                            (0u8, String::new())
                        }
                    }
                    _ => return Err(format!("view {} is not a file panel", id.raw())),
                };
                out.u8(kind);
                out.str(&self.cp.from_core(&text));
            }

            op::MARKED_NAMES => {
                // The marked files - or, with none marked, the one under the
                // cursor, which is what F5 in Norton Commander copied.
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let names = match self.ui()?.kind(id) {
                    Kind::Files(f) => {
                        let m = f.marked_names();
                        if m.is_empty() {
                            f.selected().map(|e| vec![e.name.clone()]).unwrap_or_default()
                        } else {
                            m
                        }
                    }
                    _ => return Err(format!("view {} is not a file panel", id.raw())),
                };
                out.u16(names.len() as u16);
                for n in &names {
                    out.str(&self.cp.from_core(n));
                }
            }

            op::ACTIVATE => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let ui = self.ui()?;
                if !matches!(ui.kind(id), Kind::Window(_)) {
                    return Err(format!("view {} is not a window", id.raw()));
                }
                ui.activate(id);
            }

            op::ACTIVE => {
                out.u16(self.ui()?.active_window().map_or(0, |w| w.raw() as u16));
            }

            op::MENU_BAR => {
                // The bar across the top, one per desktop; a second replaces
                // the first. Its items send commands through TAKE.
                let items = r.menu_items(&mut self.cp, 0)?;
                let old = self.menu.take();
                let ui = self.ui()?;
                if let Some(old) = old {
                    if ui.is_alive(old) {
                        ui.close(old);
                    }
                }
                let root = ui.root();
                let screen = ui.rect(root);
                let id = ui.insert(root, Rect::new(0, 0, screen.w, 1), Kind::MenuBar(MenuBar::new(items)));
                self.menu = Some(id);
                out.id(id);
            }

            op::WINDOW_STATUS => {
                // The keys a window carries: shown on the status line and
                // bound while the window is the active one, gone when it
                // is not. Same items as STATUS.
                let id = r.id("id")?;
                let items = r.status_items(&mut self.cp)?;
                let id = self.alive(id)?;
                if !self.ui()?.set_window_status(id, items) {
                    return Err(format!("view {} is not a window", id.raw()));
                }
            }

            op::WINDOW_MENU => {
                // The menus a window carries, merged into the bar while it
                // is active: a submenu named like one on the bar goes into
                // that one, after a line; any other goes on the end.
                let id = r.id("id")?;
                let items = r.menu_items(&mut self.cp, 0)?;
                let id = self.alive(id)?;
                if !self.ui()?.set_window_menu(id, items) {
                    return Err(format!("view {} is not a window", id.raw()));
                }
            }

            op::MENU_CHECK => {
                let cmd = r.u16("cmd")?;
                let on = r.u8("on")? != 0;
                if !self.ui()?.set_menu_checked(cmd, on) {
                    return Err(format!("no menu item sends {cmd}"));
                }
            }

            op::CLUSTER => {
                // Check boxes (any number on) or radio buttons (exactly one).
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let single = r.u8("kind")? != 0;
                let n = r.u8("item count")?;
                let mut labels = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    labels.push(self.cp.to_core(&r.str("label")?));
                }
                let parent = self.alive(parent)?;
                let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                let mut c = Cluster::checks(&refs);
                if single {
                    c.mode = Choice::One;
                    for (i, on) in c.on.iter_mut().enumerate() {
                        *on = i == 0;
                    }
                }
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Cluster(c));
                ui.set_dock(id, Dock::Manual);
                self.settle_focus()?;
                out.id(id);
            }

            op::GET_CLUSTER => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                match self.ui()?.kind(id) {
                    Kind::Cluster(c) => {
                        out.u8(c.on.len() as u8);
                        for &on in &c.on {
                            out.u8(on as u8);
                        }
                        out.u8(c.current as u8);
                    }
                    _ => return Err(format!("view {} is not a cluster", id.raw())),
                }
            }

            op::GET_CLICK => {
                // Where the mouse last went down on a canvas, once.
                let id = r.id("id")?;
                let id = self.alive(id)?;
                match self.ui()?.kind_mut(id) {
                    Kind::Canvas(c) => match c.clicked.take() {
                        Some((x, y)) => {
                            out.u8(1);
                            out.i16(x);
                            out.i16(y);
                        }
                        None => {
                            out.u8(0);
                            out.i16(0);
                            out.i16(0);
                        }
                    },
                    _ => return Err(format!("view {} is not a canvas", id.raw())),
                }
            }

            op::CYCLE => {
                self.ui()?.cycle_windows();
            }

            op::CASCADE => {
                self.ui()?.cascade();
            }

            op::WINDOW_LIST => {
                self.ui()?.window_list();
            }

            op::CYCLE_BACK => {
                self.ui()?.cycle_windows_back();
            }

            op::SIZE_MOVE => {
                self.ui()?.begin_size_move();
            }

            op::SET_HISTORY => {
                // What an input line has been given before, newest first;
                // Down or the ▼ at its end lists them.
                let id = r.id("id")?;
                let n = r.u8("count")?;
                let mut items = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    items.push(self.cp.to_core(&r.str("history.item")?));
                }
                let id = self.alive(id)?;
                if !self.ui()?.set_history(id, items) {
                    return Err(format!("view {} is not an input line", id.raw()));
                }
            }

            op::PALETTE => {
                // Every colour by group and name, in the order SET_COLOR
                // indexes: what a colour dialog lists.
                let ui = self.ui()?;
                let names = owlosui_core::Palette::NAMES;
                out.u8(names.len() as u8);
                for (ix, (group, name)) in names.iter().enumerate() {
                    out.str(group);
                    out.str(name);
                    out.u8(ui.palette.get(ix).unwrap_or(0));
                }
            }

            op::SET_COLOR => {
                // One colour changed, and the next frame wears it: the
                // palette is looked up at every draw, never copied.
                let ix = r.u8("index")? as usize;
                let attr = r.u8("attr")?;
                if !self.ui()?.palette.set(ix, attr) {
                    return Err(format!("no palette entry {ix}"));
                }
            }

            op::GET_HISTORY => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let items = self.ui()?.history(id);
                out.u8(items.len().min(255) as u8);
                for it in items.iter().take(255) {
                    out.str(&self.cp.from_core(it));
                }
            }

            op::TILE => {
                self.ui()?.tile();
            }

            op::ZOOM => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let ui = self.ui()?;
                if !matches!(ui.kind(id), Kind::Window(_)) {
                    return Err(format!("view {} is not a window", id.raw()));
                }
                ui.toggle_zoom(id);
            }

            op::BUTTON_ROW => {
                // A row of buttons placed by hand, from the left: one row
                // of a keypad. Enter still finds a default button in it,
                // and a click presses without taking the focus.
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                // Bit 0: not a Tab stop - a keypad is for the mouse and
                // for Enter, and typing goes on past it.
                let flags = r.u8("row.flags")?;
                let mut row = r.buttons(&mut self.cp)?;
                row.align = Align::Left;
                row.selectable = flags & 1 == 0;
                let parent = self.alive(parent)?;
                let width = row.width();
                let ui = self.ui()?;
                // One column more than the buttons need, for the shadow of
                // the last one; the row's own clip would cut it off.
                let rect = Rect::new(rect.x, rect.y, if rect.w > 0 { rect.w } else { width + 1 }, 2);
                let id = ui.insert(parent, rect, Kind::Buttons(row));
                ui.set_dock(id, Dock::Manual);
                self.settle_focus()?;
                out.id(id);
            }

            op::FOCUS => {
                // Put the focus on one control: a calculator hands the
                // caret back to its display once a mode has been chosen.
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let ui = self.ui()?;
                let Some(win) = ui.window_of(id) else {
                    return Err(format!("view {} is not in a window", id.raw()));
                };
                ui.focus_on(win, id);
            }

            op::SET_BUTTON => {
                let id = r.id("id")?;
                let ix = r.u8("index")? as usize;
                let on = r.u8("enabled")? != 0;
                let id = self.alive(id)?;
                if !self.ui()?.set_button_enabled(id, ix, on) {
                    return Err(format!("view {} has no button {ix}", id.raw()));
                }
            }

            op::CANVAS => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let parent = self.alive(parent)?;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Canvas(Canvas::new(rect.w, rect.h)));
                ui.set_dock(id, Dock::Manual);
                out.id(id);
            }

            op::BLIT => {
                // Cells as characters with attributes; the font turns each
                // character into a glyph, growing if it has to.
                let id = r.id("id")?;
                let (x, y, w, h) = (r.i16("x")?, r.i16("y")?, r.i16("w")?, r.i16("h")?);
                if w < 0 || h < 0 {
                    return Err("a block with a negative size".into());
                }
                let n = (w as usize) * (h as usize);
                let mut cells = Vec::with_capacity(n);
                for _ in 0..n {
                    let ch = r.u16("cell.ch")?;
                    let attr = r.u8("cell.attr")?;
                    let c = char::from_u32(ch as u32).unwrap_or('?');
                    cells.push(Cell::new(self.cp.cell_glyph(c), attr));
                }
                let id = self.alive(id)?;
                match self.ui()?.kind_mut(id) {
                    Kind::Canvas(c) => c.blit(x, y, w, h, &cells),
                    _ => return Err(format!("view {} is not a canvas", id.raw())),
                }
            }

            op::SET_TEXT => {
                // New words for something that already has some. A static
                // keeps its place and width; an input keeps its label; an
                // editor starts over with the new lines.
                let id = r.id("id")?;
                let text = r.str("text")?;
                let core = self.cp.to_core(&text);
                let lines = lines_of(&mut self.cp, &text);
                let id = self.alive(id)?;
                match self.ui()?.kind_mut(id) {
                    Kind::Static(t) => t.text = core,
                    Kind::Input(i) => i.set_text(&core),
                    Kind::Text(t) => {
                        *t = {
                            let mut n = TextView::new(lines);
                            n.readonly = t.readonly;
                            n.boxed = t.boxed;
                            n.focused = t.focused;
                            n
                        };
                    }
                    _ => return Err(format!("view {} has no text to set", id.raw())),
                }
            }

            op::GET_MARKED => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let marked = match self.ui()?.kind(id) {
                    Kind::List(l) => l.marked(),
                    _ => return Err(format!("view {} is not a list", id.raw())),
                };
                out.u16(marked.len() as u16);
                for m in marked {
                    out.u16(m as u16);
                }
            }

            op::GET_CURRENT => {
                let id = r.id("id")?;
                let id = self.alive(id)?;
                let cur = match self.ui()?.kind(id) {
                    Kind::List(l) => l.current,
                    _ => return Err(format!("view {} is not a list", id.raw())),
                };
                out.u16(cur as u16);
            }

            op::MOUSE => {
                let kind = r.u8("kind")?;
                let button = r.u8("button")?;
                let (x, y) = (r.i16("x")?, r.i16("y")?);
                let button = match button {
                    0 => Button::Left,
                    1 => Button::Right,
                    2 => Button::Middle,
                    _ => return Err(format!("no mouse button {button}")),
                };
                let kind = match kind {
                    0 => MouseKind::Down(button),
                    1 => MouseKind::Up(button),
                    2 => MouseKind::Drag,
                    3 => MouseKind::Move,
                    4 => MouseKind::ScrollUp,
                    5 => MouseKind::ScrollDown,
                    6 => MouseKind::Double(button),
                    _ => return Err(format!("no mouse kind {kind}")),
                };
                self.ui()?.handle(Event::Mouse(Mouse { x, y, kind }));
            }

            op::TICK => {
                self.ui()?.complete_pick();
            }

            op::FRAME => {
                let font_len = self.cp.len() as u16;
                let ui = self.ui()?;
                let root = ui.root();
                let r = ui.rect(root);
                let mut buf = owlosui_core::Buffer::new(r.w, r.h);
                ui.draw(&mut buf);
                let cur = ui.cursor();
                out.i16(r.w);
                out.i16(r.h);
                out.i16(cur.map_or(-1, |p| p.x));
                out.i16(cur.map_or(-1, |p| p.y));
                // "Show this one for a moment, then send TICK": a button
                // that a key just put down, a menu item just chosen.
                out.u8(ui.pick_pending() as u8);
                // How long the font is now: a client whose table is shorter
                // fetches it again before it draws.
                out.u16(font_len);
                // A cell on the wire is `glyph:u16 attr:u8`: three bytes,
                // because the font can be longer than 256.
                for y in 0..r.h {
                    for x in 0..r.w {
                        let (ch, attr) = buf.get(x, y).map_or((b' ' as u16, 0), |c| (c.ch as u16, c.attr));
                        out.u16(ch);
                        out.u8(attr);
                    }
                }
            }

            op::TAKE => {
                let ui = self.ui()?;
                out.u16(ui.take_pressed().unwrap_or(0));
                out.u16(ui.take_command().unwrap_or(0));
            }

            _ => return Err(format!("no op {op:#04x}")),
        }
        Ok(true)
    }
}

fn lines_of(cp: &mut Font, text: &str) -> Vec<Vec<owlosui_core::Glyph>> {
    let mut v: Vec<Vec<owlosui_core::Glyph>> = text.split('\n').map(|l| cp.encode(l)).collect();
    if v.is_empty() {
        v.push(Vec::new());
    }
    v
}

fn decode_key(kind: u8, value: u16, m: u8) -> Res<Key> {
    let code = match kind {
        0 => KeyCode::Char(char::from_u32(value as u32).ok_or("not a character")?),
        1 => KeyCode::F(value as u8),
        2 => named_key(value).ok_or_else(|| format!("no named key {value}"))?,
        _ => return Err(format!("no key kind {kind}")),
    };
    let mods = Mods {
        shift: m & 1 != 0,
        ctrl: m & 2 != 0,
        alt: m & 4 != 0,
    };
    Ok(Key { code, mods })
}

fn named_key(v: u16) -> Option<KeyCode> {
    Some(match v {
        0 => KeyCode::Enter,
        1 => KeyCode::Esc,
        2 => KeyCode::Tab,
        3 => KeyCode::BackTab,
        4 => KeyCode::Backspace,
        5 => KeyCode::Delete,
        6 => KeyCode::Insert,
        7 => KeyCode::Home,
        8 => KeyCode::End,
        9 => KeyCode::PageUp,
        10 => KeyCode::PageDown,
        11 => KeyCode::Up,
        12 => KeyCode::Down,
        13 => KeyCode::Left,
        14 => KeyCode::Right,
        _ => return None,
    })
}
