//! Coordinates. Everything is measured in character cells, never pixels.
//!
//! `i16` is deliberate: it is the natural word on the smallest machine we
//! intend to run on, and no terminal is 32767 columns wide.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Point {
    pub const fn new(x: i16, y: i16) -> Self {
        Point { x, y }
    }
}

/// A rectangle given by its top-left corner and its size.
///
/// Turbo Vision stored two corners (`TRect.A` / `TRect.B`); we store origin
/// plus extent because every layout calculation we do is about sizes, and
/// `w`/`h` are then never a subtraction away.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

impl Rect {
    pub const fn new(x: i16, y: i16, w: i16, h: i16) -> Self {
        Rect { x, y, w, h }
    }

    pub const fn sized(w: i16, h: i16) -> Self {
        Rect { x: 0, y: 0, w, h }
    }

    pub const fn right(&self) -> i16 {
        self.x + self.w
    }

    pub const fn bottom(&self) -> i16 {
        self.y + self.h
    }

    pub const fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    /// The overlap of two rectangles, or an empty rectangle if they miss.
    pub fn intersect(&self, other: &Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        Rect {
            x,
            y,
            w: (r - x).max(0),
            h: (b - y).max(0),
        }
    }

    /// Same rectangle moved by `(dx, dy)`.
    pub fn offset(&self, dx: i16, dy: i16) -> Rect {
        Rect {
            x: self.x + dx,
            y: self.y + dy,
            ..*self
        }
    }

    /// Same rectangle shrunk by `n` cells on every side.
    pub fn inset(&self, n: i16) -> Rect {
        Rect {
            x: self.x + n,
            y: self.y + n,
            w: (self.w - 2 * n).max(0),
            h: (self.h - 2 * n).max(0),
        }
    }
}
