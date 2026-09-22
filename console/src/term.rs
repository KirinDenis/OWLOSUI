//! The terminal backend.
//!
//! Two things are worth knowing about this file.
//!
//! First, it writes VT escape sequences by hand rather than calling
//! `WriteConsoleOutput`. Under Windows Terminal the console is reached through
//! ConPTY, which turns the legacy screen buffer back into a VT stream anyway —
//! so the "fast" Windows path is now the slow one, and the same code serves
//! Windows and Unix.
//!
//! Second, a frame is assembled into one `String` and written with a single
//! `write_all`. Terminal speed is almost never about how many bytes you send;
//! it is about how many times you send them. Rust's `print!` locks and flushes
//! per call, which is why people conclude that terminals are slow.

use std::io::{Result, Write};

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyEventKind, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{event as ct, execute};

use owlosui_core::{Buffer, Button, Cell, Event, Key, KeyCode, Mods, Mouse, MouseKind, Point};

use crate::cp437;

/// IBM attribute nibble to ANSI colour number. The two orders are not the
/// same: IBM counts blue as 1, ANSI counts red as 1.
const ANSI: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];

/// The sixteen VGA colours as the DAC actually produced them.
///
/// Asking a terminal for "colour 1" gets whatever blue its theme happens to
/// use, which is why the same screen looks washed out next to a DOSBox window
/// showing the real thing. Sending the RGB values instead makes our picture
/// the DOS picture everywhere that understands 24-bit colour.
///
/// These are the 6-bit DAC values scaled up: 0x00, 0x55 and 0xAA are 0, 21 and
/// 42 out of 63. The palette is defined in DAC terms and expanded here, never
/// the reverse — going the other way loses precision on the machine that
/// matters.
const VGA: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00), // black
    (0x00, 0x00, 0xAA), // blue
    (0x00, 0xAA, 0x00), // green
    (0x00, 0xAA, 0xAA), // cyan
    (0xAA, 0x00, 0x00), // red
    (0xAA, 0x00, 0xAA), // magenta
    (0xAA, 0x55, 0x00), // brown
    (0xAA, 0xAA, 0xAA), // light grey
    (0x55, 0x55, 0x55), // dark grey
    (0x55, 0x55, 0xFF), // light blue
    (0x55, 0xFF, 0x55), // light green
    (0x55, 0xFF, 0xFF), // light cyan
    (0xFF, 0x55, 0x55), // light red
    (0xFF, 0x55, 0xFF), // light magenta
    (0xFF, 0xFF, 0x55), // yellow
    (0xFF, 0xFF, 0xFF), // white
];

pub struct Term {
    out: std::io::Stdout,
    /// What the terminal is currently showing, so we send only differences.
    /// On DOS this shadow copy would be pure waste — there, blitting the whole
    /// screen costs less than comparing it. Here every changed cell is an
    /// escape sequence, so the comparison pays for itself many times over.
    shadow: Vec<Cell>,
    w: i16,
    h: i16,
    frame: String,
    /// Off via `OWLOSUI_COLOR=16` for a terminal that cannot do 24-bit.
    truecolor: bool,
    /// Where the terminal cursor currently is, so it is only moved when it
    /// needs moving.
    cursor: Option<Point>,
}

impl Term {
    pub fn new() -> Result<Self> {
        let mut out = std::io::stdout();
        enable_raw_mode()?;
        execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
        // Start with the cursor hidden — `render` turns it back on and parks
        // it wherever the caret is — and turn off auto-wrap.
        //
        // Auto-wrap matters more than it looks: writing the bottom-right cell
        // of the screen wraps the cursor, which scrolls the terminal by a line
        // and shifts the whole picture up. A full-screen UI fills that cell on
        // every frame, so this is not an edge case — it is every frame.
        out.write_all(b"\x1b[?25l\x1b[?7l")?;
        out.flush()?;
        Ok(Term {
            out,
            shadow: Vec::new(),
            w: 0,
            h: 0,
            frame: String::new(),
            truecolor: std::env::var("OWLOSUI_COLOR").as_deref() != Ok("16"),
            cursor: None,
        })
    }

