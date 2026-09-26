//! Editing.
//!
//! Two decisions shape this file, and both were taken by looking at how
//! Borland did it.
//!
//! **Every change to the text is one operation.** `splice` replaces a span
//! with new lines and hands back what was there. Typing a character, pressing
//! Enter, Backspace, deleting a selection, pasting — all of them are a splice,
//! so undo is written once and is correct for commands that do not exist yet.
//! The alternative is an undo case per command, and the case somebody forgets
//! is the one that corrupts a file.
//!
//! **Commands are named and separate from keys.** The editor knows
//! `Cmd::WordRight`; it does not know Ctrl+Right. That is how Borland shipped
//! four keymaps for one editor, it is how we can ship an authentic one and a
//! modern one, and it is why a test can be a script of command names rather
//! than a simulation of somebody's fingers.

use crate::cell::Glyph;
use crate::geom::Point;
use crate::views::TextView;

/// What an editor can be asked to do. Keys are somebody else's problem.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    CharLeft,
    CharRight,
    LineUp,
    LineDown,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    PageUp,
    PageDown,
    TextStart,
    TextEnd,

    Insert(char),
    NewLine,
    DeleteLeft,
    DeleteRight,
    DeleteLine,

    SelectAll,
    SelectNone,

    Cut,
    Copy,
    Paste,

    Undo,
    Redo,
}

impl Cmd {
    /// Whether this is a movement — the commands that extend a selection when
    /// Shift is held, and only those.
    pub fn is_movement(self) -> bool {
        use Cmd::*;
        matches!(
            self,
            CharLeft
                | CharRight
                | LineUp
                | LineDown
                | WordLeft
                | WordRight
                | LineStart
                | LineEnd
                | PageUp
                | PageDown
                | TextStart
                | TextEnd
        )
    }
}

/// One change, kept so it can be taken back.
///
/// It describes a splice: replace the span `from`..`to` with `text`. Applying
/// it returns the same shape describing how to undo it, which is why redo
/// costs nothing extra.
#[derive(Clone)]
pub struct Edit {
    from: Point,
    to: Point,
    text: Vec<Vec<Glyph>>,
    /// Where the caret was before, so undo puts it back where the typist left
    /// it rather than where the machine finished.
    cur: Point,
}

fn before(a: Point, b: Point) -> bool {
    (a.y, a.x) < (b.y, b.x)
}

fn ordered(a: Point, b: Point) -> (Point, Point) {
    if before(b, a) {
        (b, a)
    } else {
        (a, b)
    }
}

fn is_word(b: Glyph) -> bool {
    // ASCII letters and digits, and `_`. Beyond ASCII every glyph is a
    // letter: the core cannot tell a Cyrillic letter from a box-drawing
    // piece, and a word boundary in the wrong place is the smaller sin.
    b >= 128 || (b as u8).is_ascii_alphanumeric() || b == b'_' as Glyph
}

impl TextView {
    // ---------------------------------------------------------------- geometry

    // Lengths and counts come back as `i16` because that is what a caret is
    // made of, and a caret is that small because a screen is. A line longer
    // than 32767 bytes, or a file with more lines than that, is therefore
    // seen through a ceiling: the caret stops at the ceiling rather than
    // wrapping round to a negative number and taking the program with it.
    // (The hex view exists for exactly those files; this is only about not
    // falling over when one is opened as text by mistake.)

    fn line_len(&self, y: i16) -> i16 {
        self.lines.get(y as usize).map_or(0, |l| l.len().min(i16::MAX as usize) as i16)
    }

    fn last_line(&self) -> i16 {
        (self.lines.len().min(i16::MAX as usize) as i16 - 1).max(0)
    }

    /// Put the caret somewhere that exists. A caret past the end of a line is
    /// a whole class of bug on its own; it is cheaper to make it impossible.
    fn clamp_cur(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        self.cur.y = self.cur.y.clamp(0, self.last_line());
        self.cur.x = self.cur.x.clamp(0, self.line_len(self.cur.y));
    }

    /// The selected span, if there is one.
    pub fn selection(&self) -> Option<(Point, Point)> {
        let a = self.anchor?;
        if a == self.cur {
            return None;
        }
        Some(ordered(a, self.cur))
    }

    pub fn selected_text(&self) -> Vec<Vec<Glyph>> {
        match self.selection() {
            Some((a, b)) => self.extract(a, b),
            None => Vec::new(),
        }
    }

