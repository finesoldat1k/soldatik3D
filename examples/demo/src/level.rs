//! The test polygon: procedural geometry for exercising the renderer.
//!
//! Layout (top-down, north is −Z):
//!
//! ```text
//!          [north deck + stairs]        [flight tower]
//!
//!   [parkour course]   [central plaza]   [light gallery]
//!
//!                            [step lanes]      [tower]
//! ```
//!
//! Everything is built once at startup. The mesh builder emits render meshes
//! and collision boxes together, and switches per-vertex metallic/roughness
//! values between areas so one draw call can mix concrete, painted surfaces and
//! polished metal. There are no gameplay goals here: the polygon exists to
//! compare materials, lighting, shadows, bloom and traversal in one place.

use std::f32::consts::{PI, TAU};

use runtime::glam::Vec3;
use runtime::{
    Aabb, Light, Material, Mesh, PrimaryLight, Scene, ShadowSettings, TextureData, Vertex,
};

use crate::physics::PhysicsWorld;

const WALL_T: f32 = 0.4;
/// One albedo tile covers four metres of world space.
const UV_SCALE: f32 = 0.25;

const GROUND: [f32; 3] = [0.42, 0.44, 0.47];
const GRID_LINE: [f32; 3] = [0.24, 0.26, 0.30];
const CONCRETE: [f32; 3] = [0.58, 0.60, 0.63];
const CONCRETE_DARK: [f32; 3] = [0.34, 0.36, 0.40];
const WALL_DARK: [f32; 3] = [0.22, 0.24, 0.28];
const STAIR: [f32; 3] = [0.54, 0.56, 0.60];
const METAL: [f32; 3] = [0.60, 0.64, 0.70];
const STEEL_DARK: [f32; 3] = [0.24, 0.27, 0.32];
const GOLD: [f32; 3] = [0.96, 0.72, 0.30];
const COPPER: [f32; 3] = [0.86, 0.42, 0.24];
const SILVER: [f32; 3] = [0.88, 0.90, 0.94];
const CERAMIC: [f32; 3] = [0.92, 0.90, 0.86];
const PAINT_RED: [f32; 3] = [0.70, 0.14, 0.11];
const PAINT_GREEN: [f32; 3] = [0.16, 0.52, 0.30];
const PAINT_BLUE: [f32; 3] = [0.14, 0.33, 0.60];
const PAINT_ORANGE: [f32; 3] = [0.86, 0.42, 0.10];
const PAINT_WHITE: [f32; 3] = [0.82, 0.84, 0.86];
const EMIT_CYAN: [f32; 3] = [0.10, 0.78, 1.00];
const EMIT_WARM: [f32; 3] = [1.00, 0.56, 0.20];
const EMIT_GREEN: [f32; 3] = [0.22, 1.00, 0.42];
const EMIT_RED: [f32; 3] = [1.00, 0.14, 0.07];

/// Player spawn in the south of the plaza, looking north.
pub const SPAWN: Vec3 = Vec3::new(0.0, 0.002, 24.0);
pub const SPAWN_YAW: f32 = -PI * 0.5;
/// Falling below this height respawns the player at the plaza.
pub const FALL_LIMIT: f32 = -12.0;

/// Everything the demo needs to run and render the polygon.
pub struct TestPolygon {
    pub scene: Scene,
    pub physics: PhysicsWorld,
}

/// Builds the scene, colliders and light rig.
pub fn build_polygon() -> TestPolygon {
    let mut assembly = Assembly::new();
    add_ground(&mut assembly);
    add_perimeter(&mut assembly);
    add_plaza(&mut assembly);
    add_material_study(&mut assembly);
    add_north_deck(&mut assembly);
    add_light_gallery(&mut assembly);
    add_parkour(&mut assembly);
    add_step_lanes(&mut assembly);
    add_flight_tower(&mut assembly);

    let material = Material {
        albedo: TextureData::new(256, 256, generate_surface_texture()),
        metallic: 0.0,
        roughness: 0.9,
    };
    let scene = Scene::new(material)
        .with_meshes(assembly.meshes)
        .with_lights(create_lights())
        .with_primary_light(PrimaryLight {
            position: Vec3::new(34.0, 46.0, 20.0),
            target: Vec3::new(0.0, 0.0, -8.0),
            up: Vec3::Z,
            color: Vec3::new(1.0, 0.94, 0.84),
            intensity: 0.85,
            shadow: ShadowSettings {
                ortho_half_extent: 58.0,
                near: 1.0,
                far: 140.0,
            },
        })
        .with_clear_color([0.012, 0.018, 0.032, 1.0]);

    TestPolygon {
        scene,
        physics: PhysicsWorld::new(assembly.colliders),
    }
}

/// Accumulates render meshes and collision boxes while areas are built.
struct Assembly {
    meshes: Vec<Mesh>,
    colliders: Vec<Aabb>,
}

impl Assembly {
    fn new() -> Self {
        Self {
            meshes: Vec::new(),
            colliders: Vec::new(),
        }
    }

    /// Adds a frustum-culling chunk and merges its colliders.
    fn chunk(&mut self, bounds: Aabb, build: impl FnOnce(&mut ChunkBuilder)) {
        let mut builder = ChunkBuilder::new();
        build(&mut builder);
        self.colliders.extend(builder.colliders);
        if !builder.vertices.is_empty() {
            self.meshes.push(Mesh::new(builder.vertices, bounds));
        }
    }
}

/// Small mesh builder with world-space tiled UVs and a material state.
struct ChunkBuilder {
    vertices: Vec<Vertex>,
    colliders: Vec<Aabb>,
    metallic: f32,
    roughness: f32,
}

