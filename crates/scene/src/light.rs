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
    /// View-projection used by the shadow map. This is the engine's
    /// OpenGL-convention matrix ([−1, 1] depth); Vulkan converts it at the
    /// backend boundary.
    pub fn view_projection(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let h = self.shadow.ortho_half_extent.max(0.01);
        let projection = Mat4::orthographic_rh(-h, h, -h, h, self.shadow.near, self.shadow.far);
        projection * view
    }
}

/// std140-compatible light buffer shared by all backends.
///
/// The layout is deliberately two separate `vec4` arrays, mirroring the GLSL
/// `LightBuffer` block. Interleaving position and color in one array of
/// structs produces colored corruption because std140 pads struct members to
/// 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniformBuffer {
    pub position_radius: [[f32; 4]; MAX_LIGHTS],
    pub color_intensity: [[f32; 4]; MAX_LIGHTS],
}

impl Default for LightUniformBuffer {
    fn default() -> Self {
        Self {
            position_radius: [[0.0; 4]; MAX_LIGHTS],
            color_intensity: [[0.0; 4]; MAX_LIGHTS],
        }
    }
}

impl LightUniformBuffer {
    /// Packs the nearest `MAX_LIGHTS` lights around `camera_position`.
    ///
    /// Selecting the closest lights keeps the forward pass relevant in large
    /// levels without a light grid. Both backends call this helper, so the
    /// selection policy is centralized.
    pub fn from_nearest(lights: &[Light], camera_position: Vec3) -> Self {
        let mut buffer = Self::default();
        let mut nearest: Vec<(f32, usize)> = lights
            .iter()
            .enumerate()
            .map(|(index, light)| (light.position.distance_squared(camera_position), index))
            .collect();
        nearest.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (slot, (_, index)) in nearest.iter().take(MAX_LIGHTS).enumerate() {
            let light = lights[*index];
            buffer.position_radius[slot] = [
                light.position.x,
                light.position.y,
                light.position.z,
                light.radius,
            ];
            buffer.color_intensity[slot] =
                [light.color.x, light.color.y, light.color.z, light.intensity];
        }
        buffer
    }
}