    /// True if this cell is inside the selection — what the renderer asks.
    pub fn is_selected(&self, y: i16, x: i16) -> bool {
        let Some((a, b)) = self.selection() else {
            return false;
        };
        let p = Point::new(x, y);
        !before(p, a) && before(p, b)
    }

    // ------------------------------------------------------------------ splice

    fn extract(&self, a: Point, b: Point) -> Vec<Vec<Glyph>> {
        if a.y == b.y {
            let l = &self.lines[a.y as usize];
            return vec![l[a.x as usize..b.x as usize].to_vec()];
        }
        let mut out = vec![self.lines[a.y as usize][a.x as usize..].to_vec()];
        for y in (a.y + 1)..b.y {
            out.push(self.lines[y as usize].clone());
        }
        out.push(self.lines[b.y as usize][..b.x as usize].to_vec());
        out
    }

    /// Replace everything between two points with `new`, and return what was
    /// there. The single place the text changes.
    fn splice(&mut self, a: Point, b: Point, new: &[Vec<Glyph>]) -> Vec<Vec<Glyph>> {
        let old = self.extract(a, b);
        self.modified = true;

        let prefix = self.lines[a.y as usize][..a.x as usize].to_vec();
        let suffix = self.lines[b.y as usize][b.x as usize..].to_vec();

        let mut rebuilt: Vec<Vec<Glyph>> = Vec::with_capacity(new.len());
        if new.len() <= 1 {
            let mut only = prefix;
            if let Some(n) = new.first() {
                only.extend_from_slice(n);
            }
            only.extend_from_slice(&suffix);
            rebuilt.push(only);
        } else {
            let mut first = prefix;
            first.extend_from_slice(&new[0]);
            rebuilt.push(first);
            for n in &new[1..new.len() - 1] {
                rebuilt.push(n.clone());
            }
            let mut last = new[new.len() - 1].clone();
            last.extend_from_slice(&suffix);
            rebuilt.push(last);
        }

        self.lines
            .splice(a.y as usize..=(b.y as usize), rebuilt);
        old
    }

    /// Where a splice of `text` starting at `a` finishes.
    fn end_of(a: Point, text: &[Vec<Glyph>]) -> Point {
        match text.len() {
            0 => a,
            1 => Point::new(a.x.saturating_add(text[0].len().min(i16::MAX as usize) as i16), a.y),
            n => Point::new(
                text[n - 1].len().min(i16::MAX as usize) as i16,
                a.y.saturating_add(n.min(i16::MAX as usize) as i16 - 1),
            ),
        }
    }

    /// Do an edit and hand back the one that undoes it.
    fn do_edit(&mut self, e: Edit) -> Edit {
        let cur = self.cur;
        let old = self.splice(e.from, e.to, &e.text);
        let to = Self::end_of(e.from, &e.text);
        self.cur = to;
        self.anchor = None;
        self.clamp_cur();
        Edit {
            from: e.from,
            to,
            text: old,
            cur,
        }
    }

    fn change(&mut self, from: Point, to: Point, text: Vec<Vec<Glyph>>) {
        if self.readonly {
            return;
        }
        let (from, to) = ordered(from, to);
        let inverse = self.do_edit(Edit {
            from,
            to,
            text,
            cur: self.cur,
        });
        self.undo_stack.push(inverse);
        self.redo_stack.clear();
    }

    /// Remove the selection if there is one, so an insertion replaces it.
    fn drop_selection(&mut self) {
        if let Some((a, b)) = self.selection() {
            self.change(a, b, vec![Vec::new()]);
        }
        self.anchor = None;
    }

    // ---------------------------------------------------------------- commands

    // ------------------------------------------------------------ searching

    /// Find `pat` after the caret (after the selection, when the selection
    /// is a match already, so "find next" moves on) and select it. False
    /// when there is no next one: the search does not wrap, and a dialog
    /// that wants to ask "from the top?" can move the caret and ask again.
    pub fn find(&mut self, pat: &[Glyph], case_sensitive: bool, whole_word: bool) -> bool {
        if pat.is_empty() {
            return false;
        }
        let start = match self.selection() {
            Some((a, b)) if self.matches_at(a, pat, case_sensitive, whole_word) => {
                let _ = b;
                Point::new(a.x + 1, a.y)
            }
            _ => self.cur,
        };
        let Some(at) = self.find_from(start, pat, case_sensitive, whole_word) else {
            return false;
        };
        let end = Point::new(at.x + pat.len() as i16, at.y);
        self.anchor = Some(at);
        self.cur = end;
        self.follow_caret(1);
        true
    }

