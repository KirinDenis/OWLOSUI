//! A console: text that arrives and keeps arriving, coloured by the ANSI
//! sequences in it.
//!
//! Not an editor and not a page. An editor's lines are the document and
//! can be changed anywhere; a console's are a record - written once, at the
//! end, and then only read, scrolled back through and copied out. That is
//! what a log is, and what a bug report wants: "show me what is in the
//! console".
//!
//! What it understands is what programs actually print: the colours of SGR
//! (`ESC[1;33;44m`, the 256-colour and 24-bit forms folded onto the sixteen
//! this screen has), carriage return, backspace, tab, erase-line,
//! cursor-forward and clear-screen. Cursor movement *up* is not understood
//! and is swallowed: a record that the writer can go back and rewrite is
//! not a record, and the full screen of an ANSI drawing that needs it is a
//! canvas's job. Every sequence is consumed whole, so one it does not know
//! leaves nothing behind on the screen.
//!
//! Long lines fold to the width the view is given, and fold again when the
//! window is resized: the lines are kept as written, the rows are worked
//! out from them.

#[allow(unused_imports)]
use alloc::{string::String, vec::Vec};

use crate::cell::{Cell, Glyph};

/// Light grey on black: what a terminal looks like before anyone colours it.
pub const CONSOLE_ATTR: u8 = 0x07;

/// The most a line can hold before it is broken for the writer. A program
/// that never prints a newline must not grow one line without end.
const MAX_LINE: usize = 2048;

/// ANSI's colour order to the IBM attribute's: ANSI counts red as 1, the
/// IBM card blue.
const ANSI_TO_IBM: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];

