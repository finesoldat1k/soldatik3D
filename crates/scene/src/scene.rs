use crate::geometry::Mesh;
use crate::light::{Light, PrimaryLight};
use crate::material::Material;

/// Everything the renderer needs to build its immutable GPU representation of
/// the world. Per-frame state (camera, UI) is passed separately in
/// [`crate::render::FrameData`].
#[derive(Clone, Debug)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub lights: Vec<Light>,
    pub primary_light: PrimaryLight,
    pub material: Material,
    pub clear_color: [f32; 4],
}

impl Scene {
    pub fn new(material: Material) -> Self {
        Self {
            meshes: Vec::new(),
            lights: Vec::new(),
            primary_light: PrimaryLight::default(),
            material,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn with_meshes(mut self, meshes: Vec<Mesh>) -> Self {
        self.meshes = meshes;
        self
    }

    pub fn with_lights(mut self, lights: Vec<Light>) -> Self {
        self.lights = lights;
        self
    }

    pub fn with_primary_light(mut self, light: PrimaryLight) -> Self {
        self.primary_light = light;
        self
    }
}