    pub fn size() -> Result<(i16, i16)> {
        let (w, h) = crossterm::terminal::size()?;
        Ok((w as i16, h as i16))
    }

    pub fn render(&mut self, buf: &Buffer, cursor: Option<Point>) -> Result<()> {
        let (w, h) = (buf.width(), buf.height());
        let full = w != self.w || h != self.h || self.shadow.len() != buf.cells().len();
        if full {
            self.w = w;
            self.h = h;
            self.shadow = vec![
                Cell::new(0xFF, 0xFF); // impossible value: forces a full repaint
                (w as usize) * (h as usize)
            ];
        }

        self.frame.clear();
        let mut cur_attr: Option<u8> = None;

        for y in 0..h {
            let row = (y as usize) * (w as usize);
            // Only the span between the first and last changed cell needs
            // redrawing. Finer granularity is possible and not worth it: the
            // cursor-move sequence costs about as much as a few cells.
            let mut first = None;
            let mut last = 0usize;
            for x in 0..(w as usize) {
                if buf.cells()[row + x] != self.shadow[row + x] {
                    if first.is_none() {
                        first = Some(x);
                    }
                    last = x;
                }
            }
            let Some(first) = first else { continue };

            self.frame
                .push_str(&format!("\x1b[{};{}H", y + 1, first as i16 + 1));
            for x in first..=last {
                let c = buf.cells()[row + x];
                if cur_attr != Some(c.attr) {
                    push_sgr(&mut self.frame, c.attr, self.truecolor);
                    cur_attr = Some(c.attr);
                }
                self.frame.push(cp437::to_char(c.ch));
                self.shadow[row + x] = c;
            }
        }

        // The caret is the terminal's own cursor, parked wherever the core says
        // it is. Drawing anything at all moves the cursor, so it has to be put
        // back on every frame that wrote something — not only when it moved.
        if self.frame.is_empty() && cursor == self.cursor {
            return Ok(());
        }
        self.frame.push_str("\x1b[0m");
        match cursor {
            Some(p) => self
                .frame
                .push_str(&format!("\x1b[{};{}H\x1b[?25h", p.y + 1, p.x + 1)),
            None => self.frame.push_str("\x1b[?25l"),
        }
        self.cursor = cursor;

        self.out.write_all(self.frame.as_bytes())?;
        self.out.flush()
    }

