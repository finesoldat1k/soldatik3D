use assets::TextureData;

/// The material currently supported by the forward PBR pass.
///
/// A single material is shared by all meshes of a [`crate::Scene`]. Per-mesh
/// materials are a planned extension; the backend contract already keeps the
/// material separate from the geometry so it can grow without breaking games.
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
