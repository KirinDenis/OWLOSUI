//! The small controls a dialog is made of.
//!
//! A cluster of choices, a list to pick from, and a piece of text that does
//! nothing. Between them and the input line, the button row and the editor,
//! that is a dialog.
//!
//! Check boxes and radio buttons are **one view with two modes**, because the
//! difference between them is one rule — how many may be on at once — and
//! everything else, the walking, the marking, the hotkeys, is the same. The
//! classic toolkits had them as two descendants of one class for the same reason. Two
//! separate controls would be two copies of the keyboard handling, and the
//! second copy is where the bugs would be.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
/// How many of a cluster's choices may be on at once.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    /// Check boxes: any number, including none.
    Many,
    /// Radio buttons: exactly one, always.
    One,
}

pub struct Cluster {
    pub mode: Choice,
    /// Labels, with the hotkey between tildes.
    pub items: Vec<String>,
    /// Which are on. For `Choice::One` exactly one is.
    pub on: Vec<bool>,
    pub current: usize,
    pub focused: bool,
    pub enabled: bool,
}

impl Cluster {
    pub fn checks(items: &[&str]) -> Self {
        Cluster {
            mode: Choice::Many,
            items: items.iter().map(|s| s.to_string()).collect(),
            on: vec![false; items.len()],
            current: 0,
            focused: false,
            enabled: true,
        }
    }

    pub fn radio(items: &[&str]) -> Self {
        let mut on = vec![false; items.len()];
        if !on.is_empty() {
            on[0] = true;
        }
        Cluster {
            mode: Choice::One,
            items: items.iter().map(|s| s.to_string()).collect(),
            on,
            current: 0,
            focused: false,
            enabled: true,
        }
    }

    pub fn label(&self, i: usize) -> String {
        self.items[i].replace('~', "")
    }

    pub fn hotkey_at(&self, i: usize) -> Option<usize> {
        self.items[i]
            .find('~')
            .map(|p| self.items[i][..p].chars().count())
    }

    pub fn hotkey(&self, i: usize) -> Option<char> {
        let mut it = self.items[i].split('~');
        it.next()?;
        it.next()?.chars().next().map(|c| c.to_ascii_lowercase())
    }

    /// `[X]` or `(•)`, which is the whole visual difference and is on purpose:
    /// a square means "and", a round one means "or", and people read that
    /// without ever being told.
    pub fn marker(&self, i: usize) -> [u8; 3] {
        let on = self.on.get(i).copied().unwrap_or(false);
        match self.mode {
            Choice::Many => [b'[', if on { b'X' } else { b' ' }, b']'],
            Choice::One => [b'(', if on { 0x07 } else { b' ' }, b')'],
        }
    }

    pub fn step(&mut self, d: i16) {
        if self.items.is_empty() {
            return;
        }
        let n = self.items.len() as i16;
        self.current = ((self.current as i16 + d).rem_euclid(n)) as usize;
    }

    /// Turn the current one on, or over.
    pub fn toggle(&mut self) {
        if self.current >= self.on.len() {
            return;
        }
        match self.mode {
            Choice::Many => self.on[self.current] = !self.on[self.current],
            Choice::One => {
                // Exactly one, always. Turning the chosen one off would leave
                // a question with no answer and no way to give one.
                for v in self.on.iter_mut() {
                    *v = false;
                }
                self.on[self.current] = true;
            }
        }
    }

    pub fn by_hotkey(&self, c: char) -> Option<usize> {
        let c = c.to_ascii_lowercase();
        (0..self.items.len()).find(|&i| self.hotkey(i) == Some(c))
    }

    /// Which are on, for whoever asked the question.
    pub fn chosen(&self) -> Vec<usize> {
        (0..self.on.len()).filter(|&i| self.on[i]).collect()
    }
}

/// A single column of things to pick from.
pub struct ListBox {
    pub items: Vec<String>,
    pub current: usize,
    pub top: i16,
    pub focused: bool,
    rows: i16,
    /// Insert marks an item and moves down, the way the classic file managers marked
    /// files. Off by default: a list that answers a question has one
    /// answer, and a mark on it would be a second one.
    pub multi: bool,
    marked: Vec<bool>,
    /// Scrolled by the wheel or the bar while the cursor was on this item:
    /// the view stays where it was put, the cursor out of it if need be,
    /// until the cursor moves - as in every list since the mouse wheel.
    scrolled_at: Option<usize>,
}

