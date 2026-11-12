use glam::Vec3;

/// A single world-space vertex.
///
/// The memory layout is part of the public engine/renderer contract and is
/// mirrored by the vertex input declarations in the Vulkan backend.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub emission: f32,
}

impl Vertex {
    pub const STRIDE: u32 = std::mem::size_of::<Self>() as u32;
    pub const POSITION_OFFSET: u32 = 0;
    pub const COLOR_OFFSET: u32 = 12;
    pub const NORMAL_OFFSET: u32 = 24;
    pub const UV_OFFSET: u32 = 36;
    pub const EMISSION_OFFSET: u32 = 44;

    pub fn new(position: [f32; 3], color: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            color,
            normal,
            uv,
            emission: 0.0,
        }
    }

    pub fn with_emission(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }
}

/// Axis-aligned bounding box. Used for culling and by gameplay code for
/// collision shapes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const ZERO: Self = Self {
        min: Vec3::ZERO,
        max: Vec3::ZERO,
    };

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

/// A drawable piece of geometry together with its bounds.
#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub bounds: Aabb,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, bounds: Aabb) -> Self {
        Self { vertices, bounds }
    }
}
