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

use owlosui_console::cp437;
use owlosui_core::{
    Button, ButtonRow, Dock, Event, InputLine, Key, KeyCode, Kind, Mods, Mouse, MouseKind,
    PushButton, Rect, StaticText, TextView, Ui, ViewId, WinPalette, Window,
};

mod op {
    pub const QUIT: u8 = 0x00;
    pub const INIT: u8 = 0x01;
    pub const RESIZE: u8 = 0x02;
    pub const WINDOW: u8 = 0x10;
    pub const TEXT: u8 = 0x11;
    pub const STATIC: u8 = 0x12;
    pub const INPUT: u8 = 0x13;
    pub const BUTTONS: u8 = 0x14;
    pub const MESSAGE_BOX: u8 = 0x15;
    pub const CLOSE: u8 = 0x20;
    pub const GET_TEXT: u8 = 0x21;
    pub const KEY: u8 = 0x30;
    pub const MOUSE: u8 = 0x31;
    pub const TICK: u8 = 0x32;
    pub const FRAME: u8 = 0x40;
    pub const TAKE: u8 = 0x41;
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
    /// `n ×` (`cmd:u16 flags:u8 label:str`) — the shape both BUTTONS and
    /// MESSAGE_BOX carry.
    fn buttons(&mut self) -> Res<ButtonRow> {
        let n = self.u8("button count")?;
        let mut v = Vec::with_capacity(n as usize);
        for i in 0..n {
            let cmd = self.u16("button.cmd")?;
            if cmd == 0 {
                return Err(format!("button {i} has command 0, which means none"));
            }
            let flags = self.u8("button.flags")?;
            let label = self.str("button.label")?;
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
                self.ui = Some(Ui::new(w, h));
            }
            op::RESIZE => {
                let (w, h) = (r.i16("w")?, r.i16("h")?);
                self.ui()?.handle(Event::Resize(w, h));
            }

            op::WINDOW => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let flags = r.u8("flags")?;
                let title = r.str("title")?;
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
                let mut t = TextView::new(lines_of(&text));
                t.readonly = flags & 1 != 0;
                t.boxed = flags & 2 != 0;
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Text(t));
                ui.set_dock(id, if dock == 1 { Dock::Manual } else { Dock::Fill });
                out.id(id);
            }

            op::STATIC => {
                let parent = r.id("parent")?;
                let rect = r.rect()?;
                let text = r.str("text")?;
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
                let label = r.str("label")?;
                let text = r.str("text")?;
                let parent = self.alive(parent)?;
                let mut i = InputLine::new(&label, &text);
                if max > 0 {
                    i.max = max as usize;
                }
                let ui = self.ui()?;
                let id = ui.insert(parent, rect, Kind::Input(i));
                ui.set_dock(id, Dock::Manual);
                out.id(id);
            }

            op::BUTTONS => {
                let parent = r.id("parent")?;
                let row = r.buttons()?;
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
                let title = r.str("title")?;
                let text = r.str("text")?;
                let row = r.buttons()?;
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
                let s = match self.ui()?.kind(id) {
                    Kind::Text(t) => t
                        .lines
                        .iter()
                        .map(|l| l.iter().map(|&b| cp437::to_char(b)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    Kind::Input(i) => i.text.clone(),
                    _ => return Err(format!("view {} has no text", id.raw())),
                };
                out.str(&s);
            }

            op::KEY => {
                let kind = r.u8("kind")?;
                let value = r.u16("value")?;
                let m = r.u8("mods")?;
                let code = match kind {
                    0 => KeyCode::Char(
                        char::from_u32(value as u32).ok_or("not a character")?,
                    ),
                    1 => KeyCode::F(value as u8),
                    2 => named_key(value).ok_or_else(|| format!("no named key {value}"))?,
                    _ => return Err(format!("no key kind {kind}")),
                };
                let mods = Mods {
                    shift: m & 1 != 0,
                    ctrl: m & 2 != 0,
                    alt: m & 4 != 0,
                };
                self.ui()?.handle(Event::Key(Key { code, mods }));
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

fn lines_of(text: &str) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = text.split('\n').map(cp437::encode).collect();
    if v.is_empty() {
        v.push(Vec::new());
    }
    v
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
    let mut server = Server { ui: None };

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
