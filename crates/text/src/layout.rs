//! Text style and layout output types.

use crate::font::FontId;

/// Horizontal alignment within the text's layout box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Description of how a string should be laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Explicit font, or `None` for the text system's default font.
    pub font: Option<FontId>,
    /// Pixel size (not a multiplier).
    pub size: f32,
    /// Straight RGBA colour. The text system stores it for convenience; the
    /// renderer can override it per draw call.
    pub color: [f32; 4],
    pub align: TextAlign,
    /// Line-height multiplier. `0.0` uses the font's own metrics.
    pub line_height: f32,
    /// Optional soft-wrap width in pixels.
    pub wrap_width: Option<f32>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: None,
            size: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
            align: TextAlign::Left,
            line_height: 0.0,
            wrap_width: None,
        }
    }
}

impl TextStyle {
    pub fn new(size: f32) -> Self {
        Self {
            size,
            ..Self::default()
        }
    }

    pub fn with_font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn line_height(mut self, multiplier: f32) -> Self {
        self.line_height = multiplier;
        self
    }

    pub fn wrap(mut self, width: f32) -> Self {
        self.wrap_width = Some(width.max(1.0));
        self
    }
}

/// A glyph placed in text-local pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionedGlyph {
    pub font: FontId,
    pub glyph: u16,
    /// Top-left of the glyph bitmap relative to the text origin.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Normalized atlas UV rect: `[u0, v0, u1, v1]`.
    pub uv: [f32; 4],
}

/// One laid-out line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextLine {
    pub width: f32,
    /// Baseline Y relative to the text origin.
    pub baseline: f32,
    pub glyphs: Vec<PositionedGlyph>,
}

/// Result of laying out a string.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextLayout {
    pub width: f32,
    pub height: f32,
    pub lines: Vec<TextLine>,
    pub glyph_count: usize,
}

impl TextLayout {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() || self.width <= 0.0 || self.height <= 0.0
    }
}

/// Result of measuring a string without producing glyph quads.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub line_count: usize,
    pub ascent: f32,
    pub descent: f32,
}
