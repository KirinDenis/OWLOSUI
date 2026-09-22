//! The small controls a dialog is made of.
//!
//! A cluster of choices, a list to pick from, and a piece of text that does
//! nothing. Between them and the input line, the button row and the editor,
//! that is a dialog.
//!
//! Check boxes and radio buttons are **one view with two modes**, because the
//! difference between them is one rule — how many may be on at once — and
//! everything else, the walking, the marking, the hotkeys, is the same. Turbo
//! Vision had them as two descendants of one class for the same reason. Two
//! separate controls would be two copies of the keyboard handling, and the
//! second copy is where the bugs would be.

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
}

impl ListBox {
    pub fn new(items: &[&str]) -> Self {
        ListBox {
            items: items.iter().map(|s| s.to_string()).collect(),
            current: 0,
            top: 0,
            focused: false,
            rows: 1,
        }
    }

    pub fn set_rows(&mut self, rows: i16) {
        self.rows = rows.max(1);
        self.follow();
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
}

impl StaticText {
    pub fn new(text: &str) -> Self {
        StaticText {
            text: text.to_string(),
        }
    }

    /// Broken to a width, keeping words whole.
    pub fn lines(&self, width: i16) -> Vec<String> {
        let w = width.max(4) as usize;
        let mut out = Vec::new();
        for para in self.text.split('\n') {
            let mut cur = String::new();
            for word in para.split_whitespace() {
                if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > w {
                    out.push(std::mem::take(&mut cur));
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
