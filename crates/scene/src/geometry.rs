use glam::{Mat4, Quat, Vec3};

/// A 3D transform with position, rotation, and scale.
///
/// Provides convenient conversion between local and global coordinate spaces.
/// Uses row-major convention matching glam's Mat4 layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Creates a new transform with the given position (identity rotation, unit scale).
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Creates a new transform with position and rotation (unit scale).
    pub fn from_pos_rot(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            ..Default::default()
        }
    }

    /// Creates a new transform with position, rotation, and scale.
    pub fn from_pos_rot_scale(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    /// Returns the transformation matrix (local to global).
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Returns the inverse transformation matrix (global to local).
    pub fn inverse_matrix(&self) -> Mat4 {
        self.matrix().inverse()
    }

    /// Converts a local-space point to global space.
    pub fn local_to_global(&self, local_point: Vec3) -> Vec3 {
        self.matrix().transform_point3(local_point)
    }

    /// Converts a global-space point to local space.
    pub fn global_to_local(&self, global_point: Vec3) -> Vec3 {
        self.inverse_matrix().transform_point3(global_point)
    }

    /// Converts a local-space direction vector to global space.
    pub fn local_to_global_dir(&self, local_dir: Vec3) -> Vec3 {
        self.matrix().transform_vector3(local_dir)
    }

    /// Converts a global-space direction vector to local space.
    pub fn global_to_local_dir(&self, global_dir: Vec3) -> Vec3 {
        self.inverse_matrix().transform_vector3(global_dir)
    }

    /// Returns the forward vector in global space (local -Z).
    pub fn forward(&self) -> Vec3 {
        self.rotation * -Vec3::Z
    }

    /// Returns the right vector in global space (local +X).
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Returns the up vector in global space (local +Y).
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Creates a transform looking at a target from a position.
    pub fn looking_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - eye).normalize_or_zero();
        let right = up.cross(forward).normalize_or_zero();
        let up = forward.cross(right);
        let rotation = Quat::from_mat3(&glam::Mat3::from_cols(right, up, forward));
        Self::from_pos_rot(eye, rotation)
    }
}

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
    /// Per-vertex metallic/roughness override. A negative component means
    /// "use the scene material" for that property, so geometry can mix
    /// polished metal, rough concrete and painted surfaces in one draw call.
    pub material: [f32; 2],
}

impl Vertex {
    pub const STRIDE: u32 = std::mem::size_of::<Self>() as u32;
    pub const POSITION_OFFSET: u32 = 0;
    pub const COLOR_OFFSET: u32 = 12;
    pub const NORMAL_OFFSET: u32 = 24;
    pub const UV_OFFSET: u32 = 36;
    pub const EMISSION_OFFSET: u32 = 44;
    pub const MATERIAL_OFFSET: u32 = 48;

