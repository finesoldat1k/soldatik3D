//! Engine-owned CPU assets shared by every rendering backend.
//!
//! This crate has no graphics API dependency. It provides procedural and
//! built-in data (textures, the bitmap font atlas, the environment cubemap)
//! that backends upload to the GPU. Games can also generate their own
//! [`TextureData`] and place it in a material.

mod environment;
mod font;
mod texture;

pub use environment::generate_environment_cubemap;
pub use font::{
    FONT_ATLAS_SIZE, FONT_CELL_SIZE, FONT_GLYPH_COLUMNS, FONT_GLYPH_HEIGHT, FONT_GLYPH_WIDTH,
    font_atlas_rgba8,
};
pub use texture::TextureData;