impl ChunkBuilder {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            colliders: Vec::new(),
            metallic: -1.0,
            roughness: -1.0,
        }
    }

    /// Sets the metallic/roughness override copied into every following
    /// vertex. `-1.0` lets the scene material supply the value.
    fn set_material(&mut self, metallic: f32, roughness: f32) {
        self.metallic = metallic;
        self.roughness = roughness;
    }

    fn clear_material(&mut self) {
        self.set_material(-1.0, -1.0);
    }

    fn vertex(
        &mut self,
        position: Vec3,
        normal: Vec3,
        uv: [f32; 2],
        color: [f32; 3],
        emission: f32,
    ) {
        self.vertices.push(
            Vertex::new(position.to_array(), color, normal.to_array(), uv)
                .with_emission(emission)
                .with_material(self.metallic, self.roughness),
        );
    }

    fn quad(
        &mut self,
        positions: [Vec3; 4],
        normal: Vec3,
        uvs: [[f32; 2]; 4],
        color: [f32; 3],
        emission: f32,
    ) {
        for index in [0, 1, 2, 0, 2, 3] {
            self.vertex(positions[index], normal, uvs[index], color, emission);
        }
    }

    fn triangle(
        &mut self,
        positions: [Vec3; 3],
        normals: [Vec3; 3],
        uvs: [[f32; 2]; 3],
        color: [f32; 3],
        emission: f32,
    ) {
        for index in 0..3 {
            self.vertex(
                positions[index],
                normals[index],
                uvs[index],
                color,
                emission,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Primitive helpers
// ---------------------------------------------------------------------------

impl ChunkBuilder {
    /// Axis-aligned box with collision.
    fn solid_box(&mut self, center: Vec3, size: Vec3, color: [f32; 3]) {
        self.box_faces(center, size, color, 0.0);
        let half = size.abs() * 0.5;
        self.colliders.push(Aabb::new(center - half, center + half));
    }

    /// Axis-aligned box without collision.
    fn visual_box(&mut self, center: Vec3, size: Vec3, color: [f32; 3]) {
        self.box_faces(center, size, color, 0.0);
    }

    fn emissive_box(&mut self, center: Vec3, size: Vec3, color: [f32; 3], emission: f32) {
        self.box_faces(center, size, color, emission);
    }

    fn box_faces(&mut self, center: Vec3, size: Vec3, color: [f32; 3], emission: f32) {
        let half = size.abs() * 0.5;
        let (x0, x1) = (center.x - half.x, center.x + half.x);
        let (y0, y1) = (center.y - half.y, center.y + half.y);
        let (z0, z1) = (center.z - half.z, center.z + half.z);
        let s = UV_SCALE;
        self.quad(
            [
                Vec3::new(x0, y0, z1),
                Vec3::new(x1, y0, z1),
                Vec3::new(x1, y1, z1),
                Vec3::new(x0, y1, z1),
            ],
            Vec3::Z,
            [
                [x0 * s, y0 * s],
                [x1 * s, y0 * s],
                [x1 * s, y1 * s],
                [x0 * s, y1 * s],
            ],
            color,
            emission,
        );
        self.quad(
            [
                Vec3::new(x1, y0, z0),
                Vec3::new(x0, y0, z0),
                Vec3::new(x0, y1, z0),
                Vec3::new(x1, y1, z0),
            ],
            -Vec3::Z,
            [
                [x1 * s, y0 * s],
                [x0 * s, y0 * s],
                [x0 * s, y1 * s],
                [x1 * s, y1 * s],
            ],
            color,
            emission,
        );
        self.quad(
            [
                Vec3::new(x0, y1, z1),
                Vec3::new(x1, y1, z1),
                Vec3::new(x1, y1, z0),
                Vec3::new(x0, y1, z0),
            ],
            Vec3::Y,
            [
                [x0 * s, z1 * s],
                [x1 * s, z1 * s],
                [x1 * s, z0 * s],
                [x0 * s, z0 * s],
            ],
            color,
            emission,
        );
        self.quad(
            [
                Vec3::new(x0, y0, z0),
                Vec3::new(x1, y0, z0),
                Vec3::new(x1, y0, z1),
                Vec3::new(x0, y0, z1),
            ],
            -Vec3::Y,
            [
                [x0 * s, z0 * s],
                [x1 * s, z0 * s],
                [x1 * s, z1 * s],
                [x0 * s, z1 * s],
            ],
            color,
            emission,
        );
        self.quad(
            [
                Vec3::new(x1, y0, z1),
                Vec3::new(x1, y0, z0),
                Vec3::new(x1, y1, z0),
                Vec3::new(x1, y1, z1),
            ],
            Vec3::X,
            [
                [z1 * s, y0 * s],
                [z0 * s, y0 * s],
                [z0 * s, y1 * s],
                [z1 * s, y1 * s],
            ],
            color,
            emission,
        );
        self.quad(
            [
                Vec3::new(x0, y0, z0),
                Vec3::new(x0, y0, z1),
                Vec3::new(x0, y1, z1),
                Vec3::new(x0, y1, z0),
            ],
            -Vec3::X,
            [
                [z0 * s, y0 * s],
                [z1 * s, y0 * s],
                [z1 * s, y1 * s],
                [z0 * s, y1 * s],
            ],
            color,
            emission,
        );
    }

    fn floor(&mut self, x1: f32, x2: f32, z1: f32, z2: f32, y: f32, color: [f32; 3]) {
        self.floor_surface(x1, x2, z1, z2, y, color);
        self.colliders
            .push(Aabb::new(Vec3::new(x1, y - 0.8, z1), Vec3::new(x2, y, z2)));
    }

    fn floor_surface(&mut self, x1: f32, x2: f32, z1: f32, z2: f32, y: f32, color: [f32; 3]) {
        let s = UV_SCALE;
        self.quad(
            [
                Vec3::new(x1, y, z1),
                Vec3::new(x1, y, z2),
                Vec3::new(x2, y, z2),
                Vec3::new(x2, y, z1),
            ],
            Vec3::Y,
            [
                [x1 * s, z1 * s],
                [x1 * s, z2 * s],
                [x2 * s, z2 * s],
                [x2 * s, z1 * s],
            ],
            color,
            0.0,
        );
    }

    /// Wall at a fixed X. `facing` selects the visible normal (±1).
    #[allow(clippy::too_many_arguments)]
    fn wall_x(&mut self, x: f32, z1: f32, z2: f32, y1: f32, y2: f32, facing: f32, color: [f32; 3]) {
        let s = UV_SCALE;
        let (z1, z2) = (z1.min(z2), z1.max(z2));
        let (y1, y2) = (y1.min(y2), y1.max(y2));
        if facing > 0.0 {
            self.quad(
                [
                    Vec3::new(x, y1, z1),
                    Vec3::new(x, y2, z1),
                    Vec3::new(x, y2, z2),
                    Vec3::new(x, y1, z2),
                ],
                Vec3::X,
                [
                    [z1 * s, y1 * s],
                    [z1 * s, y2 * s],
                    [z2 * s, y2 * s],
                    [z2 * s, y1 * s],
                ],
                color,
                0.0,
            );
        } else {
            self.quad(
                [
                    Vec3::new(x, y1, z2),
                    Vec3::new(x, y2, z2),
                    Vec3::new(x, y2, z1),
                    Vec3::new(x, y1, z1),
                ],
                -Vec3::X,
                [
                    [z2 * s, y1 * s],
                    [z2 * s, y2 * s],
                    [z1 * s, y2 * s],
                    [z1 * s, y1 * s],
                ],
                color,
                0.0,
            );
        }
        self.colliders.push(Aabb::new(
            Vec3::new(x - WALL_T * 0.5, y1, z1),
            Vec3::new(x + WALL_T * 0.5, y2, z2),
        ));
    }

    /// Wall at a fixed Z. `facing` selects the visible normal (±1).
    #[allow(clippy::too_many_arguments)]
    fn wall_z(&mut self, z: f32, x1: f32, x2: f32, y1: f32, y2: f32, facing: f32, color: [f32; 3]) {
        let s = UV_SCALE;
        let (x1, x2) = (x1.min(x2), x1.max(x2));
        let (y1, y2) = (y1.min(y2), y1.max(y2));
        if facing > 0.0 {
            self.quad(
                [
                    Vec3::new(x1, y1, z),
                    Vec3::new(x2, y1, z),
                    Vec3::new(x2, y2, z),
                    Vec3::new(x1, y2, z),
                ],
                Vec3::Z,
                [
                    [x1 * s, y1 * s],
                    [x2 * s, y1 * s],
                    [x2 * s, y2 * s],
                    [x1 * s, y2 * s],
                ],
                color,
                0.0,
            );
        } else {
            self.quad(
                [
                    Vec3::new(x2, y1, z),
                    Vec3::new(x1, y1, z),
                    Vec3::new(x1, y2, z),
                    Vec3::new(x2, y2, z),
                ],
                -Vec3::Z,
                [
                    [x2 * s, y1 * s],
                    [x1 * s, y1 * s],
                    [x1 * s, y2 * s],
                    [x2 * s, y2 * s],
                ],
                color,
                0.0,
            );
        }
        self.colliders.push(Aabb::new(
            Vec3::new(x1, y1, z - WALL_T * 0.5),
            Vec3::new(x2, y2, z + WALL_T * 0.5),
        ));
    }

    /// Straight stair flight ascending towards −Z when `north` is true.
    #[allow(clippy::too_many_arguments)]
    fn stairs_z(
        &mut self,
        x1: f32,
        x2: f32,
        z_start: f32,
        steps: usize,
        rise: f32,
        run: f32,
        north: bool,
        color: [f32; 3],
    ) {
        let direction = if north { -1.0 } else { 1.0 };
        for index in 0..steps {
            let top = rise * (index + 1) as f32;
            let z0 = z_start + direction * run * index as f32;
            let z1 = z0 + direction * run;
            let (zmin, zmax) = (z0.min(z1), z0.max(z1));
            self.solid_box(
                Vec3::new((x1 + x2) * 0.5, top * 0.5, (zmin + zmax) * 0.5),
                Vec3::new(x2 - x1, top, run),
                color,
            );
        }
    }

    /// Railing running along X at a fixed Z. Adds a thin blocking collider.
    fn railing_x(&mut self, z: f32, x1: f32, x2: f32, base_y: f32, color: [f32; 3]) {
        let (x1, x2) = (x1.min(x2), x1.max(x2));
        let span = x2 - x1;
        let posts = ((span / 1.4).ceil() as usize).max(1);
        for index in 0..=posts {
            let x = x1 + span * index as f32 / posts as f32;
            self.visual_box(
                Vec3::new(x, base_y + 0.55, z),
                Vec3::new(0.08, 1.1, 0.08),
                color,
            );
        }
        self.visual_box(
            Vec3::new((x1 + x2) * 0.5, base_y + 1.1, z),
            Vec3::new(span, 0.09, 0.09),
            color,
        );
        self.visual_box(
            Vec3::new((x1 + x2) * 0.5, base_y + 0.6, z),
            Vec3::new(span, 0.06, 0.06),
            color,
        );
        self.colliders.push(Aabb::new(
            Vec3::new(x1, base_y, z - 0.1),
            Vec3::new(x2, base_y + 1.15, z + 0.1),
        ));
    }

    /// Railing running along Z at a fixed X.
    fn railing_z(&mut self, x: f32, z1: f32, z2: f32, base_y: f32, color: [f32; 3]) {
        let (z1, z2) = (z1.min(z2), z1.max(z2));
        let span = z2 - z1;
        let posts = ((span / 1.4).ceil() as usize).max(1);
        for index in 0..=posts {
            let z = z1 + span * index as f32 / posts as f32;
            self.visual_box(
                Vec3::new(x, base_y + 0.55, z),
                Vec3::new(0.08, 1.1, 0.08),
                color,
            );
        }
        self.visual_box(
            Vec3::new(x, base_y + 1.1, (z1 + z2) * 0.5),
            Vec3::new(0.09, 0.09, span),
            color,
        );
        self.visual_box(
            Vec3::new(x, base_y + 0.6, (z1 + z2) * 0.5),
            Vec3::new(0.06, 0.06, span),
            color,
        );
        self.colliders.push(Aabb::new(
            Vec3::new(x - 0.1, base_y, z1),
            Vec3::new(x + 0.1, base_y + 1.15, z2),
        ));
    }

    /// Cylinder between two points, including caps.
    fn cylinder_between(
        &mut self,
        from: Vec3,
        to: Vec3,
        radius: f32,
        sides: usize,
        color: [f32; 3],
        emission: f32,
    ) {
        let sides = sides.max(3);
        let axis = to - from;
        let length = axis.length();
        if length < 1e-4 {
            return;
        }
        let axis = axis / length;
        let reference = if axis.dot(Vec3::Y).abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let u = axis.cross(reference).normalize();
        let v = axis.cross(u).normalize();
        let s = UV_SCALE;
        let circumference = TAU * radius * s;
        for index in 0..sides {
            let a = TAU * index as f32 / sides as f32;
            let b = TAU * (index + 1) as f32 / sides as f32;
            let (sa, ca) = a.sin_cos();
            let (sb, cb) = b.sin_cos();
            let normal_a = u * ca + v * sa;
            let normal_b = u * cb + v * sb;
            let a0 = from + normal_a * radius;
            let a1 = to + normal_a * radius;
            let b0 = from + normal_b * radius;
            let b1 = to + normal_b * radius;
            let ua = circumference * index as f32 / sides as f32;
            let ub = circumference * (index + 1) as f32 / sides as f32;
            self.quad(
                [a0, a1, b1, b0],
                (normal_a + normal_b).normalize(),
                [[ua, 0.0], [ua, length * s], [ub, length * s], [ub, 0.0]],
                color,
                emission,
            );
        }
        // Caps, fanned from the centre.
        for (center, normal, reverse) in [(from, -axis, true), (to, axis, false)] {
            for index in 0..sides {
                let a = TAU * index as f32 / sides as f32;
                let b = TAU * (index + 1) as f32 / sides as f32;
                let p0 = center + (u * a.cos() + v * a.sin()) * radius;
                let p1 = center + (u * b.cos() + v * b.sin()) * radius;
                let points = if reverse {
                    [center, p1, p0]
                } else {
                    [center, p0, p1]
                };
                let uvs = [
                    [center.x * s, center.z * s],
                    [p0.x * s, p0.z * s],
                    [p1.x * s, p1.z * s],
                ];
                self.triangle(points, [normal; 3], uvs, color, emission);
            }
        }
    }

    /// Cylinder standing on `base`, extending up by `height`.
    fn cylinder_y(
        &mut self,
        base: Vec3,
        radius: f32,
        height: f32,
        sides: usize,
        color: [f32; 3],
        emission: f32,
    ) {
        self.cylinder_between(
            base,
            base + Vec3::Y * height,
            radius,
            sides,
            color,
            emission,
        );
    }

    /// UV sphere with smooth normals. Used for the material-study props.
    fn sphere(
        &mut self,
        center: Vec3,
        radius: f32,
        segments: usize,
        rings: usize,
        color: [f32; 3],
    ) {
        let segments = segments.max(3);
        let rings = rings.max(2);
        for ring in 0..rings {
            let t0 = ring as f32 / rings as f32;
            let t1 = (ring + 1) as f32 / rings as f32;
            let phi0 = PI * t0;
            let phi1 = PI * t1;
            let (sin0, cos0) = phi0.sin_cos();
            let (sin1, cos1) = phi1.sin_cos();
            for segment in 0..segments {
                let a0 = TAU * segment as f32 / segments as f32;
                let a1 = TAU * (segment + 1) as f32 / segments as f32;
                let (sa0, ca0) = a0.sin_cos();
                let (sa1, ca1) = a1.sin_cos();
                let normals = [
                    Vec3::new(cos0 * ca0, sin0, cos0 * sa0),
                    Vec3::new(cos1 * ca0, sin1, cos1 * sa0),
                    Vec3::new(cos1 * ca1, sin1, cos1 * sa1),
                    Vec3::new(cos0 * ca1, sin0, cos0 * sa1),
                ];
                let positions = normals.map(|normal| center + normal * radius);
                let u0 = segment as f32 / segments as f32;
                let u1 = (segment + 1) as f32 / segments as f32;
                let uvs = [[u0, t0], [u0, t1], [u1, t1], [u1, t0]];
                for index in [0, 1, 2, 0, 2, 3] {
                    self.vertex(positions[index], normals[index], uvs[index], color, 0.0);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Areas
// ---------------------------------------------------------------------------

fn add_ground(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-41.0, -1.0, -41.0), Vec3::new(41.0, 0.2, 41.0)),
        |b| {
            b.clear_material();
            b.floor(-40.0, 40.0, -40.0, 40.0, 0.0, GROUND);

            // Five-metre alignment grid. It reads as scale reference in every
            // screenshot and makes camera movement easy to judge.
            b.set_material(0.0, 0.95);
            let mut x = -40.0;
            while x <= 40.0 {
                b.visual_box(
                    Vec3::new(x, 0.006, 0.0),
                    Vec3::new(0.045, 0.012, 80.0),
                    GRID_LINE,
                );
                x += 5.0;
            }
            let mut z = -40.0;
            while z <= 40.0 {
                b.visual_box(
                    Vec3::new(0.0, 0.006, z),
                    Vec3::new(80.0, 0.012, 0.045),
                    GRID_LINE,
                );
                z += 5.0;
            }

            // A slightly lighter plaza inset with a dark border. Both are
            // coplanar decals just above the ground, so they do not create a
            // step for the character controller.
            b.set_material(-1.0, -1.0);
            b.floor_surface(-16.0, 16.0, -6.0, 20.0, 0.014, [0.48, 0.50, 0.53]);
            b.set_material(0.0, 0.8);
            b.floor_surface(-16.3, 16.3, -6.0, 20.0, 0.018, [0.22, 0.24, 0.28]);
            b.floor_surface(-16.0, 16.0, -6.0, -5.7, 0.022, [0.52, 0.54, 0.57]);
        },
    );
}

fn add_perimeter(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-41.0, 0.0, -41.0), Vec3::new(41.0, 11.0, 41.0)),
        |b| {
            b.set_material(0.0, 0.82);
            b.wall_x(-40.0, -40.0, 40.0, 0.0, 10.0, 1.0, CONCRETE_DARK);
            b.wall_x(40.0, -40.0, 40.0, 0.0, 10.0, -1.0, CONCRETE_DARK);
            b.wall_z(-40.0, -40.0, 40.0, 0.0, 10.0, 1.0, CONCRETE_DARK);
            b.wall_z(40.0, -40.0, 40.0, 0.0, 10.0, -1.0, CONCRETE_DARK);

            // Interior pilasters break up the long walls and catch grazing
            // sunlight, which makes the shadow bias easy to inspect.
            b.set_material(0.0, 0.7);
            let mut offset = -36.0;
            while offset <= 36.0 {
                b.visual_box(
                    Vec3::new(-39.6, 5.0, offset),
                    Vec3::new(0.8, 10.0, 1.1),
                    CONCRETE,
                );
                b.visual_box(
                    Vec3::new(39.6, 5.0, offset),
                    Vec3::new(0.8, 10.0, 1.1),
                    CONCRETE,
                );
                b.visual_box(
                    Vec3::new(offset, 5.0, -39.6),
                    Vec3::new(1.1, 10.0, 0.8),
                    CONCRETE,
                );
                b.visual_box(
                    Vec3::new(offset, 5.0, 39.6),
                    Vec3::new(1.1, 10.0, 0.8),
                    CONCRETE,
                );
                offset += 8.0;
            }

            // Emissive cap along the top edge, mostly for bloom calibration.
            b.set_material(0.0, 0.5);
            b.emissive_box(
                Vec3::new(0.0, 9.75, -39.5),
                Vec3::new(78.0, 0.1, 0.12),
                EMIT_CYAN,
                3.0,
            );
        },
    );
}

fn add_plaza(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-19.0, 0.0, -7.0), Vec3::new(19.0, 9.0, 25.0)),
        |b| {
            b.set_material(0.0, 0.72);
            for (x, z) in [(-14.0, -4.0), (14.0, -4.0), (-14.0, 8.0), (14.0, 8.0)] {
                b.cylinder_y(Vec3::new(x, 0.0, z), 0.7, 7.2, 20, CONCRETE, 0.0);
                b.visual_box(
                    Vec3::new(x, 7.2, z),
                    Vec3::new(2.0, 0.4, 2.0),
                    CONCRETE_DARK,
                );
                b.colliders.push(Aabb::new(
                    Vec3::new(x - 0.7, 0.0, z - 0.7),
                    Vec3::new(x + 0.7, 7.2, z + 0.7),
                ));
            }

            // Lintels tie the colonnade together without blocking the view.
            b.set_material(0.0, 0.55);
            for x in [-14.0, 14.0] {
                b.visual_box(
                    Vec3::new(x, 7.3, 2.0),
                    Vec3::new(1.0, 0.6, 18.0),
                    CONCRETE_DARK,
                );
                b.set_material(0.0, 0.35);
                b.emissive_box(
                    Vec3::new(x, 6.95, 2.0),
                    Vec3::new(0.35, 0.08, 17.0),
                    EMIT_WARM,
                    3.5,
                );
                b.set_material(0.0, 0.55);
            }

            // Benches and planters give the eye a human-scale reference.
            b.set_material(0.0, 0.65);
            for (x, z) in [(-9.0, 15.0), (9.0, 15.0)] {
                b.solid_box(
                    Vec3::new(x, 0.50, z),
                    Vec3::new(2.6, 0.16, 0.72),
                    STEEL_DARK,
                );
                b.visual_box(
                    Vec3::new(x - 1.0, 0.25, z),
                    Vec3::new(0.12, 0.5, 0.55),
                    METAL,
                );
                b.visual_box(
                    Vec3::new(x + 1.0, 0.25, z),
                    Vec3::new(0.12, 0.5, 0.55),
                    METAL,
                );
            }
            for (x, z) in [(-7.0, 18.5), (7.0, 18.5)] {
                b.solid_box(
                    Vec3::new(x, 0.40, z),
                    Vec3::new(1.7, 0.8, 1.7),
                    CONCRETE_DARK,
                );
                b.set_material(0.0, 0.9);
                b.visual_box(
                    Vec3::new(x, 1.15, z),
                    Vec3::new(1.25, 0.7, 1.25),
                    PAINT_GREEN,
                );
                b.set_material(0.0, 0.65);
            }

            // Entry pylons frame the spawn and provide a cyan bloom anchor.
            b.set_material(0.0, 0.35);
            for x in [-4.0, 4.0] {
                b.solid_box(
                    Vec3::new(x, 3.0, 22.0),
                    Vec3::new(0.8, 6.0, 0.8),
                    STEEL_DARK,
                );
                b.emissive_box(
                    Vec3::new(x, 3.2, 22.42),
                    Vec3::new(0.42, 3.4, 0.08),
                    EMIT_CYAN,
                    4.0,
                );
            }
        },
    );
}

