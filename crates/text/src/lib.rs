//! Dependency-free TrueType text engine.
//!
//! The crate owns everything between a `.ttf` file and positioned glyph
//! quads: SFNT parsing, `cmap` lookup, quadratic outline extraction, an
//! anti-aliased scanline rasterizer, a growable R8 glyph atlas and text
//! layout. It deliberately has no rendering-backend dependency, so the same
//! text stack can be used by the UI, the console, debug overlays or 3D text.
//!
//! # Quick start
//!
//! ```
//! use text::{TextStyle, TextSystem};
//!
//! let mut text = TextSystem::new();
//! let layout = text.layout("Привет, Soldatik3D", &TextStyle::new(18.0));
//! assert!(layout.width > 0.0);
//! ```
//!
//! Only TrueType outlines (`glyf`/`loca`) are supported. OpenType/CFF fonts
//! return [`FontError::Unsupported`] with a clear message instead of silently
//! drawing garbage.

pub mod atlas;
pub mod font;
pub mod layout;
pub mod raster;
pub mod system;

pub use atlas::{AtlasUpdate, TextureRect};
pub use font::{Font, FontData, FontError, FontId, FontMetrics, Outline};
pub use layout::{PositionedGlyph, TextAlign, TextLayout, TextLine, TextStyle};
pub use system::TextSystem;

/// Re-exported for callers that do not want to depend on `glam` directly.
pub use glam::Vec2;
