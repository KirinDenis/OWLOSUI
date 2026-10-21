//! The screen: a flat grid of cells, plus clipped ways of writing into it.
//!
//! Every drawing call takes an explicit clip rectangle. Views never get a
//! sub-buffer of their own and never learn where they sit on screen — the
//! parent hands down absolute coordinates and a clip, and a child that draws
//! outside it simply has nothing happen. That is the whole occlusion story:
//! draw back to front, clip on the way down.
//!
//! A cell can also be a *hole*: drawn as the window's own background, and
//! marked as showing what lies under the screen - a page's emulator picture
//! behind the canvas. Whatever is drawn over a hole afterwards, a menu, a
//! shadow, another window, writes the cell the ordinary way and the mark
//! goes with it, so the marks left at the end of a frame are exactly the
//! hole cells nothing covers. A screen with nothing underneath shows the
//! background and never asks.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::cell::{attr, glyph, Cell, Color, Glyph};
use crate::geom::Rect;

pub struct Buffer {
    w: i16,
    h: i16,
    cells: Vec<Cell>,
    /// One flag a cell, true where it is a hole; empty while there is
    /// none, which is every frame of a program that never makes one.
    holes: Vec<bool>,
}

impl Buffer {
    pub fn new(w: i16, h: i16) -> Self {
        let w = w.max(0);
        let h = h.max(0);
        Buffer {
            w,
            h,
            cells: vec![Cell::new(glyph::SPACE, attr(Color::LightGray, Color::Black)); (w as usize) * (h as usize)],
            holes: Vec::new(),
        }
    }

    pub fn width(&self) -> i16 {
        self.w
    }

    pub fn height(&self) -> i16 {
        self.h
    }

    pub fn rect(&self) -> Rect {
        Rect::sized(self.w, self.h)
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn resize(&mut self, w: i16, h: i16) {
        let w = w.max(0);
        let h = h.max(0);
        if w == self.w && h == self.h {
            return;
        }
        self.w = w;
        self.h = h;
        self.holes.clear();
        self.cells.clear();
        self.cells.resize(
            (w as usize) * (h as usize),
            Cell::new(glyph::SPACE, attr(Color::LightGray, Color::Black)),
        );
    }

    pub fn get(&self, x: i16, y: i16) -> Option<Cell> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        Some(self.cells[(y as usize) * (self.w as usize) + (x as usize)])
    }

    /// Write one cell. Silently does nothing outside the buffer or the clip.
    pub fn put(&mut self, x: i16, y: i16, ch: impl Into<Glyph>, a: u8, clip: Rect) {
        let ch: Glyph = ch.into();
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if !clip.contains(crate::geom::Point::new(x, y)) {
            return;
        }
        let i = (y as usize) * (self.w as usize) + (x as usize);
        self.cells[i] = Cell::new(ch, a);
        if !self.holes.is_empty() {
            self.holes[i] = false;
        }
    }

    /// Make one cell a hole: a space in `a`, the background a screen with
    /// nothing under it shows, marked to show what is under the screen
    /// instead. Silently does nothing outside the buffer or the clip.
    pub fn hole(&mut self, x: i16, y: i16, a: u8, clip: Rect) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if !clip.contains(crate::geom::Point::new(x, y)) {
            return;
        }
        let i = (y as usize) * (self.w as usize) + (x as usize);
        self.cells[i] = Cell::new(glyph::SPACE, a);
        if self.holes.is_empty() {
            self.holes.resize(self.cells.len(), false);
        }
        self.holes[i] = true;
    }

    /// Whether a cell is a hole that nothing has been drawn over.
    pub fn is_hole(&self, x: i16, y: i16) -> bool {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || self.holes.is_empty() {
            return false;
        }
        self.holes[(y as usize) * (self.w as usize) + (x as usize)]
    }

    /// Whether any cell is a hole: a screen with none skips looking.
    pub fn has_holes(&self) -> bool {
        self.holes.iter().any(|&h| h)
    }

    /// Every mark taken off: before a frame is drawn into a buffer that
    /// held the last one.
    pub fn clear_holes(&mut self) {
        self.holes.clear();
    }

    /// Recolour one cell without touching its glyph. Used for selection.
    pub fn recolor(&mut self, x: i16, y: i16, a: u8, clip: Rect) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if !clip.contains(crate::geom::Point::new(x, y)) {
            return;
        }
        let i = (y as usize) * (self.w as usize) + (x as usize);
        self.cells[i].attr = a;
        // A shadow over a hole is a shadow: the cell is the screen's again.
        if !self.holes.is_empty() {
            self.holes[i] = false;
        }
    }

    /// Write raw code page bytes.
    pub fn raw(&mut self, x: i16, y: i16, bytes: &[Glyph], a: u8, clip: Rect) {
        for (i, b) in bytes.iter().enumerate() {
            self.put(x + i as i16, y, *b, a, clip);
        }
    }

    /// Write text. Characters outside ASCII are drawn as `?` — mapping Unicode
    /// onto the active code page is the caller's business, not the grid's.
    /// Write a string of glyph indices.
    ///
    /// A `String` in the core is a byte string: each `char` is the glyph
    /// index itself, `U+0000..U+00FF`, put there by the backend's code page
    /// on the way in. So a char up to 255 is written as that byte, and
    /// anything above it - which no backend should have let in - becomes
    /// `?` rather than a hole. Nothing here knows what code page it is.
    pub fn text(&mut self, x: i16, y: i16, s: &str, a: u8, clip: Rect) -> i16 {
        let mut n = 0i16;
        for c in s.chars() {
            self.put(x + n, y, crate::cell::glyph_of(c), a, clip);
            n += 1;
        }
        n
    }

    pub fn fill(&mut self, r: Rect, ch: impl Into<Glyph>, a: u8, clip: Rect) {
        let ch: Glyph = ch.into();
        let r = r.intersect(&clip);
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.put(x, y, ch, a, clip);
            }
        }
    }

    pub fn hline(&mut self, x: i16, y: i16, len: i16, ch: impl Into<Glyph>, a: u8, clip: Rect) {
        let ch: Glyph = ch.into();
        for i in 0..len {
            self.put(x + i, y, ch, a, clip);
        }
    }

    pub fn vline(&mut self, x: i16, y: i16, len: i16, ch: impl Into<Glyph>, a: u8, clip: Rect) {
        let ch: Glyph = ch.into();
        for i in 0..len {
            self.put(x, y + i, ch, a, clip);
        }
    }
}