fn add_material_study(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-15.0, 0.0, -7.0), Vec3::new(15.0, 7.0, 12.0)),
        |b| {
            // A planar metal patch shows environment reflections on a flat
            // surface, where screen-space artefacts are easiest to spot.
            b.set_material(0.92, 0.12);
            b.floor_surface(-6.0, 6.0, -6.0, 6.0, 0.03, [0.23, 0.25, 0.30]);

            b.set_material(0.0, 0.55);
            b.solid_box(
                Vec3::new(0.0, 1.1, 0.0),
                Vec3::new(5.0, 2.2, 5.0),
                CONCRETE_DARK,
            );
            b.set_material(1.0, 0.04);
            b.sphere(Vec3::new(0.0, 4.35, 0.0), 2.0, 32, 20, SILVER);

            // Four material probes on matching plinths: gold, copper, brushed
            // steel and glazed ceramic. The negative metallic override on the
            // ceramic makes it fall back to the scene metallic value.
            for (x, color, metallic, roughness) in [
                (-9.0, GOLD, 1.0, 0.12),
                (-3.0, COPPER, 1.0, 0.28),
                (3.0, SILVER, 1.0, 0.50),
                (9.0, CERAMIC, -1.0, 0.10),
            ] {
                b.set_material(0.0, 0.7);
                b.solid_box(
                    Vec3::new(x, 0.45, 9.0),
                    Vec3::new(2.4, 0.9, 2.4),
                    CONCRETE_DARK,
                );
                b.set_material(metallic, roughness);
                b.sphere(Vec3::new(x, 2.05, 9.0), 1.15, 28, 18, color);
            }
        },
    );
}

