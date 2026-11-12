use glam::{Mat4, Vec3};

/// Maximum number of local lights uploaded to the forward pass.
pub const MAX_LIGHTS: usize = 8;

/// A local point light.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub position: Vec3,
    pub color: Vec3,
    pub radius: f32,
    pub intensity: f32,
}

impl Light {
    pub fn new(position: Vec3, color: Vec3, radius: f32) -> Self {
        Self {
            position,
            color,
            radius,
            intensity: 1.0,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ShadowSettings {
    /// Half-size of the orthographic shadow frustum in world units.
    pub ortho_half_extent: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            ortho_half_extent: 50.0,
            near: 0.1,
            far: 100.0,
        }
    }
}

/// The single shadow-casting light of the scene. It also acts as the primary
/// direct light in the PBR shader.
#[derive(Clone, Copy, Debug)]
pub struct PrimaryLight {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub shadow: ShadowSettings,
}

impl Default for PrimaryLight {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 10.0, 0.0),
            target: Vec3::ZERO,
            up: Vec3::Z,
            color: Vec3::ONE,
            intensity: 1.0,
            shadow: ShadowSettings::default(),
        }
    }
}

impl PrimaryLight {
    /// View-projection used by the shadow map. The Vulkan Y-flip is applied by
    /// the renderer, so this stays backend-agnostic.
    pub fn view_projection(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let h = self.shadow.ortho_half_extent.max(0.01);
        let projection = Mat4::orthographic_rh(-h, h, -h, h, self.shadow.near, self.shadow.far);
        projection * view
    }
}
