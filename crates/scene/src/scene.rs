use crate::geometry::Mesh;
use crate::light::{Light, PrimaryLight};
use crate::material::Material;

/// Everything the renderer needs to build its immutable GPU representation of
/// the world. Per-frame state (camera, UI) is passed separately by the runtime.
///
/// A scene is uploaded once when the renderer is created. Camera motion and UI
/// are per-frame data, so gameplay can switch cameras freely without touching
/// GPU buffers.
#[derive(Clone, Debug)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub lights: Vec<Light>,
    pub primary_light: PrimaryLight,
    pub material: Material,
    pub clear_color: [f32; 4],
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            meshes: Vec::new(),
            lights: Vec::new(),
            primary_light: PrimaryLight::default(),
            material: Material::default(),
            clear_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl Scene {
    pub fn new(material: Material) -> Self {
        Self {
            material,
            ..Self::default()
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

    pub fn with_clear_color(mut self, color: [f32; 4]) -> Self {
        self.clear_color = color;
        self
    }

    pub fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(mesh);
    }

    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }
}
