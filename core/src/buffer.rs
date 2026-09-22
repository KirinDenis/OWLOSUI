//! The screen: a flat grid of cells, plus clipped ways of writing into it.
//!
//! Every drawing call takes an explicit clip rectangle. Views never get a
//! sub-buffer of their own and never learn where they sit on screen — the
//! parent hands down absolute coordinates and a clip, and a child that draws
//! outside it simply has nothing happen. That is the whole occlusion story:
//! draw back to front, clip on the way down.

use crate::cell::{attr, Cell, Color};
use crate::geom::Rect;

pub struct Buffer {
    w: i16,
    h: i16,
    cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(w: i16, h: i16) -> Self {
        let w = w.max(0);
        let h = h.max(0);
        Buffer {
            w,
            h,
            cells: vec![Cell::new(b' ', attr(Color::LightGray, Color::Black)); (w as usize) * (h as usize)],
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
        self.cells.clear();
        self.cells.resize(
            (w as usize) * (h as usize),
            Cell::new(b' ', attr(Color::LightGray, Color::Black)),
        );
    }

    pub fn get(&self, x: i16, y: i16) -> Option<Cell> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        Some(self.cells[(y as usize) * (self.w as usize) + (x as usize)])
    }

    /// Write one cell. Silently does nothing outside the buffer or the clip.
    pub fn put(&mut self, x: i16, y: i16, ch: u8, a: u8, clip: Rect) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if !clip.contains(crate::geom::Point::new(x, y)) {
            return;
        }
        self.cells[(y as usize) * (self.w as usize) + (x as usize)] = Cell::new(ch, a);
    }

    /// Recolour one cell without touching its glyph. Used for selection.
    pub fn recolor(&mut self, x: i16, y: i16, a: u8, clip: Rect) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if !clip.contains(crate::geom::Point::new(x, y)) {
            return;
        }
        self.cells[(y as usize) * (self.w as usize) + (x as usize)].attr = a;
    }

    /// Write raw code page bytes.
    pub fn raw(&mut self, x: i16, y: i16, bytes: &[u8], a: u8, clip: Rect) {
        for (i, b) in bytes.iter().enumerate() {
            self.put(x + i as i16, y, *b, a, clip);
        }
    }

    /// Write text. Characters outside ASCII are drawn as `?` — mapping Unicode
    /// onto the active code page is the caller's business, not the grid's.
    pub fn text(&mut self, x: i16, y: i16, s: &str, a: u8, clip: Rect) -> i16 {
        let mut n = 0i16;
        for c in s.chars() {
            let b = if (c as u32) < 128 { c as u8 } else { b'?' };
            self.put(x + n, y, b, a, clip);
            n += 1;
        }
        n
    }

    pub fn fill(&mut self, r: Rect, ch: u8, a: u8, clip: Rect) {
        let r = r.intersect(&clip);
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.put(x, y, ch, a, clip);
            }
        }
    }

    pub fn hline(&mut self, x: i16, y: i16, len: i16, ch: u8, a: u8, clip: Rect) {
        for i in 0..len {
            self.put(x + i, y, ch, a, clip);
        }
    }

    pub fn vline(&mut self, x: i16, y: i16, len: i16, ch: u8, a: u8, clip: Rect) {
        for i in 0..len {
            self.put(x, y + i, ch, a, clip);
        }
    }
}