impl ListBox {
    pub fn new(items: &[&str]) -> Self {
        ListBox {
            items: items.iter().map(|s| s.to_string()).collect(),
            current: 0,
            top: 0,
            focused: false,
            rows: 1,
            multi: false,
            marked: Vec::new(),
            scrolled_at: None,
        }
    }

    /// The view by `delta` rows, the cursor left where it is.
    pub fn scroll(&mut self, delta: i16) {
        let max = (self.items.len() as i16 - self.rows).max(0);
        // Everything is showing: there is nothing to scroll, and a wheel
        // that did nothing would look broken. It moves the cursor instead,
        // one item a notch, as the arrow keys do.
        if max == 0 {
            return self.step(delta.signum());
        }
        self.top = (self.top.saturating_add(delta)).clamp(0, max);
        self.scrolled_at = Some(self.current);
    }

    /// Mark or unmark the current item and step to the next, so that
    /// holding Insert marks a run. Nothing happens on a single-choice list.
    pub fn toggle_mark(&mut self) {
        if !self.multi || self.items.is_empty() {
            return;
        }
        if self.marked.len() != self.items.len() {
            self.marked.resize(self.items.len(), false);
        }
        self.marked[self.current] = !self.marked[self.current];
        self.step(1);
    }

    pub fn is_marked(&self, ix: usize) -> bool {
        self.marked.get(ix).copied().unwrap_or(false)
    }

    /// The marked items, in order. Empty on a single-choice list.
    pub fn marked(&self) -> Vec<usize> {
        (0..self.items.len()).filter(|&i| self.is_marked(i)).collect()
    }

    pub fn set_rows(&mut self, rows: i16) {
        self.rows = rows.max(1);
        if self.scrolled_at == Some(self.current) {
            // Scrolled, and the cursor has not moved since: stay put.
            let max = (self.items.len() as i16 - self.rows).max(0);
            self.top = self.top.clamp(0, max);
        } else {
            self.scrolled_at = None;
            self.follow();
        }
    }

    pub fn rows(&self) -> i16 {
        self.rows
    }

    pub fn step(&mut self, d: i16) {
        if self.items.is_empty() {
            return;
        }
        let n = self.items.len() as i16;
        self.current = (self.current as i16 + d).clamp(0, n - 1) as usize;
        self.scrolled_at = None;
        self.follow();
    }

    fn follow(&mut self) {
        let cur = self.current as i16;
        if cur < self.top {
            self.top = cur;
        } else if cur >= self.top + self.rows {
            self.top = cur - self.rows + 1;
        }
        let max = (self.items.len() as i16 - self.rows).max(0);
        self.top = self.top.clamp(0, max);
    }

    pub fn at_row(&self, row: i16) -> Option<usize> {
        let ix = (self.top + row) as usize;
        (row >= 0 && row < self.rows && ix < self.items.len()).then_some(ix)
    }
}

/// Words that do nothing.
///
/// Not a decoration to be skipped: it is where a dialog says what it is for,
/// and a dialog that cannot say so is a dialog people cancel.
pub struct StaticText {
    pub text: String,
    /// The first line shown: words longer than their box scroll with the
    /// wheel, and an arrow in the last column says there is more.
    pub top: i16,
}

impl StaticText {
    pub fn new(text: &str) -> Self {
        StaticText {
            text: text.to_string(),
            top: 0,
        }
    }

    /// New words, read from their first line.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.top = 0;
    }

    /// The wheel: `delta` lines, within the words broken to `width` and a
    /// box `rows` high.
    pub fn scroll(&mut self, delta: i16, width: i16, rows: i16) {
        let max = (self.lines(width).len() as i16 - rows).max(0);
        self.top = (self.top + delta).clamp(0, max);
    }

    /// Broken to a width, keeping words whole.
    pub fn lines(&self, width: i16) -> Vec<String> {
        let w = width.max(4) as usize;
        let mut out = Vec::new();
        for para in self.text.split('\n') {
            let mut cur = String::new();
            for word in para.split_whitespace() {
                if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > w {
                    out.push(core::mem::take(&mut cur));
                }
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(word);
            }
            out.push(cur);
        }
        out
    }
}

