//! Geometry primitives shared by layout, widgets and the draw list.

use glam::Vec2;

/// Axis-aligned rectangle in screen pixels (top-left origin).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };

    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w: w.max(0.0),
            h: h.max(0.0),
        }
    }

    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self::new(min.x, min.y, size.x, size.y)
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.w
            && point.y < self.y + self.h
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn min(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn max(&self) -> Vec2 {
        Vec2::new(self.x + self.w, self.y + self.h)
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.w, self.h)
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn inset(&self, margin: f32) -> Self {
        self.inset_edges(EdgeInsets::all(margin))
    }

    pub fn inset_edges(&self, edges: EdgeInsets) -> Self {
        Self::new(
            self.x + edges.left,
            self.y + edges.top,
            (self.w - edges.left - edges.right).max(0.0),
            (self.h - edges.top - edges.bottom).max(0.0),
        )
    }

    pub fn intersect(&self, other: Rect) -> Option<Rect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        (x0 < x1 && y0 < y1).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }

    pub fn union(&self, other: Rect) -> Rect {
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = self.right().max(other.right());
        let y1 = self.bottom().max(other.bottom());
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    pub fn translated(&self, offset: Vec2) -> Rect {
        Rect::new(self.x + offset.x, self.y + offset.y, self.w, self.h)
    }

    pub fn with_position(&self, position: Vec2) -> Rect {
        Rect::new(position.x, position.y, self.w, self.h)
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }
}

/// Four-sided inset used for padding and margins.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    pub const fn all(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }

    pub const fn horizontal(value: f32) -> Self {
        Self {
            left: value,
            top: 0.0,
            right: value,
            bottom: 0.0,
        }
    }

    pub const fn vertical(value: f32) -> Self {
        Self {
            left: 0.0,
            top: value,
            right: 0.0,
            bottom: value,
        }
    }

    pub fn horizontal_sum(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical_sum(&self) -> f32 {
        self.top + self.bottom
    }

    pub fn main_sum(&self, row: bool) -> f32 {
        if row {
            self.horizontal_sum()
        } else {
            self.vertical_sum()
        }
    }
}