fn add_north_deck(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-19.0, 0.0, -37.0), Vec3::new(19.0, 12.0, -7.0)),
        |b| {
            b.set_material(0.0, 0.78);
            b.solid_box(
                Vec3::new(0.0, 3.75, -26.0),
                Vec3::new(36.0, 0.5, 20.0),
                CONCRETE,
            );

            // Support columns under the deck.
            for x in [-14.0, 14.0] {
                for z in [-19.0, -25.0, -31.0] {
                    b.cylinder_y(Vec3::new(x, 0.0, z), 0.42, 3.5, 16, CONCRETE_DARK, 0.0);
                    b.colliders.push(Aabb::new(
                        Vec3::new(x - 0.42, 0.0, z - 0.42),
                        Vec3::new(x + 0.42, 3.5, z + 0.42),
                    ));
                }
            }

            b.set_material(0.0, 0.45);
            b.visual_box(
                Vec3::new(0.0, 3.82, -16.05),
                Vec3::new(36.0, 0.75, 0.32),
                STEEL_DARK,
            );

            // Broad stairs: ten 0.4 m rises stay below the 0.45 m step height
            // of the character controller, so they can be walked up.
            b.set_material(0.0, 0.72);
            b.stairs_z(-7.0, 7.0, -8.0, 10, 0.4, 0.8, true, STAIR);

            b.set_material(0.0, 0.82);
            b.wall_z(-35.8, -18.0, 18.0, 4.0, 10.5, 1.0, WALL_DARK);

            // Reflective back panel: the polished section must mirror the same
            // sun that lights the deck. Vertical seams keep the reflection
            // from reading as one flat, fake sheet.
            b.set_material(1.0, 0.07);
            b.wall_z(-35.55, -8.0, 8.0, 4.2, 10.2, 1.0, SILVER);
            b.set_material(0.35, 0.45);
            for x in [-6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0] {
                b.visual_box(
                    Vec3::new(x, 7.2, -35.48),
                    Vec3::new(0.07, 6.0, 0.08),
                    STEEL_DARK,
                );
            }
            for y in [6.2, 8.2] {
                b.visual_box(
                    Vec3::new(0.0, y, -35.48),
                    Vec3::new(16.0, 0.07, 0.08),
                    STEEL_DARK,
                );
            }
            b.set_material(0.0, 0.35);
            b.emissive_box(
                Vec3::new(0.0, 10.4, -35.42),
                Vec3::new(16.2, 0.12, 0.12),
                EMIT_CYAN,
                4.0,
            );

            // Railings leave the stair opening clear.
            b.set_material(0.2, 0.5);
            b.railing_x(-16.0, -18.0, -7.0, 4.0, METAL);
            b.railing_x(-16.0, 7.0, 18.0, 4.0, METAL);
            b.railing_z(-18.0, -34.6, -16.0, 4.0, METAL);
            b.railing_z(18.0, -34.6, -16.0, 4.0, METAL);

            // Props: crates and two bollards.
            b.set_material(0.2, 0.6);
            b.solid_box(
                Vec3::new(-11.0, 4.55, -21.0),
                Vec3::new(1.4, 1.1, 1.4),
                STEEL_DARK,
            );
            b.solid_box(
                Vec3::new(-9.5, 4.4, -22.3),
                Vec3::new(1.0, 0.8, 1.0),
                PAINT_ORANGE,
            );
            b.set_material(0.0, 0.4);
            for x in [-9.0, 9.0] {
                b.cylinder_y(Vec3::new(x, 4.0, -17.0), 0.13, 1.2, 12, METAL, 0.0);
                b.emissive_box(
                    Vec3::new(x, 5.25, -17.0),
                    Vec3::new(0.3, 0.28, 0.3),
                    EMIT_WARM,
                    5.0,
                );
            }
        },
    );
}

