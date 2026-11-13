//! RGBA8 pixel data used to build a GPU texture.
//!
//! Texture upload is backend-specific, but the CPU-side representation is
//! engine-owned so games and tools can generate or load textures once and
//! hand them to any backend.

/// RGBA8 pixel data used to build a GPU texture.
#[derive(Clone, Debug)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl TextureData {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        assert_eq!(
            pixels.len(),
            (width * height * 4) as usize,
            "RGBA texture data has an invalid size"
        );
        Self {
            width,
            height,
            pixels,
        }
    }

    /// A 1x1 opaque white texture, useful as a neutral default.
    pub fn white() -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![255, 255, 255, 255],
        }
    }

    /// Builds a RGBA8 texture from an RGB8 buffer.
    pub fn from_rgb8(width: u32, height: u32, rgb: &[u8]) -> Self {
        assert_eq!(
            rgb.len(),
            (width * height * 3) as usize,
            "RGB texture data has an invalid size"
        );
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for chunk in rgb.chunks_exact(3) {
            pixels.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
        }
        Self::new(width, height, pixels)
    }
}