/// Words with a hotkey, standing beside the control they name.
///
/// `~N~ame:` next to an input line means Alt+N puts the caret there, and a
/// click on the words does the same. The classic label was the
/// difference between a dialog you could drive blind and one you had to
/// Tab through counting; it costs one field, the handle of the control.
pub struct Label {
    pub text: String,
    pub target: Option<crate::ui::ViewId>,
}

impl Label {
    pub fn new(text: &str, target: Option<crate::ui::ViewId>) -> Self {
        Label {
            text: text.into(),
            target,
        }
    }

    pub fn label(&self) -> String {
        self.text.replace('~', "")
    }

    pub fn hotkey(&self) -> Option<char> {
        let mut it = self.text.split('~');
        it.next()?;
        it.next()?.chars().next().map(|c| c.to_ascii_lowercase())
    }

    pub fn hotkey_at(&self) -> Option<usize> {
        self.text.find('~').map(|i| self.text[..i].chars().count())
    }
}

/// A bar that fills up: copying, searching, waiting.
///
/// `value` of `max`, drawn as `█` for what is done and `░` for what is not,
/// with the percentage at the right if there is room for it. The program
/// sets the value; nothing here counts. The classic toolkits never had one, and
/// every program that needed one drew a row of blocks by hand - which is
/// exactly the case for it being here.
pub struct Progress {
    pub value: u32,
    pub max: u32,
    /// Show `42%` at the right end.
    pub percent: bool,
}

impl Progress {
    pub fn new(max: u32) -> Self {
        Progress {
            value: 0,
            max: max.max(1),
            percent: true,
        }
    }

    pub fn set(&mut self, value: u32) {
        self.value = value.min(self.max);
    }

    /// How many of `width` cells are filled.
    pub fn filled(&self, width: i16) -> i16 {
        let w = width.max(0) as u64;
        ((self.value as u64 * w) / self.max as u64) as i16
    }

    pub fn percent_text(&self) -> String {
        format!("{}%", (self.value as u64 * 100 / self.max as u64).min(100))
    }
}

/// A grid of cells the program draws itself.
///
/// The classic answer to "my view is not one of yours" was a view
/// with its own `draw`. Over a wire there is no `draw` to call, so the
/// program sends cells: characters with attributes, into a rectangle that
/// is then drawn as it is - clipped, scrolled with its window, covered by
/// what floats above, and never reflowed. A game board, a chart, a piece
/// of ANSI art. What it is not is text: nothing here wraps or collapses.
pub struct Canvas {
    pub w: i16,
    pub h: i16,
    pub cells: Vec<crate::cell::Cell>,
    /// Where the mouse last went down on it, in canvas cells, until
    /// somebody asks. A canvas has no idea what its cells mean - a
    /// calculator's keys, a puzzle's tiles - so the click is handed out
    /// as a place and the program decides what was hit.
    pub clicked: Option<(i16, i16)>,
}

impl Canvas {
    /// A cell that is not drawn: whatever is behind the canvas shows
    /// through. Glyph 0 with a white-on-white blinking attribute is a
    /// picture nobody means, which is what makes it safe as a marker. A
    /// new canvas is all clear - it is the window until something is put
    /// on it.
    pub const CLEAR: crate::cell::Cell = crate::cell::Cell { ch: 0, attr: 0xFF };

    pub fn new(w: i16, h: i16) -> Self {
        let (w, h) = (w.max(0), h.max(0));
        Canvas {
            w,
            h,
            cells: vec![Canvas::CLEAR; (w as usize) * (h as usize)],
            clicked: None,
        }
    }

    pub fn get(&self, x: i16, y: i16) -> Option<crate::cell::Cell> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        Some(self.cells[(y as usize) * (self.w as usize) + (x as usize)])
    }

    /// Put a block of cells at a place. What falls outside the canvas is
    /// dropped, cell by cell, so a block half off the edge is not an error.
    pub fn blit(&mut self, x: i16, y: i16, w: i16, h: i16, cells: &[crate::cell::Cell]) {
        for j in 0..h.max(0) {
            for i in 0..w.max(0) {
                let (cx, cy) = (x + i, y + j);
                if cx < 0 || cy < 0 || cx >= self.w || cy >= self.h {
                    continue;
                }
                let Some(&c) = cells.get((j as usize) * (w as usize) + (i as usize)) else {
                    return;
                };
                self.cells[(cy as usize) * (self.w as usize) + (cx as usize)] = c;
            }
        }
    }
}