fn add_light_gallery(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(21.0, 0.0, -29.0), Vec3::new(39.0, 7.0, -7.0)),
        |b| {
            b.set_material(0.0, 0.8);
            // Outer shell as solid segments so the building reads correctly
            // from outside as well as from the plaza. Three 2 m doorways open
            // onto the south side.
            for (x1, x2) in [(22.0, 23.5), (25.5, 29.0), (31.0, 34.5), (36.5, 38.0)] {
                b.solid_box(
                    Vec3::new((x1 + x2) * 0.5, 2.7, -8.0),
                    Vec3::new(x2 - x1, 5.4, WALL_T),
                    WALL_DARK,
                );
            }
            for center in [24.5, 30.0, 35.5] {
                b.solid_box(
                    Vec3::new(center, 4.25, -8.0),
                    Vec3::new(2.0, 2.3, WALL_T),
                    WALL_DARK,
                );
            }
            b.solid_box(
                Vec3::new(22.0, 2.7, -18.0),
                Vec3::new(WALL_T, 5.4, 20.0),
                CONCRETE,
            );
            b.solid_box(
                Vec3::new(38.0, 2.7, -18.0),
                Vec3::new(WALL_T, 5.4, 20.0),
                CONCRETE,
            );
            b.solid_box(
                Vec3::new(30.0, 2.7, -28.0),
                Vec3::new(16.0, 5.4, WALL_T),
                CONCRETE,
            );

            // Two interior dividers with 3 m doorways.
            b.set_material(0.0, 0.82);
            for x in [27.3, 32.7] {
                b.solid_box(
                    Vec3::new(x, 2.7, -24.25),
                    Vec3::new(0.3, 5.4, 7.5),
                    CONCRETE_DARK,
                );
                b.solid_box(
                    Vec3::new(x, 2.7, -12.75),
                    Vec3::new(0.3, 5.4, 9.5),
                    CONCRETE_DARK,
                );
                b.solid_box(
                    Vec3::new(x, 4.25, -19.0),
                    Vec3::new(0.3, 2.3, 3.0),
                    CONCRETE_DARK,
                );
            }

            // Roof with a skylight opening above the cool room.
            b.set_material(0.0, 0.85);
            b.solid_box(
                Vec3::new(25.0, 5.6, -18.0),
                Vec3::new(6.0, 0.4, 20.0),
                CONCRETE,
            );
            b.solid_box(
                Vec3::new(35.0, 5.6, -18.0),
                Vec3::new(6.0, 0.4, 20.0),
                CONCRETE,
            );
            b.solid_box(
                Vec3::new(30.0, 5.6, -25.0),
                Vec3::new(4.0, 0.4, 6.0),
                CONCRETE,
            );
            b.solid_box(
                Vec3::new(30.0, 5.6, -11.0),
                Vec3::new(4.0, 0.4, 6.0),
                CONCRETE,
            );

            // Glazing reads as a bright sky panel even without a sky pass.
            b.set_material(0.15, 0.1);
            b.emissive_box(
                Vec3::new(30.0, 5.82, -18.0),
                Vec3::new(3.9, 0.04, 7.9),
                [0.42, 0.58, 0.86],
                1.5,
            );
            b.set_material(0.4, 0.35);
            for (center, size) in [
                (Vec3::new(30.0, 5.86, -22.0), Vec3::new(4.3, 0.12, 0.28)),
                (Vec3::new(30.0, 5.86, -14.0), Vec3::new(4.3, 0.12, 0.28)),
                (Vec3::new(28.0, 5.86, -18.0), Vec3::new(0.28, 0.12, 8.3)),
                (Vec3::new(32.0, 5.86, -18.0), Vec3::new(0.28, 0.12, 8.3)),
                (Vec3::new(30.0, 5.88, -18.0), Vec3::new(0.12, 0.1, 8.0)),
                (Vec3::new(30.0, 5.88, -18.0), Vec3::new(4.0, 0.1, 0.12)),
            ] {
                b.visual_box(center, size, METAL);
            }

            // Coloured ceiling strips and light-wash panels per room.
            for (center_x, color) in [(24.65, EMIT_WARM), (30.0, EMIT_CYAN), (35.35, EMIT_GREEN)] {
                b.set_material(0.0, 0.3);
                for offset in [-1.0, 0.0, 1.0] {
                    b.emissive_box(
                        Vec3::new(center_x + offset, 5.16, -18.0),
                        Vec3::new(0.18, 0.06, 7.0),
                        color,
                        5.0,
                    );
                }
                b.set_material(0.0, 0.6);
                b.emissive_box(
                    Vec3::new(center_x, 2.5, -27.74),
                    Vec3::new(2.8, 2.4, 0.06),
                    color,
                    0.8,
                );
            }

            // Material props: polished gold column, silver sphere and a
            // satin painted cube.
            b.set_material(1.0, 0.18);
            b.cylinder_y(Vec3::new(24.6, 0.0, -23.0), 0.55, 3.2, 20, GOLD, 0.0);
            b.set_material(0.0, 0.7);
            b.solid_box(
                Vec3::new(24.6, 0.12, -23.0),
                Vec3::new(2.0, 0.24, 2.0),
                CONCRETE_DARK,
            );
            b.set_material(1.0, 0.06);
            b.solid_box(
                Vec3::new(30.0, 0.15, -24.0),
                Vec3::new(2.8, 0.3, 2.8),
                STEEL_DARK,
            );
            b.sphere(Vec3::new(30.0, 1.45, -24.0), 1.15, 28, 18, SILVER);
            b.set_material(0.55, 0.34);
            b.solid_box(
                Vec3::new(35.35, 0.8, -23.0),
                Vec3::new(2.0, 1.6, 2.0),
                PAINT_GREEN,
            );
        },
    );
}

