//! Font parsing and glyph outline access.

mod cmap;
pub mod embedded;
mod glyf;
mod kern;
mod sfnt;

use std::fmt;
use std::path::Path;

pub use glyf::{Outline, OutlineContour, OutlinePoint};
pub use sfnt::FontData;

/// Error produced while loading or decoding a font.
#[derive(Debug)]
pub enum FontError {
    /// The file is not a valid SFNT/TrueType font.
    Invalid(String),
    /// The font uses a feature this engine does not implement yet.
    Unsupported(String),
    /// Reading the font from disk failed.
    Io(std::io::Error),
}

impl FontError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(message.into())
    }
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "invalid font: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported font: {message}"),
            Self::Io(error) => write!(f, "font I/O error: {error}"),
        }
    }
}

impl std::error::Error for FontError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for FontError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Stable index of a font inside a [`TextSystem`](crate::TextSystem).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(pub u32);

impl FontId {
    /// The default embedded font (Open Sans Regular).
    pub const DEFAULT: Self = Self(0);
    /// The embedded bold face (Open Sans Bold), when available.
    pub const BOLD: Self = Self(1);
}

/// A parsed font face.
#[derive(Debug)]
pub struct Font {
    /// Identifier assigned by the owning text system.
    pub id: FontId,
    /// Human-readable family name from the `name` table.
    pub family: String,
    /// Human-readable style name from the `name` table.
    pub style: String,
    /// Parsed table data.
    pub data: FontData,
}

impl Font {
    /// Parses a TTF from memory.
    pub fn from_bytes(id: FontId, bytes: Vec<u8>) -> Result<Self, FontError> {
        let data = FontData::parse(bytes)?;
        Ok(Self {
            id,
            family: data.family_name.clone(),
            style: data.style_name.clone(),
            data,
        })
    }

    /// Loads and parses a TTF from disk.
    pub fn from_file(id: FontId, path: impl AsRef<Path>) -> Result<Self, FontError> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(id, bytes)
    }

    /// Glyph index for a Unicode scalar, or `None` when the font has no glyph.
    pub fn glyph_index(&self, ch: char) -> Option<u16> {
        self.data.glyph_index(ch)
    }

    /// Horizontal advance of a glyph in font units.
    pub fn advance(&self, glyph: u16) -> f32 {
        self.data.advance(glyph)
    }

    /// Legacy `kern` pair adjustment in font units.
    pub fn kern(&self, left: u16, right: u16) -> f32 {
        self.data.kern(left, right)
    }

    /// Extracts the glyph outline in font units.
    pub fn outline(&self, glyph: u16) -> Result<Outline, FontError> {
        self.data.outline(glyph)
    }

    /// Vertical metrics scaled to `size_px`.
    pub fn metrics(&self, size_px: f32) -> FontMetrics {
        let scale = size_px / self.data.units_per_em.max(1) as f32;
        let ascent = self.data.ascender as f32 * scale;
        let descent = self.data.descender as f32 * scale;
        let line_gap = self.data.line_gap as f32 * scale;
        FontMetrics {
            size_px,
            ascent,
            descent,
            line_gap,
            line_height: (ascent - descent + line_gap).max(size_px),
            units_per_em: self.data.units_per_em,
        }
    }
}

/// Vertical metrics of a font at a concrete pixel size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontMetrics {
    pub size_px: f32,
    /// Distance from the baseline to the top of the em box (positive).
    pub ascent: f32,
    /// Distance from the baseline to the bottom of the em box (negative).
    pub descent: f32,
    pub line_gap: f32,
    /// Recommended distance between baselines.
    pub line_height: f32,
    pub units_per_em: u16,
}