/// The sixteen colours as the VGA showed them, to fold 24-bit colour onto.
const VGA: [(u8, u8, u8); 16] = [
    (0, 0, 0), (0, 0, 170), (0, 170, 0), (0, 170, 170),
    (170, 0, 0), (170, 0, 170), (170, 85, 0), (170, 170, 170),
    (85, 85, 85), (85, 85, 255), (85, 255, 85), (85, 255, 255),
    (255, 85, 85), (255, 85, 255), (255, 255, 85), (255, 255, 255),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Esc {
    None,
    /// After ESC.
    Esc,
    /// Inside `ESC[`, gathering numbers.
    Csi,
    /// Inside `ESC]`, an operating-system string, up to BEL or `ESC\`.
    Osc,
    OscEsc,
}

pub struct Console {
    /// As written; the last one is the line being written, and there is
    /// always one.
    lines: Vec<Vec<Cell>>,
    /// The cursor's column in the last line.
    col: usize,
    attr: u8,
    bold: bool,
    reverse: bool,
    state: Esc,
    params: Vec<u16>,
    /// How many lines are kept. The oldest go first.
    pub scrollback: usize,
    /// The first row shown.
    pub top: i16,
    /// Whether the newest row is kept in view as more arrives. Scrolling up
    /// stops following; coming back down to the end starts it again.
    pub follow: bool,
    /// Rows, as `(line, first column)`, for the width they were made for.
    rows: Vec<(u32, u16)>,
    width: i16,
    height: i16,
    dirty: bool,
    pub focused: bool,
}

impl Console {
    pub fn new(scrollback: usize) -> Self {
        Console {
            lines: vec![Vec::new()],
            col: 0,
            attr: CONSOLE_ATTR,
            bold: false,
            reverse: false,
            state: Esc::None,
            params: Vec::new(),
            scrollback: scrollback.max(1),
            top: 0,
            follow: true,
            rows: Vec::new(),
            width: 0,
            height: 0,
            dirty: true,
            focused: false,
        }
    }

    /// Text in, sequences obeyed. `glyph` turns a printable character into
    /// what the screen shows for it; the caller owns the font. A sequence
    /// cut in two between writes is carried over: the state is kept.
    pub fn write(&mut self, text: &str, glyph: &mut dyn FnMut(char) -> Glyph) {
        for c in text.chars() {
            self.feed(c, glyph);
        }
        self.trim();
        self.dirty = true;
    }

    fn feed(&mut self, c: char, glyph: &mut dyn FnMut(char) -> Glyph) {
        match self.state {
            Esc::None => match c {
                '\x1b' => self.state = Esc::Esc,
                '\n' => self.newline(),
                '\r' => self.col = 0,
                '\t' => {
                    let to = (self.col / 8 + 1) * 8;
                    while self.col < to {
                        self.put(b' ' as Glyph);
                    }
                }
                '\x08' => self.col = self.col.saturating_sub(1),
                // The other control characters ring bells and shift
                // character sets on a real terminal; here they would only
                // be pictures nobody meant.
                c if (c as u32) < 0x20 || c == '\x7f' => {}
                c => {
                    let g = glyph(c);
                    self.put(g);
                }
            },
            Esc::Esc => {
                self.state = match c {
                    '[' => {
                        self.params.clear();
                        self.params.push(0);
                        Esc::Csi
                    }
                    ']' => Esc::Osc,
                    'c' => {
                        self.sgr_reset();
                        Esc::None
                    }
                    _ => Esc::None,
                }
            }
            Esc::Csi => match c {
                '0'..='9' => {
                    if let Some(p) = self.params.last_mut() {
                        *p = p.saturating_mul(10).saturating_add(c as u16 - b'0' as u16);
                    }
                }
                ';' | ':' => {
                    if self.params.len() < 16 {
                        self.params.push(0);
                    }
                }
                // The final byte: the sequence is whole.
                '\x40'..='\x7e' => {
                    self.state = Esc::None;
                    self.csi(c);
                }
                // `?` and friends, and intermediates: noted by skipping.
                _ => {}
            },
            Esc::Osc => match c {
                '\x07' => self.state = Esc::None,
                '\x1b' => self.state = Esc::OscEsc,
                _ => {}
            },
            Esc::OscEsc => self.state = Esc::None,
        }
    }

    fn csi(&mut self, fin: char) {
        let n = |p: &[u16], i: usize| p.get(i).copied().unwrap_or(0);
        let params = core::mem::take(&mut self.params);
        let count = (n(&params, 0) as usize).max(1);
        match fin {
            'm' => self.sgr(&params),
            // Erase in line: to the end, to the start, or all of it.
            'K' => self.erase_line(n(&params, 0)),
            // Erase in display. A record has no screen below the cursor to
            // clear, so 0 is the rest of the line; 2 and 3 start over.
            'J' => match n(&params, 0) {
                2 | 3 => self.clear(),
                _ => self.erase_line(0),
            },
            'C' => {
                for _ in 0..count.min(MAX_LINE) {
                    self.advance();
                }
            }
            'D' => self.col = self.col.saturating_sub(count),
            // A column, counted from one; and a place, whose row is ignored:
            // there is only the line being written.
            'G' => self.col = count - 1,
            'H' | 'f' => self.col = (n(&params, 1) as usize).max(1) - 1,
            _ => {}
        }
        self.params = params;
        self.params.clear();
    }

    fn sgr(&mut self, p: &[u16]) {
        let mut i = 0;
        while i < p.len() {
            match p[i] {
                0 => self.sgr_reset(),
                1 => self.bold = true,
                22 => self.bold = false,
                7 => self.reverse = true,
                27 => self.reverse = false,
                v @ 30..=37 => self.set_fg(ANSI_TO_IBM[(v - 30) as usize]),
                v @ 90..=97 => self.set_fg(ANSI_TO_IBM[(v - 90) as usize] | 8),
                39 => self.set_fg(CONSOLE_ATTR & 0x0F),
                v @ 40..=47 => self.set_bg(ANSI_TO_IBM[(v - 40) as usize]),
                v @ 100..=107 => self.set_bg(ANSI_TO_IBM[(v - 100) as usize] | 8),
                49 => self.set_bg(CONSOLE_ATTR >> 4),
                // 38/48;5;n and 38/48;2;r;g;b: colours this screen does not
                // have, given the nearest of the ones it does.
                v @ (38 | 48) => {
                    let c = match p.get(i + 1) {
                        Some(5) => {
                            let c = p.get(i + 2).map(|&n| from_256(n));
                            i += 2;
                            c
                        }
                        Some(2) => {
                            let ch = |k| p.get(i + k).copied().unwrap_or(0).min(255) as u8;
                            let c = nearest(ch(2), ch(3), ch(4));
                            i += 4;
                            Some(c)
                        }
                        _ => None,
                    };
                    if let Some(c) = c {
                        if v == 38 {
                            self.set_fg(c);
                        } else {
                            self.set_bg(c);
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn sgr_reset(&mut self) {
        self.attr = CONSOLE_ATTR;
        self.bold = false;
        self.reverse = false;
    }

    fn set_fg(&mut self, c: u8) {
        self.attr = (self.attr & 0xF0) | (c & 0x0F);
    }

    fn set_bg(&mut self, c: u8) {
        self.attr = (self.attr & 0x0F) | ((c & 0x0F) << 4);
    }

    /// The attribute a character is written in now: bold is the bright
    /// half of the colours, as it was on every text-mode card.
    fn shown(&self) -> u8 {
        let mut a = self.attr;
        if self.bold {
            a |= 0x08;
        }
        if self.reverse {
            a = (a << 4) | (a >> 4);
        }
        a
    }

    fn put(&mut self, g: Glyph) {
        if self.col >= MAX_LINE {
            self.newline();
        }
        let cell = Cell::new(g, self.shown());
        let line = self.lines.last_mut().expect("there is always a line");
        while line.len() < self.col {
            line.push(Cell::new(b' ' as Glyph, CONSOLE_ATTR));
        }
        if self.col < line.len() {
            line[self.col] = cell;
        } else {
            line.push(cell);
        }
        self.col += 1;
    }

    /// Cursor forward: over what is there, and through spaces past the end.
    fn advance(&mut self) {
        let line = self.lines.last_mut().expect("there is always a line");
        if self.col >= line.len() {
            line.push(Cell::new(b' ' as Glyph, CONSOLE_ATTR));
        }
        self.col += 1;
    }

    fn erase_line(&mut self, how: u16) {
        let col = self.col;
        let line = self.lines.last_mut().expect("there is always a line");
        match how {
            1 => {
                for c in line.iter_mut().take(col + 1) {
                    *c = Cell::new(b' ' as Glyph, CONSOLE_ATTR);
                }
            }
            2 => line.clear(),
            _ => line.truncate(col),
        }
    }

    fn newline(&mut self) {
        self.lines.push(Vec::new());
        self.col = 0;
    }

    /// Everything gone, as `ESC[2J` asks: the colours stay as they were set.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.lines.push(Vec::new());
        self.col = 0;
        self.top = 0;
        self.dirty = true;
    }

    /// The oldest lines out, an eighth of the scrollback at a time, so that
    /// a busy console is not shifting its whole history on every line.
    fn trim(&mut self) {
        if self.lines.len() > self.scrollback + self.scrollback / 8 {
            let gone = self.lines.len() - self.scrollback;
            self.lines.drain(..gone);
        }
    }

    pub fn lines(&self) -> &[Vec<Cell>] {
        &self.lines
    }

    /// Rows for this size, worked out again only when the width changed or
    /// something was written; and the end kept in view while following.
    pub fn layout(&mut self, w: i16, h: i16) {
        let w = w.max(1);
        if self.dirty || w != self.width {
            self.rows.clear();
            for (i, line) in self.lines.iter().enumerate() {
                let n = line.len().max(1);
                let mut start = 0;
                while start < n {
                    self.rows.push((i as u32, start as u16));
                    start += w as usize;
                }
            }
            // A last line nobody has written on yet is not a row to look at.
            if self.lines.last().is_some_and(|l| l.is_empty()) && self.rows.len() > 1 {
                self.rows.pop();
            }
            self.width = w;
            self.dirty = false;
        }
        self.height = h.max(1);
        let max = self.max_top();
        if self.follow {
            self.top = max;
        }
        self.top = self.top.clamp(0, max);
    }

    fn max_top(&self) -> i16 {
        (self.row_count() - self.height).max(0)
    }

    pub fn row_count(&self) -> i16 {
        self.rows.len().min(i16::MAX as usize) as i16
    }

    /// The cells of row `r` as laid out, or `None` past the end.
    pub fn row(&self, r: i16) -> Option<&[Cell]> {
        let &(line, start) = self.rows.get(usize::try_from(r).ok()?)?;
        let l = &self.lines[line as usize];
        let start = (start as usize).min(l.len());
        let end = (start + self.width.max(1) as usize).min(l.len());
        Some(&l[start..end])
    }

    /// Scroll by `delta` rows. Reaching the end starts following again.
    pub fn scroll(&mut self, delta: i16) {
        let max = self.max_top();
        self.top = (self.top.saturating_add(delta)).clamp(0, max);
        self.follow = self.top >= max;
    }

    pub fn home(&mut self) {
        self.top = 0;
        self.follow = self.max_top() == 0;
    }

    pub fn end(&mut self) {
        self.top = self.max_top();
        self.follow = true;
    }
}

/// A 256-colour index onto the sixteen: the first sixteen are the sixteen,
/// the cube and the greys go to the nearest.
fn from_256(n: u16) -> u8 {
    match n {
        0..=7 => ANSI_TO_IBM[n as usize],
        8..=15 => ANSI_TO_IBM[(n - 8) as usize] | 8,
        16..=231 => {
            let n = n - 16;
            let level = |v: u16| if v == 0 { 0 } else { (55 + v * 40) as u8 };
            nearest(level(n / 36), level((n / 6) % 6), level(n % 6))
        }
        232..=255 => {
            let v = (8 + (n - 232) * 10) as u8;
            nearest(v, v, v)
        }
        _ => CONSOLE_ATTR & 0x0F,
    }
}

/// The nearest of the sixteen VGA colours. Plain distance alone puts pure
/// red, `#ff0000`, on the dark red (170,0,0) rather than the bright one
/// (255,85,85), and every terminal shows it bright: so how bright the
/// strongest channel is counts as well, twice over.
fn nearest(r: u8, g: u8, b: u8) -> u8 {
    let top = r.max(g).max(b) as i32;
    let d = |&(vr, vg, vb): &(u8, u8, u8)| {
        let (dr, dg, db) = (r as i32 - vr as i32, g as i32 - vg as i32, b as i32 - vb as i32);
        let dt = top - vr.max(vg).max(vb) as i32;
        dr * dr + dg * dg + db * db + 2 * dt * dt
    };
    (0..16u8).min_by_key(|&i| d(&VGA[i as usize])).unwrap_or(7)
}