fn add_parkour(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-38.0, 0.0, -20.0), Vec3::new(-17.0, 6.0, 18.0)),
        |b| {
            // A rising zig-zag of pads. Gaps are tuned for a running jump with
            // the demo character (about 5.5 m of horizontal range).
            let pads = [
                (-26.0, -20.0, 10.0, 14.0, 0.6, PAINT_ORANGE),
                (-31.0, -25.0, 3.0, 7.0, 1.3, PAINT_RED),
                (-36.0, -30.0, -4.0, 0.0, 2.0, PAINT_BLUE),
                (-31.0, -25.0, -11.0, -7.0, 2.7, PAINT_GREEN),
                (-26.0, -20.0, -18.0, -14.0, 3.4, PAINT_WHITE),
            ];
            for (x1, x2, z1, z2, top, color) in pads {
                b.set_material(0.15, 0.6);
                b.solid_box(
                    Vec3::new((x1 + x2) * 0.5, top * 0.5, (z1 + z2) * 0.5),
                    Vec3::new(x2 - x1, top, z2 - z1),
                    color,
                );
                b.set_material(0.0, 0.45);
                b.visual_box(
                    Vec3::new((x1 + x2) * 0.5, top + 0.012, z1 + 0.35),
                    Vec3::new(x2 - x1 - 1.0, 0.02, 0.18),
                    CONCRETE_DARK,
                );
            }

            // Low vault wall on the first pad.
            b.set_material(0.1, 0.55);
            b.solid_box(
                Vec3::new(-22.5, 1.2, 14.2),
                Vec3::new(3.0, 1.2, 0.35),
                STEEL_DARK,
            );

            // Balance beam between the third and fourth pads.
            b.set_material(0.3, 0.4);
            b.solid_box(
                Vec3::new(-33.0, 1.175, -5.5),
                Vec3::new(0.5, 2.35, 3.0),
                METAL,
            );

            // Bridge from the last pad onto the north deck.
            b.set_material(0.1, 0.6);
            b.solid_box(
                Vec3::new(-19.25, 1.85, -16.75),
                Vec3::new(2.5, 3.7, 2.5),
                PAINT_WHITE,
            );

            // Entrance gantry with an emissive header.
            b.set_material(0.3, 0.45);
            for x in [-25.5, -20.5] {
                b.solid_box(
                    Vec3::new(x, 2.25, 16.5),
                    Vec3::new(0.25, 4.5, 0.25),
                    STEEL_DARK,
                );
            }
            b.visual_box(
                Vec3::new(-23.0, 4.5, 16.5),
                Vec3::new(5.25, 0.35, 0.3),
                STEEL_DARK,
            );
            b.emissive_box(
                Vec3::new(-23.0, 4.22, 16.35),
                Vec3::new(2.2, 0.45, 0.1),
                PAINT_ORANGE,
                3.5,
            );
        },
    );
}