    /// Replace the selection, if it is a match, and find the next. Returns
    /// (replaced, found next).
    pub fn replace(&mut self, pat: &[Glyph], with: &[Glyph], case_sensitive: bool, whole_word: bool) -> (bool, bool) {
        let replaced = match self.selection() {
            Some((a, b)) if self.matches_at(a, pat, case_sensitive, whole_word) => {
                self.change(a, b, vec![with.to_vec()]);
                true
            }
            _ => false,
        };
        let found = self.find(pat, case_sensitive, whole_word);
        (replaced, found)
    }

    /// Replace every match from the top down; how many.
    pub fn replace_all(&mut self, pat: &[Glyph], with: &[Glyph], case_sensitive: bool, whole_word: bool) -> u16 {
        if pat.is_empty() || self.readonly {
            return 0;
        }
        let mut n: u16 = 0;
        let mut from = Point::new(0, 0);
        while let Some(at) = self.find_from(from, pat, case_sensitive, whole_word) {
            let end = Point::new(at.x + pat.len() as i16, at.y);
            self.change(at, end, vec![with.to_vec()]);
            from = Point::new(at.x + with.len() as i16, at.y);
            n = n.saturating_add(1);
            if n == u16::MAX {
                break;
            }
        }
        n
    }

    fn glyph_eq(a: Glyph, b: Glyph, case_sensitive: bool) -> bool {
        if case_sensitive || a >= 128 || b >= 128 {
            a == b
        } else {
            (a as u8).eq_ignore_ascii_case(&(b as u8))
        }
    }

    fn matches_at(&self, at: Point, pat: &[Glyph], case_sensitive: bool, whole_word: bool) -> bool {
        let Some(line) = self.lines.get(at.y as usize) else { return false };
        let x = at.x as usize;
        if x + pat.len() > line.len() {
            return false;
        }
        if !line[x..x + pat.len()].iter().zip(pat).all(|(&a, &b)| Self::glyph_eq(a, b, case_sensitive)) {
            return false;
        }
        if whole_word {
            let before = x > 0 && is_word(line[x - 1]);
            let after = x + pat.len() < line.len() && is_word(line[x + pat.len()]);
            if before || after {
                return false;
            }
        }
        true
    }

    fn find_from(&self, from: Point, pat: &[Glyph], case_sensitive: bool, whole_word: bool) -> Option<Point> {
        for y in from.y.max(0)..self.lines.len() as i16 {
            let line = &self.lines[y as usize];
            let start = if y == from.y { from.x.max(0) as usize } else { 0 };
            if pat.len() > line.len() {
                continue;
            }
            for x in start..=line.len() - pat.len() {
                let at = Point::new(x as i16, y);
                if self.matches_at(at, pat, case_sensitive, whole_word) {
                    return Some(at);
                }
            }
        }
        None
    }

