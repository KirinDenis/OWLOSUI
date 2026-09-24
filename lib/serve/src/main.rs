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

use std::io::{Read, Write};

use owlosui_console::codepage::CodePage;
use owlosui_core::{
    Button, ButtonRow, Dock, Event, FileEntry, FileList, InputLine, Key, KeyCode, Kind, Label,
    ListBox, Mods, Mouse, MouseKind, Progress, PushButton, Rect, StaticText, StatusItem, StatusLine,
    TextView, Ui, ViewId, WinPalette, Window,
};

mod op {
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
    pub const ACTIVE: u8 = 0x2A;
    pub const KEY: u8 = 0x30;
    pub const MOUSE: u8 = 0x31;
    pub const TICK: u8 = 0x32;
    pub const FRAME: u8 = 0x40;
    pub const TAKE: u8 = 0x41;
    pub const GET_GLYPHS: u8 = 0x42;
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
    fn str(&mut self, what: &str) -> Res<String> {
        let n = self.u16(what)? as usize;
        let s = self.take(n, what)?;
        String::from_utf8(s.to_vec()).map_err(|_| format!("{what} is not UTF-8"))
    }
    /// `n:u16` then `n ×` (`name:str size:u32 year:u16 month:u8 day:u8
    /// hour:u8 minute:u8 attrs:u8`): a directory listing, read by the
    /// client, because the server reads no files and no directories.
    fn entries(&mut self, cp: &CodePage) -> Res<Vec<FileEntry>> {
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

    /// `n ×` (`cmd:u16 flags:u8 label:str`) — the shape both BUTTONS and
    /// MESSAGE_BOX carry.
    fn buttons(&mut self, cp: &CodePage) -> Res<ButtonRow> {
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

struct Server {
    ui: Option<Ui>,
    /// The status line, if one has been made. A second one replaces it.
    status: Option<ViewId>,
    /// The code page every string is turned into on the way in and back
    /// on the way out. One per session, like the font in a video card.
    cp: CodePage,
}

impl Server {
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

            op::INIT => {
                let (w, h) = (r.i16("w")?, r.i16("h")?);
                if w < 1 || h < 1 {
                    return Err(format!("cannot make a {w}x{h} screen"));
                }
                // An optional code page; absent or 0 is 437.
                if r.remaining() >= 2 {
                    let n = r.u16("codepage")?;
                    if n != 0 {
                        self.cp = CodePage::by_number(n).ok_or_else(|| format!("no code page {n}"))?;
                    }
                }
                self.ui = Some(Ui::new(w, h));
                self.status = None;
            }

            op::CODEPAGE => {
                let n = r.u16("codepage")?;
                self.cp = CodePage::by_number(n).ok_or_else(|| format!("no code page {n}"))?;
            }

            op::GET_GLYPHS => {
                // What each of the 256 glyph indices looks like, as Unicode,
                // so a client draws with the server's table and never keeps
                // one of its own.
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
                let mut t = TextView::new(lines_of(&self.cp, &text));
                t.readonly = flags & 1 != 0;
                t.boxed = flags & 2 != 0;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Text(t));
                ui.set_dock(id, if dock == 1 { Dock::Manual } else { Dock::Fill });
                self.settle_focus()?;
                out.id(id);
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
                let row = r.buttons(&self.cp)?;
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
                let row = r.buttons(&self.cp)?;
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
                        key.code = KeyCode::Char(self.cp.from_char(c) as char);
                    }
                }
                self.ui()?.handle(Event::Key(key));
            }

            op::STATUS => {
                let n = r.u8("item count")?;
                let mut items = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    let cmd = r.u16("status.cmd")?;
                    let key = r.key()?;
                    let text = self.cp.to_core(&r.str("status.text")?);
                    items.push(StatusItem::new(&text, key, cmd));
                }
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
                let entries = r.entries(&self.cp)?;
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
                let id = ui.insert(parent, rect, Kind::Files(f));
                ui.set_dock(id, Dock::Fill);
                out.id(id);
            }

            op::SET_FILES => {
                let id = r.id("id")?;
                let path = self.cp.to_core(&r.str("path")?);
                let mask = self.cp.to_core(&r.str("mask")?);
                let entries = r.entries(&self.cp)?;
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
                    _ => return Err(format!("no mouse kind {kind}")),
                };
                self.ui()?.handle(Event::Mouse(Mouse { x, y, kind }));
            }

            op::TICK => {
                self.ui()?.complete_pick();
            }

            op::FRAME => {
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
                for y in 0..r.h {
                    for x in 0..r.w {
                        let (ch, attr) = buf.get(x, y).map_or((b' ', 0), |c| (c.ch, c.attr));
                        out.u8(ch);
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

fn lines_of(cp: &CodePage, text: &str) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = text.split('\n').map(|l| cp.encode(l)).collect();
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

fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = std::io::BufWriter::new(stdout.lock());
    let mut server = Server {
        ui: None,
        status: None,
        cp: CodePage::by_number(437).expect("437 is built in"),
    };

    loop {
        // op:u8 len:u16, then the payload. End of input is a normal way for a
        // client to leave: it closed the pipe, and so do we.
        let mut head = [0u8; 3];
        if input.read_exact(&mut head).is_err() {
            return Ok(());
        }
        let op = head[0];
        let len = u16::from_le_bytes([head[1], head[2]]) as usize;
        let mut payload = vec![0u8; len];
        input.read_exact(&mut payload)?;

        let mut out = Out::default();
        let (status, keep_going) = match server.handle(op, &payload, &mut out) {
            Ok(going) => (0u8, going),
            Err(e) => {
                out = Out::default();
                out.str(&e);
                (1u8, true)
            }
        };

        output.write_all(&[status])?;
        output.write_all(&(out.0.len() as u16).to_le_bytes())?;
        output.write_all(&out.0)?;
        output.flush()?;

        if !keep_going {
            return Ok(());
        }
    }
}