fn add_step_lanes(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(19.0, 0.0, 2.0), Vec3::new(35.0, 5.0, 16.0)),
        |b| {
            let lanes = [
                (20.0, 24.0, 0.22, PAINT_GREEN, EMIT_GREEN),
                (25.0, 29.0, 0.38, PAINT_ORANGE, EMIT_WARM),
                (30.0, 34.0, 0.55, PAINT_RED, EMIT_RED),
            ];
            for (x1, x2, rise, color, glow) in lanes {
                b.set_material(0.1, 0.65);
                b.stairs_z(x1, x2, 14.0, 6, rise, 1.2, true, color);
                let top = rise * 6.0;
                b.solid_box(
                    Vec3::new((x1 + x2) * 0.5, top * 0.5, 4.9),
                    Vec3::new(x2 - x1, top, 3.8),
                    color,
                );
                b.set_material(0.0, 0.35);
                b.emissive_box(
                    Vec3::new((x1 + x2) * 0.5, 0.025, 14.7),
                    Vec3::new(3.0, 0.04, 0.4),
                    glow,
                    2.5,
                );
            }
        },
    );
}

fn add_flight_tower(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(25.0, 0.0, 21.0), Vec3::new(35.0, 37.0, 31.0)),
        |b| {
            b.set_material(0.2, 0.5);
            for x in [26.75, 33.25] {
                for z in [22.75, 29.25] {
                    b.solid_box(Vec3::new(x, 17.0, z), Vec3::new(0.5, 34.0, 0.5), STEEL_DARK);
                }
            }
            b.cylinder_y(Vec3::new(30.0, 0.0, 26.0), 0.22, 34.0, 12, METAL, 0.0);

            let mut level = 4.0;
            while level <= 32.0 {
                b.set_material(0.35, 0.45);
                b.solid_box(
                    Vec3::new(30.0, level - 0.18, 26.0),
                    Vec3::new(7.0, 0.36, 7.0),
                    METAL,
                );
                b.set_material(0.0, 0.3);
                b.emissive_box(
                    Vec3::new(30.0, level + 0.02, 29.45),
                    Vec3::new(6.4, 0.05, 0.15),
                    EMIT_CYAN,
                    2.5,
                );
                level += 4.0;
            }

            b.set_material(0.25, 0.45);
            b.solid_box(
                Vec3::new(30.0, 33.8, 26.0),
                Vec3::new(8.0, 0.4, 8.0),
                STEEL_DARK,
            );
            b.railing_x(22.0, 26.0, 34.0, 34.0, METAL);
            b.railing_x(30.0, 26.0, 28.0, 34.0, METAL);
            b.railing_x(30.0, 32.0, 34.0, 34.0, METAL);
            b.railing_z(26.0, 22.0, 30.0, 34.0, METAL);
            b.railing_z(34.0, 22.0, 30.0, 34.0, METAL);

            b.cylinder_y(Vec3::new(30.0, 34.0, 26.0), 0.1, 1.0, 10, METAL, 0.0);
            b.set_material(0.0, 0.2);
            b.emissive_box(
                Vec3::new(30.0, 35.4, 26.0),
                Vec3::new(1.0, 1.0, 1.0),
                EMIT_RED,
                7.0,
            );
        },
    );
}