    /// Run one command. `page` is the height of the view, needed by PageUp and
    /// PageDown and by nothing else; `extend` is Shift being held.
    pub fn exec(&mut self, cmd: Cmd, extend: bool, page: i16, clip: &mut Vec<Vec<Glyph>>) {
        use Cmd::*;

        // A viewer answers to movement and to copying, and to nothing that
        // would change the text. Refusing in `change` alone was not enough:
        // Backspace walks the caret back *before* asking to delete, and a
        // refused delete left the caret moved.
        if self.readonly && !cmd.is_movement() && !matches!(cmd, SelectAll | SelectNone | Copy) {
            return;
        }

        // Selection bookkeeping, in one place rather than in every movement.
        if cmd.is_movement() {
            if extend {
                if self.anchor.is_none() {
                    self.anchor = Some(self.cur);
                }
            } else {
                self.anchor = None;
            }
        }

        match cmd {
            CharLeft => {
                if self.cur.x > 0 {
                    self.cur.x -= 1;
                } else if self.cur.y > 0 {
                    self.cur.y -= 1;
                    self.cur.x = self.line_len(self.cur.y);
                }
            }
            CharRight => {
                if self.cur.x < self.line_len(self.cur.y) {
                    self.cur.x += 1;
                } else if self.cur.y < self.last_line() {
                    self.cur.y += 1;
                    self.cur.x = 0;
                }
            }
            LineUp => self.cur.y = (self.cur.y - 1).max(0),
            LineDown => self.cur.y = self.cur.y.saturating_add(1).min(self.last_line()),
            PageUp => self.cur.y = self.cur.y.saturating_sub(page).max(0),
            PageDown => self.cur.y = self.cur.y.saturating_add(page).min(self.last_line()),
            LineStart => self.cur.x = 0,
            LineEnd => self.cur.x = self.line_len(self.cur.y),
            TextStart => self.cur = Point::new(0, 0),
            TextEnd => {
                self.cur.y = self.last_line();
                self.cur.x = self.line_len(self.cur.y);
            }

            WordLeft => {
                self.exec(CharLeft, extend, page, clip);
                while self.cur.x > 0 && !is_word(self.lines[self.cur.y as usize][self.cur.x as usize - 1])
                {
                    self.cur.x -= 1;
                }
                while self.cur.x > 0 && is_word(self.lines[self.cur.y as usize][self.cur.x as usize - 1])
                {
                    self.cur.x -= 1;
                }
            }
            WordRight => {
                let len = self.line_len(self.cur.y);
                while self.cur.x < len && is_word(self.lines[self.cur.y as usize][self.cur.x as usize]) {
                    self.cur.x += 1;
                }
                while self.cur.x < len && !is_word(self.lines[self.cur.y as usize][self.cur.x as usize])
                {
                    self.cur.x += 1;
                }
                if self.cur.x == len && self.cur.y < self.last_line() {
                    self.cur.y += 1;
                    self.cur.x = 0;
                }
            }

            Insert(c) => {
                self.drop_selection();
                // A typed character arrives as its glyph index, a char up
                // to U+00FF put there by the backend's code page; anything
                // above that has no glyph and is typed as `?`.
                let b = crate::cell::glyph_of(c);
                let at = self.cur;
                self.change(at, at, vec![vec![b]]);
            }
            NewLine => {
                self.drop_selection();
                let at = self.cur;
                self.change(at, at, vec![Vec::new(), Vec::new()]);
            }
            DeleteLeft => {
                if self.selection().is_some() {
                    self.drop_selection();
                } else if self.cur.x > 0 || self.cur.y > 0 {
                    let to = self.cur;
                    self.exec(CharLeft, false, page, clip);
                    let from = self.cur;
                    self.change(from, to, vec![Vec::new()]);
                }
            }
            DeleteRight => {
                if self.selection().is_some() {
                    self.drop_selection();
                } else {
                    let from = self.cur;
                    let to = if self.cur.x < self.line_len(self.cur.y) {
                        Point::new(self.cur.x + 1, self.cur.y)
                    } else if self.cur.y < self.last_line() {
                        Point::new(0, self.cur.y + 1)
                    } else {
                        return;
                    };
                    self.change(from, to, vec![Vec::new()]);
                }
            }
            DeleteLine => {
                let y = self.cur.y;
                let (from, to) = if y < self.last_line() {
                    (Point::new(0, y), Point::new(0, y + 1))
                } else {
                    (Point::new(0, y), Point::new(self.line_len(y), y))
                };
                self.change(from, to, vec![Vec::new()]);
                self.cur = Point::new(0, y.min(self.last_line()));
            }

            SelectAll => {
                self.anchor = Some(Point::new(0, 0));
                self.cur = Point::new(self.line_len(self.last_line()), self.last_line());
            }
            SelectNone => self.anchor = None,

            Copy => {
                if self.selection().is_some() {
                    *clip = self.selected_text();
                }
            }
            Cut => {
                if self.selection().is_some() {
                    *clip = self.selected_text();
                    self.drop_selection();
                }
            }
            Paste => {
                if !clip.is_empty() {
                    self.drop_selection();
                    let at = self.cur;
                    let text = clip.clone();
                    self.change(at, at, text);
                }
            }

            Undo => {
                if let Some(e) = self.undo_stack.pop() {
                    let cur = e.cur;
                    let inverse = self.do_edit(e);
                    self.redo_stack.push(inverse);
                    self.cur = cur;
                }
            }
            Redo => {
                if let Some(e) = self.redo_stack.pop() {
                    let inverse = self.do_edit(e);
                    self.undo_stack.push(inverse);
                }
            }
        }

        self.clamp_cur();
        self.follow_caret(page);
    }

    /// Scroll only as far as it takes to see the caret. Anything more and the
    /// text jumps under the typist's hands.
    fn follow_caret(&mut self, page: i16) {
        if self.cur.y < self.top {
            self.top = self.cur.y;
        } else if self.cur.y >= self.top + page {
            self.top = self.cur.y - page + 1;
        }
        self.top = self.top.max(0);
    }

    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.iter().map(|&g| char::from_u32(g as u32).unwrap_or('?')).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