    pub fn new(position: [f32; 3], color: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            color,
            normal,
            uv,
            emission: 0.0,
            material: [-1.0, -1.0],
        }
    }

    pub fn with_emission(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }

    /// Overrides the scene metallic/roughness for this vertex.
    pub fn with_material(mut self, metallic: f32, roughness: f32) -> Self {
        self.material = [metallic, roughness];
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

/// Primitive mesh builders for common shapes.
///
/// These functions generate vertices with proper normals, UVs, and optional
/// material overrides. All shapes are centered at the origin in local space;
/// use a `Transform` to position them in the world.
pub mod primitives {
    use super::*;
    use std::f32::consts::{PI, TAU};

    /// Creates a unit cube (size 1.0) centered at origin.
    /// Returns (vertices, AABB).
    pub fn cube(
        color: [f32; 3],
        metallic: f32,
        roughness: f32,
        emission: f32,
    ) -> (Vec<Vertex>, Aabb) {
        let s = 0.5;
        let positions = [
            // Front (+Z)
            Vec3::new(-s, -s, s),
            Vec3::new(s, -s, s),
            Vec3::new(s, s, s),
            Vec3::new(-s, s, s),
            // Back (-Z)
            Vec3::new(s, -s, -s),
            Vec3::new(-s, -s, -s),
            Vec3::new(-s, s, -s),
            Vec3::new(s, s, -s),
            // Right (+X)
            Vec3::new(s, -s, s),
            Vec3::new(s, -s, -s),
            Vec3::new(s, s, -s),
            Vec3::new(s, s, s),
            // Left (-X)
            Vec3::new(-s, -s, -s),
            Vec3::new(-s, -s, s),
            Vec3::new(-s, s, s),
            Vec3::new(-s, s, -s),
            // Top (+Y)
            Vec3::new(-s, s, s),
            Vec3::new(s, s, s),
            Vec3::new(s, s, -s),
            Vec3::new(-s, s, -s),
            // Bottom (-Y)
            Vec3::new(-s, -s, -s),
            Vec3::new(s, -s, -s),
            Vec3::new(s, -s, s),
            Vec3::new(-s, -s, s),
        ];
        let normals = [
            Vec3::Z,
            Vec3::Z,
            Vec3::Z,
            Vec3::Z,
            -Vec3::Z,
            -Vec3::Z,
            -Vec3::Z,
            -Vec3::Z,
            Vec3::X,
            Vec3::X,
            Vec3::X,
            Vec3::X,
            -Vec3::X,
            -Vec3::X,
            -Vec3::X,
            -Vec3::X,
            Vec3::Y,
            Vec3::Y,
            Vec3::Y,
            Vec3::Y,
            -Vec3::Y,
            -Vec3::Y,
            -Vec3::Y,
            -Vec3::Y,
        ];
        let uvs = [
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ];
        let indices = [
            0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16,
            17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23,
        ];

        let mut vertices = Vec::with_capacity(36);
        for &i in &indices {
            vertices.push(
                Vertex::new(
                    positions[i].to_array(),
                    color,
                    normals[i].to_array(),
                    uvs[i],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
        }

        (
            vertices,
            Aabb::from_center_half_extents(Vec3::ZERO, Vec3::splat(s)),
        )
    }

    /// Creates a cube with custom size (half-extents).
    pub fn cube_sized(
        half_extents: Vec3,
        color: [f32; 3],
        metallic: f32,
        roughness: f32,
        emission: f32,
    ) -> (Vec<Vertex>, Aabb) {
        let positions = [
            // Front (+Z)
            Vec3::new(-half_extents.x, -half_extents.y, half_extents.z),
            Vec3::new(half_extents.x, -half_extents.y, half_extents.z),
            Vec3::new(half_extents.x, half_extents.y, half_extents.z),
            Vec3::new(-half_extents.x, half_extents.y, half_extents.z),
            // Back (-Z)
            Vec3::new(half_extents.x, -half_extents.y, -half_extents.z),
            Vec3::new(-half_extents.x, -half_extents.y, -half_extents.z),
            Vec3::new(-half_extents.x, half_extents.y, -half_extents.z),
            Vec3::new(half_extents.x, half_extents.y, -half_extents.z),
            // Right (+X)
            Vec3::new(half_extents.x, -half_extents.y, half_extents.z),
            Vec3::new(half_extents.x, -half_extents.y, -half_extents.z),
            Vec3::new(half_extents.x, half_extents.y, -half_extents.z),
            Vec3::new(half_extents.x, half_extents.y, half_extents.z),
            // Left (-X)
            Vec3::new(-half_extents.x, -half_extents.y, -half_extents.z),
            Vec3::new(-half_extents.x, -half_extents.y, half_extents.z),
            Vec3::new(-half_extents.x, half_extents.y, half_extents.z),
            Vec3::new(-half_extents.x, half_extents.y, -half_extents.z),
            // Top (+Y)
            Vec3::new(-half_extents.x, half_extents.y, half_extents.z),
            Vec3::new(half_extents.x, half_extents.y, half_extents.z),
            Vec3::new(half_extents.x, half_extents.y, -half_extents.z),
            Vec3::new(-half_extents.x, half_extents.y, -half_extents.z),
            // Bottom (-Y)
            Vec3::new(-half_extents.x, -half_extents.y, -half_extents.z),
            Vec3::new(half_extents.x, -half_extents.y, -half_extents.z),
            Vec3::new(half_extents.x, -half_extents.y, half_extents.z),
            Vec3::new(-half_extents.x, -half_extents.y, half_extents.z),
        ];
        let normals = [
            Vec3::Z,
            Vec3::Z,
            Vec3::Z,
            Vec3::Z,
            -Vec3::Z,
            -Vec3::Z,
            -Vec3::Z,
            -Vec3::Z,
            Vec3::X,
            Vec3::X,
            Vec3::X,
            Vec3::X,
            -Vec3::X,
            -Vec3::X,
            -Vec3::X,
            -Vec3::X,
            Vec3::Y,
            Vec3::Y,
            Vec3::Y,
            Vec3::Y,
            -Vec3::Y,
            -Vec3::Y,
            -Vec3::Y,
            -Vec3::Y,
        ];
        let uvs = [
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ];
        let indices = [
            0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16,
            17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23,
        ];

        let mut vertices = Vec::with_capacity(36);
        for &i in &indices {
            vertices.push(
                Vertex::new(
                    positions[i].to_array(),
                    color,
                    normals[i].to_array(),
                    uvs[i],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
        }

        (
            vertices,
            Aabb::from_center_half_extents(Vec3::ZERO, half_extents),
        )
    }

    /// Creates a sphere with given radius, segments (longitude), rings (latitude).
    pub fn sphere(
        radius: f32,
        segments: usize,
        rings: usize,
        color: [f32; 3],
        metallic: f32,
        roughness: f32,
        emission: f32,
    ) -> (Vec<Vertex>, Aabb) {
        let segments = segments.max(3);
        let rings = rings.max(2);
        let mut vertices = Vec::new();

        for ring in 0..rings {
            let phi0 = PI * ring as f32 / rings as f32;
            let phi1 = PI * (ring + 1) as f32 / rings as f32;
            let (sin0, cos0) = phi0.sin_cos();
            let (sin1, cos1) = phi1.sin_cos();

            for seg in 0..segments {
                let theta0 = TAU * seg as f32 / segments as f32;
                let theta1 = TAU * (seg + 1) as f32 / segments as f32;
                let (sin_t0, cos_t0) = theta0.sin_cos();
                let (sin_t1, cos_t1) = theta1.sin_cos();

                let v00 = Vec3::new(cos_t0 * sin0, cos0, sin_t0 * sin0);
                let v10 = Vec3::new(cos_t1 * sin0, cos0, sin_t1 * sin0);
                let v11 = Vec3::new(cos_t1 * sin1, cos1, sin_t1 * sin1);
                let v01 = Vec3::new(cos_t0 * sin1, cos1, sin_t0 * sin1);

                let n00 = v00;
                let n10 = v10;
                let n11 = v11;
                let n01 = v01;

                let u0 = seg as f32 / segments as f32;
                let u1 = (seg + 1) as f32 / segments as f32;
                let v0 = ring as f32 / rings as f32;
                let v1 = (ring + 1) as f32 / rings as f32;

                // Quad as two triangles
                vertices.push(
                    Vertex::new((v00 * radius).to_array(), color, n00.to_array(), [u0, v0])
                        .with_emission(emission)
                        .with_material(metallic, roughness),
                );
                vertices.push(
                    Vertex::new((v10 * radius).to_array(), color, n10.to_array(), [u1, v0])
                        .with_emission(emission)
                        .with_material(metallic, roughness),
                );
                vertices.push(
                    Vertex::new((v11 * radius).to_array(), color, n11.to_array(), [u1, v1])
                        .with_emission(emission)
                        .with_material(metallic, roughness),
                );
                vertices.push(
                    Vertex::new((v00 * radius).to_array(), color, n00.to_array(), [u0, v0])
                        .with_emission(emission)
                        .with_material(metallic, roughness),
                );
                vertices.push(
                    Vertex::new((v11 * radius).to_array(), color, n11.to_array(), [u1, v1])
                        .with_emission(emission)
                        .with_material(metallic, roughness),
                );
                vertices.push(
                    Vertex::new((v01 * radius).to_array(), color, n01.to_array(), [u0, v1])
                        .with_emission(emission)
                        .with_material(metallic, roughness),
                );
            }
        }

        (
            vertices,
            Aabb::from_center_half_extents(Vec3::ZERO, Vec3::splat(radius)),
        )
    }

    /// Creates a cylinder along Y axis.
    pub fn cylinder(
        radius: f32,
        height: f32,
        segments: usize,
        color: [f32; 3],
        metallic: f32,
        roughness: f32,
        emission: f32,
    ) -> (Vec<Vertex>, Aabb) {
        let segments = segments.max(3);
        let half_h = height * 0.5;
        let mut vertices = Vec::new();

        // Side faces
        for i in 0..segments {
            let a0 = TAU * i as f32 / segments as f32;
            let a1 = TAU * (i + 1) as f32 / segments as f32;
            let (sin0, cos0) = a0.sin_cos();
            let (sin1, cos1) = a1.sin_cos();

            let n0 = Vec3::new(cos0, 0.0, sin0);
            let n1 = Vec3::new(cos1, 0.0, sin1);

            let p00 = Vec3::new(cos0 * radius, -half_h, sin0 * radius);
            let p10 = Vec3::new(cos1 * radius, -half_h, sin1 * radius);
            let p11 = Vec3::new(cos1 * radius, half_h, sin1 * radius);
            let p01 = Vec3::new(cos0 * radius, half_h, sin0 * radius);

            let u0 = i as f32 / segments as f32;
            let u1 = (i + 1) as f32 / segments as f32;

            vertices.push(
                Vertex::new(p00.to_array(), color, n0.to_array(), [u0, 0.0])
                    .with_emission(emission)
                    .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(p10.to_array(), color, n1.to_array(), [u1, 0.0])
                    .with_emission(emission)
                    .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(p11.to_array(), color, n1.to_array(), [u1, 1.0])
                    .with_emission(emission)
                    .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(p00.to_array(), color, n0.to_array(), [u0, 0.0])
                    .with_emission(emission)
                    .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(p11.to_array(), color, n1.to_array(), [u1, 1.0])
                    .with_emission(emission)
                    .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(p01.to_array(), color, n0.to_array(), [u0, 1.0])
                    .with_emission(emission)
                    .with_material(metallic, roughness),
            );
        }

        // Top and bottom caps
        let center_top = Vec3::new(0.0, half_h, 0.0);
        let center_bottom = Vec3::new(0.0, -half_h, 0.0);
        let normal_top = Vec3::Y;
        let normal_bottom = -Vec3::Y;

        for i in 0..segments {
            let a0 = TAU * i as f32 / segments as f32;
            let a1 = TAU * (i + 1) as f32 / segments as f32;
            let (sin0, cos0) = a0.sin_cos();
            let (sin1, cos1) = a1.sin_cos();

            // Top cap
            let v0 = Vec3::new(cos0 * radius, half_h, sin0 * radius);
            let v1 = Vec3::new(cos1 * radius, half_h, sin1 * radius);
            vertices.push(
                Vertex::new(
                    center_top.to_array(),
                    color,
                    normal_top.to_array(),
                    [0.5, 0.5],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(
                    v1.to_array(),
                    color,
                    normal_top.to_array(),
                    [(cos1 + 1.0) * 0.5, (sin1 + 1.0) * 0.5],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(
                    v0.to_array(),
                    color,
                    normal_top.to_array(),
                    [(cos0 + 1.0) * 0.5, (sin0 + 1.0) * 0.5],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );

            // Bottom cap (wound opposite)
            let v0 = Vec3::new(cos0 * radius, -half_h, sin0 * radius);
            let v1 = Vec3::new(cos1 * radius, -half_h, sin1 * radius);
            vertices.push(
                Vertex::new(
                    center_bottom.to_array(),
                    color,
                    normal_bottom.to_array(),
                    [0.5, 0.5],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(
                    v0.to_array(),
                    color,
                    normal_bottom.to_array(),
                    [(cos0 + 1.0) * 0.5, (sin0 + 1.0) * 0.5],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
            vertices.push(
                Vertex::new(
                    v1.to_array(),
                    color,
                    normal_bottom.to_array(),
                    [(cos1 + 1.0) * 0.5, (sin1 + 1.0) * 0.5],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
        }

        (
            vertices,
            Aabb::from_center_half_extents(Vec3::ZERO, Vec3::new(radius, half_h, radius)),
        )
    }

    /// Creates a plane on XZ axis (horizontal).
    pub fn plane(
        half_extents: Vec3,
        color: [f32; 3],
        metallic: f32,
        roughness: f32,
        emission: f32,
    ) -> (Vec<Vertex>, Aabb) {
        let x = half_extents.x;
        let z = half_extents.z;
        let y = 0.0;

        let positions = [
            Vec3::new(-x, y, z),
            Vec3::new(x, y, z),
            Vec3::new(x, y, -z),
            Vec3::new(-x, y, -z),
        ];
        let normals = [Vec3::Y; 4];
        let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let indices = [0, 1, 2, 0, 2, 3];

        let mut vertices = Vec::with_capacity(6);
        for &i in &indices {
            vertices.push(
                Vertex::new(
                    positions[i].to_array(),
                    color,
                    normals[i].to_array(),
                    uvs[i],
                )
                .with_emission(emission)
                .with_material(metallic, roughness),
            );
        }

        (
            vertices,
            Aabb::from_center_half_extents(Vec3::ZERO, Vec3::new(x, 0.0, z)),
        )
    }
}