    /// Block for the next event, translating it into the core's vocabulary.
    /// Returns `None` for events the core has no opinion about.
    ///
    /// Alt with a letter arrives in one of two shapes and we have to take
    /// both. A terminal that knows about modifiers sends one event with the
    /// Alt flag set. An older one — and a Windows console reached through
    /// ConPTY behaves like an older one — sends Escape followed immediately by
    /// the letter, which is how Alt has been spelled on a wire since before
    /// anybody had a flag for it.
    ///
    /// The two are told apart by time and nothing else: nobody types Escape
    /// and then a letter inside a millisecond, and no terminal splits them by
    /// more than that.
    pub fn next_event(&self) -> Result<Option<Event>> {
        let first = ct::read()?;

        if let ct::Event::Key(k) = &first {
            if k.code == ct::KeyCode::Esc
                && k.kind == KeyEventKind::Press
                && k.modifiers.is_empty()
                && ct::poll(std::time::Duration::from_millis(1))?
            {
                let second = ct::read()?;
                if let ct::Event::Key(k2) = &second {
                    if let ct::KeyCode::Char(c) = k2.code {
                        if k2.kind == KeyEventKind::Press {
                            return Ok(Some(Event::Key(Key::new(
                                KeyCode::Char(c),
                                Mods::alt(),
                            ))));
                        }
                    }
                }
                // Not a prefix after all: a real Escape and then something
                // else. The something else is the one that would be lost, so
                // it is what we hand back — the Escape has already done its
                // job of not being a modifier.
                return Ok(translate(second));
            }
        }

        Ok(translate(first))
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        let _ = self.out.write_all(b"\x1b[0m\x1b[?7h\x1b[?25h");
        let _ = execute!(self.out, DisableMouseCapture, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn push_sgr(s: &mut String, attr: u8, truecolor: bool) {
    let fg = attr & 0x0F;
    let bg = attr >> 4;

    if truecolor {
        let (fr, fg_, fb) = VGA[fg as usize];
        let (br, bg_, bb) = VGA[bg as usize];
        s.push_str(&format!(
            "\x1b[38;2;{fr};{fg_};{fb};48;2;{br};{bg_};{bb}m"
        ));
        return;
    }

    // Fallback for terminals without 24-bit colour: the indexed sixteen, whose
    // exact shades belong to the user's theme and not to us.
    let f = if fg < 8 {
        30 + ANSI[fg as usize] as u16
    } else {
        90 + ANSI[(fg - 8) as usize] as u16
    };
    let b = if bg < 8 {
        40 + ANSI[bg as usize] as u16
    } else {
        100 + ANSI[(bg - 8) as usize] as u16
    };
    s.push_str(&format!("\x1b[{};{}m", f, b));
}

fn mods(m: KeyModifiers) -> Mods {
    Mods {
        ctrl: m.contains(KeyModifiers::CONTROL),
        alt: m.contains(KeyModifiers::ALT),
        shift: m.contains(KeyModifiers::SHIFT),
    }
}

fn translate(e: ct::Event) -> Option<Event> {
    match e {
        ct::Event::Resize(w, h) => Some(Event::Resize(w as i16, h as i16)),

        ct::Event::Key(k) => {
            // Windows reports press *and* release. Without this filter every
            // keystroke arrives twice.
            if k.kind != KeyEventKind::Press {
                return None;
            }
            let code = match k.code {
                ct::KeyCode::Char(c) => KeyCode::Char(c),
                ct::KeyCode::Enter => KeyCode::Enter,
                ct::KeyCode::Esc => KeyCode::Esc,
                ct::KeyCode::Tab => KeyCode::Tab,
                ct::KeyCode::BackTab => KeyCode::BackTab,
                ct::KeyCode::Backspace => KeyCode::Backspace,
                ct::KeyCode::Delete => KeyCode::Delete,
                ct::KeyCode::Insert => KeyCode::Insert,
                ct::KeyCode::Home => KeyCode::Home,
                ct::KeyCode::End => KeyCode::End,
                ct::KeyCode::PageUp => KeyCode::PageUp,
                ct::KeyCode::PageDown => KeyCode::PageDown,
                ct::KeyCode::Up => KeyCode::Up,
                ct::KeyCode::Down => KeyCode::Down,
                ct::KeyCode::Left => KeyCode::Left,
                ct::KeyCode::Right => KeyCode::Right,
                ct::KeyCode::F(n) => KeyCode::F(n),
                _ => return None,
            };
            Some(Event::Key(Key::new(code, mods(k.modifiers))))
        }

        ct::Event::Mouse(m) => {
            let kind = match m.kind {
                MouseEventKind::Down(MouseButton::Left) => MouseKind::Down(Button::Left),
                MouseEventKind::Down(MouseButton::Right) => MouseKind::Down(Button::Right),
                MouseEventKind::Down(MouseButton::Middle) => MouseKind::Down(Button::Middle),
                MouseEventKind::Up(MouseButton::Left) => MouseKind::Up(Button::Left),
                MouseEventKind::Up(MouseButton::Right) => MouseKind::Up(Button::Right),
                MouseEventKind::Up(MouseButton::Middle) => MouseKind::Up(Button::Middle),
                MouseEventKind::Drag(_) => MouseKind::Drag,
                MouseEventKind::Moved => MouseKind::Move,
                MouseEventKind::ScrollUp => MouseKind::ScrollUp,
                MouseEventKind::ScrollDown => MouseKind::ScrollDown,
                _ => return None,
            };
            Some(Event::Mouse(Mouse {
                x: m.column as i16,
                y: m.row as i16,
                kind,
            }))
        }

        _ => None,
    }
}
