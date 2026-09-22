//! A hexadecimal view of a file.
//!
//! A separate view and not a mode on the editor, and the deciding thing is the
//! data rather than the drawing. An editor's content is *lines*; this one's is
//! a flat run of bytes with no lines in it at all. Forcing a binary into a
//! list of lines is exactly the lie a hex view exists to stop you telling —
//! the moment you do it, a file with no newline in it has one line of four
//! megabytes, and every operation that thought in lines is wrong.
//!
//! What they share — a window, a scrollbar, a caret, a place in the tree —
//! they share by both being views, which is what the tree is for.

use crate::geom::Rect;

pub struct HexView {
    pub bytes: Vec<u8>,
    /// First visible row.
    pub top: i16,
    /// The byte under the caret.
    pub cursor: usize,
    rows: i16,
}

/// Sixteen bytes to a row, always, with a wider gap in the middle.
///
/// Not worked out from the width, which is what this did first and was wrong.
/// The whole point of sixteen is that **the low nibble of the address is the
/// column number**: you read `000001A0` at the left, count along, and know the
/// byte you are looking at is at `1A7` without doing any arithmetic. Change
/// the row length with the window and that stops being true, and a hex dump
/// whose addresses cannot be read at a glance is a hex dump doing none of its
/// job.
///
/// The gap after the eighth is the same idea at a smaller scale: it halves the
/// distance the eye has to count.
pub const PER_ROW: i16 = 16;

/// Where byte `i` starts, measured from the left of the view.
pub const fn byte_x(i: i16) -> i16 {
    10 + i * 3 + if i >= 8 { 1 } else { 0 }
}

/// Where the text column's left bar sits.
pub const fn text_bar_x() -> i16 {
    byte_x(PER_ROW - 1) + 3
}

impl HexView {
    pub fn new(bytes: Vec<u8>) -> Self {
        HexView {
            bytes,
            top: 0,
            cursor: 0,
            rows: 1,
        }
    }

    pub fn per_row(&self) -> i16 {
        PER_ROW
    }

    pub fn rows(&self) -> i16 {
        self.rows
    }

    pub fn total_rows(&self) -> i16 {
        let n = self.bytes.len() as i64;
        let p = PER_ROW as i64;
        (((n + p - 1) / p).max(1)).min(i16::MAX as i64) as i16
    }

    /// Only the height matters. The row is a fixed shape and a window too
    /// narrow for it shows less of the row, not a different row.
    pub fn layout(&mut self, r: Rect) {
        self.rows = r.h.max(1);
        self.follow();
    }

    pub fn row_of(&self, at: usize) -> i16 {
        (at / PER_ROW as usize) as i16
    }

    fn follow(&mut self) {
        let row = self.row_of(self.cursor);
        if row < self.top {
            self.top = row;
        } else if row >= self.top + self.rows {
            self.top = row - self.rows + 1;
        }
        self.top = self.top.clamp(0, (self.total_rows() - self.rows).max(0));
    }

    pub fn step(&mut self, delta: i64) {
        let n = self.bytes.len() as i64;
        if n == 0 {
            return;
        }
        self.cursor = (self.cursor as i64 + delta).clamp(0, n - 1) as usize;
        self.follow();
    }

    pub fn scroll(&mut self, rows: i16) {
        self.top = (self.top + rows).clamp(0, (self.total_rows() - self.rows).max(0));
    }

    pub fn home(&mut self) {
        self.cursor = 0;
        self.follow();
    }

    pub fn end(&mut self) {
        self.cursor = self.bytes.len().saturating_sub(1);
        self.follow();
    }

    /// What the footer says: where we are and what is under the caret.
    pub fn info(&self) -> String {
        match self.bytes.get(self.cursor) {
            Some(b) => format!(
                " {:08X}  {:02X}  {}  ",
                self.cursor,
                b,
                if b.is_ascii_graphic() {
                    (*b as char).to_string()
                } else {
                    ".".into()
                }
            ),
            None => " empty ".into(),
        }
    }
}

/// Is this a file to read or a file to look at?
///
/// A NUL byte settles it: text does not contain one, and every format that
/// does is a format no editor should open by accident. Failing that, a run of
/// control characters that are not tabs or newlines says the same thing more
/// slowly.
///
/// The sample is the first few kilobytes because that is where a file declares
/// itself, and because reading all of a large one to decide how to open it is
/// work done twice.
pub fn looks_binary(bytes: &[u8]) -> bool {
    let sample = &bytes[..bytes.len().min(8192)];
    if sample.is_empty() {
        return false;
    }
    let mut odd = 0usize;
    for &b in sample {
        if b == 0 {
            return true;
        }
        if b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r' {
            odd += 1;
        }
    }
    odd * 10 > sample.len()
}