fn create_lights() -> Vec<Light> {
    // Exactly eight local lights: the forward shader uploads the nearest eight,
    // so matching the budget prevents visible popping as the camera moves.
    vec![
        Light::new(Vec3::new(0.0, 8.5, 4.0), Vec3::new(1.0, 0.78, 0.55), 24.0).with_intensity(1.4),
        Light::new(
            Vec3::new(24.6, 4.6, -18.0),
            Vec3::new(1.0, 0.55, 0.22),
            12.0,
        )
        .with_intensity(1.8),
        Light::new(
            Vec3::new(30.0, 4.6, -18.0),
            Vec3::new(0.45, 0.75, 1.0),
            12.0,
        )
        .with_intensity(1.8),
        Light::new(
            Vec3::new(35.4, 4.6, -18.0),
            Vec3::new(0.30, 1.0, 0.50),
            12.0,
        )
        .with_intensity(1.8),
        Light::new(Vec3::new(0.0, 8.5, -25.0), Vec3::new(0.60, 0.80, 1.0), 22.0)
            .with_intensity(1.2),
        Light::new(Vec3::new(-28.0, 6.5, 2.0), Vec3::new(1.0, 0.72, 0.42), 17.0)
            .with_intensity(1.2),
        Light::new(
            Vec3::new(30.0, 35.5, 26.0),
            Vec3::new(1.0, 0.18, 0.10),
            16.0,
        )
        .with_intensity(1.6),
        Light::new(Vec3::new(0.0, 5.5, 21.0), Vec3::new(0.30, 0.85, 1.0), 16.0).with_intensity(1.0),
    ]
}

/// Deterministic concrete/panel texture. The mesh UVs tile in world space, so
/// the texture reproduces a four-metre panel grid across every surface.
fn generate_surface_texture() -> Vec<u8> {
    const SIZE: u32 = 256;
    let hash = |x: u32, y: u32| -> f32 {
        let mut n = x
            .wrapping_mul(374_761_393)
            .wrapping_add(y.wrapping_mul(668_265_263))
            .wrapping_add(0x9E37_79B9);
        n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
        ((n & 0xFFFF) as f32) / 65535.0
    };

    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let cell_x = x % 128;
            let cell_y = y % 128;
            let mut value = 0.60 + hash(x, y) * 0.08;
            value += hash(x / 3, y / 3) * 0.05;
            value -= (hash(x / 24, y / 24) - 0.42).max(0.0) * 0.16;

            if cell_x < 2 || cell_y < 2 {
                value = 0.30 + hash(x, y) * 0.05;
            } else {
                if cell_x < 5 || cell_y < 5 {
                    value += 0.06;
                }
                if cell_x > 122 || cell_y > 122 {
                    value -= 0.07;
                }
            }

            // Four countersunk bolts per tile.
            for (rx, ry) in [(12, 12), (116, 12), (12, 116), (116, 116)] {
                let dx = cell_x as i32 - rx;
                let dy = cell_y as i32 - ry;
                if dx * dx + dy * dy < 18 {
                    value += if dx + dy < 0 { 0.14 } else { -0.08 };
                }
            }

            // Sparse stains keep large floors from looking perfectly flat.
            if hash(x / 2 + 71, y / 9 + 13) > 0.988 {
                value -= 0.10;
            }

            let byte = (value.clamp(0.0, 1.0) * 255.0) as u8;
            pixels.extend_from_slice(&[byte, byte, byte, 255]);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::Character;

    fn walk(polygon: &TestPolygon, position: Vec3, wish: Vec3, frames: usize) -> Character {
        let mut character = Character::new(position);
        for _ in 0..frames {
            character.update(
                1.0 / 60.0,
                &polygon.physics,
                wish,
                false,
                false,
                false,
                false,
            );
        }
        character
    }

    #[test]
    fn polygon_respects_light_budget_and_has_material_mix() {
        let polygon = build_polygon();
        assert_eq!(polygon.scene.lights.len(), 8);
        assert!(!polygon.scene.meshes.is_empty());
        assert!(!polygon.physics.colliders().is_empty());
        let vertices = polygon
            .scene
            .meshes
            .iter()
            .flat_map(|mesh| mesh.vertices.iter());
        let mut polished = false;
        let mut fallback = false;
        for vertex in vertices {
            polished |= vertex.material[0] > 0.9;
            fallback |= vertex.material == [-1.0, -1.0];
        }
        assert!(polished, "the polygon should contain polished metal");
        assert!(fallback, "some surfaces should use the scene material");
    }

    #[test]
    fn spawn_stands_on_the_plaza() {
        let polygon = build_polygon();
        let character = walk(&polygon, SPAWN, Vec3::ZERO, 120);
        assert!(character.on_ground);
        assert!(character.position.y.abs() < 0.02);
    }

    #[test]
    fn broad_stairs_are_walkable() {
        let polygon = build_polygon();
        let character = walk(&polygon, Vec3::new(0.0, 0.002, -4.0), -Vec3::Z, 300);
        assert!(
            character.position.y > 3.9,
            "should climb onto the north deck, ended at {:?}",
            character.position
        );
        assert!(character.position.z < -16.0);
    }

    #[test]
    fn flight_tower_reaches_the_beacon() {
        let polygon = build_polygon();
        let mut character = Character::new(Vec3::new(30.0, 0.5, 26.0));
        character.set_flying(true);
        for _ in 0..180 {
            character.update(
                1.0 / 60.0,
                &polygon.physics,
                Vec3::ZERO,
                false,
                true,
                false,
                false,
            );
        }
        assert!(
            character.position.y > 34.0,
            "held Space should fly above the tower, got {}",
            character.position.y
        );
    }
}
