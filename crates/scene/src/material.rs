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

    pub fn white() -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![255, 255, 255, 255],
        }
    }
}

/// The material currently supported by the forward PBR pass.
#[derive(Clone, Debug)]
pub struct Material {
    pub albedo: TextureData,
    pub metallic: f32,
    pub roughness: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            albedo: TextureData::white(),
            metallic: 0.0,
            roughness: 0.9,
        }
    }
}
