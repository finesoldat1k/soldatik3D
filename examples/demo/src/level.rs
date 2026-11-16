//! The facility: procedural geometry, static lighting and gameplay markers.
//!
//! Layout (top-down, north is −Z):
//!
//! ```text
//!                        [reactor chamber]
//!                        /               \
//!                 [NW loop]               [NE loop]
//!                      |                       |
//!                [pump room]---[atrium]---[relay room]
//! ```
//!
//! Every room is a rectangular shell with explicit door openings. Props are
//! prebuilt here because the renderer uploads scene geometry once. The same
//! builder emits render meshes, collision boxes, light fixtures and the
//! interaction points used by the game.

use std::f32::consts::{PI, TAU};

use runtime::glam::Vec3;
use runtime::{
    Aabb, Light, Material, Mesh, PrimaryLight, Scene, ShadowSettings, TextureData, Vertex,
};

use crate::physics::PhysicsWorld;

const WALL_T: f32 = 0.36;
const DOOR_TOP: f32 = 3.15;
/// One albedo tile covers four metres of world space.
const UV_SCALE: f32 = 0.25;

const FLOOR: [f32; 3] = [0.38, 0.41, 0.45];
const FLOOR_DARK: [f32; 3] = [0.20, 0.22, 0.26];
const WALL: [f32; 3] = [0.57, 0.62, 0.68];
const WALL_DARK: [f32; 3] = [0.27, 0.30, 0.35];
const CEILING: [f32; 3] = [0.28, 0.31, 0.36];
const TRIM: [f32; 3] = [0.76, 0.80, 0.85];
const METAL: [f32; 3] = [0.47, 0.52, 0.59];
const DARK_METAL: [f32; 3] = [0.13, 0.15, 0.19];
const PIPE_BLUE: [f32; 3] = [0.18, 0.45, 0.62];
const PIPE_RUST: [f32; 3] = [0.62, 0.32, 0.15];
const HAZARD_YELLOW: [f32; 3] = [0.92, 0.72, 0.12];
const HAZARD_DARK: [f32; 3] = [0.07, 0.07, 0.09];
const EMIT_CYAN: [f32; 3] = [0.1, 0.78, 1.0];
const EMIT_WARM: [f32; 3] = [1.0, 0.56, 0.2];
const EMIT_RED: [f32; 3] = [1.0, 0.09, 0.035];
const EMIT_GREEN: [f32; 3] = [0.16, 1.0, 0.38];

/// Player spawn in the south of the atrium, looking north.
pub const SPAWN: Vec3 = Vec3::new(0.0, 0.002, 6.0);
pub const SPAWN_YAW: f32 = -PI * 0.5;
/// Falling below this height respawns the player.
pub const FALL_LIMIT: f32 = -7.5;

/// A machine the player can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    CoolantPump(usize),
    Reactor,
}

/// Static interaction anchor in the world.
#[derive(Clone, Debug)]
pub struct Interactable {
    pub device: Device,
    /// Point the player should aim at, roughly at chest height.
    pub position: Vec3,
    /// Maximum eye-to-anchor distance that counts as "in reach".
    pub radius: f32,
    pub prompt: &'static str,
}

/// One respawn location and the volume that unlocks it.
#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub trigger: Aabb,
    pub position: Vec3,
    pub yaw: f32,
    pub name: &'static str,
}

/// Everything the game loop needs from the level.
pub struct Facility {
    pub scene: Scene,
    pub physics: PhysicsWorld,
    pub checkpoints: Vec<Checkpoint>,
    pub interactables: Vec<Interactable>,
}

/// Builds the scene, colliders and gameplay markers.
pub fn build_facility() -> Facility {
    let mut assembly = Assembly::new();
    add_atrium(&mut assembly);
    add_west_corridor(&mut assembly);
    add_east_corridor(&mut assembly);
    add_north_corridor(&mut assembly);
    add_pump_room(&mut assembly);
    add_relay_room(&mut assembly);
    add_reactor_chamber(&mut assembly);
    add_loop_corridors(&mut assembly);
    add_checkpoints(&mut assembly);
    add_interactables(&mut assembly);

    let material = Material {
        albedo: TextureData::new(256, 256, generate_panel_texture()),
        metallic: 0.15,
        roughness: 0.82,
    };
    let scene = Scene::new(material)
        .with_meshes(assembly.meshes)
        .with_lights(create_lights())
        .with_primary_light(PrimaryLight {
            position: Vec3::new(4.0, 48.0, -18.0),
            target: Vec3::new(0.0, 0.0, -18.0),
            up: Vec3::Z,
            color: Vec3::new(0.42, 0.5, 0.68),
            intensity: 0.45,
            shadow: ShadowSettings {
                ortho_half_extent: 58.0,
                near: 0.1,
                far: 120.0,
            },
        })
        .with_clear_color([0.006, 0.008, 0.013, 1.0]);

    Facility {
        scene,
        physics: PhysicsWorld::new(assembly.colliders),
        checkpoints: assembly.checkpoints,
        interactables: assembly.interactables,
    }
}

/// Accumulates meshes while keeping collision and gameplay data in sync.
struct Assembly {
    meshes: Vec<Mesh>,
    colliders: Vec<Aabb>,
    checkpoints: Vec<Checkpoint>,
    interactables: Vec<Interactable>,
}

impl Assembly {
    fn new() -> Self {
        Self {
            meshes: Vec::new(),
            colliders: Vec::new(),
            checkpoints: Vec::new(),
            interactables: Vec::new(),
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

    fn checkpoint(&mut self, name: &'static str, trigger: Aabb, position: Vec3, yaw: f32) {
        self.checkpoints.push(Checkpoint {
            trigger,
            position,
            yaw,
            name,
        });
    }

    fn interactable(&mut self, device: Device, position: Vec3, radius: f32, prompt: &'static str) {
        self.interactables.push(Interactable {
            device,
            position,
            radius,
            prompt,
        });
    }
}

/// Small mesh builder with world-space tiled UVs.
struct ChunkBuilder {
    vertices: Vec<Vertex>,
    colliders: Vec<Aabb>,
}

impl ChunkBuilder {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            colliders: Vec::new(),
        }
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
            Vertex::new(position.to_array(), color, normal.to_array(), uv).with_emission(emission),
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
        normal: Vec3,
        uvs: [[f32; 2]; 3],
        color: [f32; 3],
        emission: f32,
    ) {
        for index in 0..3 {
            self.vertex(positions[index], normal, uvs[index], color, emission);
        }
    }

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

    fn ceiling(&mut self, x1: f32, x2: f32, z1: f32, z2: f32, y: f32, color: [f32; 3]) {
        let s = UV_SCALE;
        self.quad(
            [
                Vec3::new(x1, y, z1),
                Vec3::new(x2, y, z1),
                Vec3::new(x2, y, z2),
                Vec3::new(x1, y, z2),
            ],
            -Vec3::Y,
            [
                [x1 * s, z1 * s],
                [x2 * s, z1 * s],
                [x2 * s, z2 * s],
                [x1 * s, z2 * s],
            ],
            color,
            0.0,
        );
        self.colliders
            .push(Aabb::new(Vec3::new(x1, y, z1), Vec3::new(x2, y + 0.8, z2)));
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

    #[allow(clippy::too_many_arguments)]
    fn wall_x_with_door(
        &mut self,
        x: f32,
        z1: f32,
        z2: f32,
        y1: f32,
        y2: f32,
        door1: f32,
        door2: f32,
        facing: f32,
        color: [f32; 3],
    ) {
        let (door1, door2) = (door1.min(door2), door1.max(door2));
        if door1 > z1 {
            self.wall_x(x, z1, door1, y1, y2, facing, color);
        }
        if door2 < z2 {
            self.wall_x(x, door2, z2, y1, y2, facing, color);
        }
        self.wall_x(x, door1, door2, DOOR_TOP.max(y1), y2, facing, color);
    }

    #[allow(clippy::too_many_arguments)]
    fn wall_z_with_door(
        &mut self,
        z: f32,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        door1: f32,
        door2: f32,
        facing: f32,
        color: [f32; 3],
    ) {
        let (door1, door2) = (door1.min(door2), door1.max(door2));
        if door1 > x1 {
            self.wall_z(z, x1, door1, y1, y2, facing, color);
        }
        if door2 < x2 {
            self.wall_z(z, door2, x2, y1, y2, facing, color);
        }
        self.wall_z(z, door1, door2, DOOR_TOP.max(y1), y2, facing, color);
    }

    /// Cylinder between two points, including caps. No collision.
    fn pipe(&mut self, from: Vec3, to: Vec3, radius: f32, sides: usize, color: [f32; 3]) {
        self.cylinder_between(from, to, radius, sides, color, 0.0);
    }

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
                self.triangle(points, normal, uvs, color, emission);
            }
        }
    }

    /// Cylinder centred on `base`, extending up by `height`.
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

    /// Railing running along X at a fixed Z. Adds a thin blocking collider.
    fn railing_x(&mut self, z: f32, x1: f32, x2: f32, base_y: f32, color: [f32; 3]) {
        self.railing_x_inner(z, x1, x2, base_y, color, true);
    }

    fn railing_x_inner(
        &mut self,
        z: f32,
        x1: f32,
        x2: f32,
        base_y: f32,
        color: [f32; 3],
        collide: bool,
    ) {
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
        if collide {
            self.colliders.push(Aabb::new(
                Vec3::new(x1, base_y, z - 0.1),
                Vec3::new(x2, base_y + 1.15, z + 0.1),
            ));
        }
    }

    /// Railing running along Z at a fixed X.
    fn railing_z(&mut self, x: f32, z1: f32, z2: f32, base_y: f32, color: [f32; 3]) {
        self.railing_z_inner(x, z1, z2, base_y, color, true);
    }

    fn railing_z_inner(
        &mut self,
        x: f32,
        z1: f32,
        z2: f32,
        base_y: f32,
        color: [f32; 3],
        collide: bool,
    ) {
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
        if collide {
            self.colliders.push(Aabb::new(
                Vec3::new(x - 0.1, base_y, z1),
                Vec3::new(x + 0.1, base_y + 1.15, z2),
            ));
        }
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

    /// Straight stair flight ascending towards +X when `east` is true.
    #[allow(clippy::too_many_arguments)]
    fn stairs_x(
        &mut self,
        z1: f32,
        z2: f32,
        x_start: f32,
        steps: usize,
        rise: f32,
        run: f32,
        east: bool,
        color: [f32; 3],
    ) {
        let direction = if east { 1.0 } else { -1.0 };
        for index in 0..steps {
            let top = rise * (index + 1) as f32;
            let x0 = x_start + direction * run * index as f32;
            let x1 = x0 + direction * run;
            let (xmin, xmax) = (x0.min(x1), x0.max(x1));
            self.solid_box(
                Vec3::new((xmin + xmax) * 0.5, top * 0.5, (z1 + z2) * 0.5),
                Vec3::new(run, top, z2 - z1),
                color,
            );
        }
    }

    fn light_panel(&mut self, center: Vec3, size: Vec3, color: [f32; 3]) {
        // Recessed fixture: a dark housing frames the glowing diffuser so
        // ceiling lights read as hardware instead of floating white bars.
        self.visual_box(
            center + Vec3::new(0.0, size.y * 0.5 + 0.055, 0.0),
            Vec3::new(size.x + 0.24, 0.11, size.z + 0.24),
            DARK_METAL,
        );
        self.emissive_box(center, size, color, 2.8);
    }

    /// Yellow/black hazard band around a rectangular opening.
    fn hazard_band_x(&mut self, z: f32, x1: f32, x2: f32, y: f32) {
        let pieces = ((x2 - x1) / 0.6).ceil() as usize;
        for index in 0..pieces {
            let a = x1 + (x2 - x1) * index as f32 / pieces as f32;
            let b = x1 + (x2 - x1) * (index + 1) as f32 / pieces as f32;
            let color = if index % 2 == 0 {
                HAZARD_YELLOW
            } else {
                HAZARD_DARK
            };
            self.visual_box(
                Vec3::new((a + b) * 0.5, y, z),
                Vec3::new(b - a, 0.16, 0.06),
                color,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rooms and corridors
// ---------------------------------------------------------------------------

fn add_atrium(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-10.0, -8.0, -10.0), Vec3::new(10.0, 7.0, 10.0)),
        |b| {
            // Floor with a maintenance pit in the middle.
            b.floor(-10.0, -4.4, -10.0, 10.0, 0.0, FLOOR);
            b.floor(4.4, 10.0, -10.0, 10.0, 0.0, FLOOR);
            b.floor(-4.4, 4.4, -10.0, -4.4, 0.0, FLOOR);
            b.floor(-4.4, 4.4, 4.4, 10.0, 0.0, FLOOR);
            b.ceiling(-10.0, 10.0, -10.0, 10.0, 7.0, CEILING);

            b.wall_x_with_door(-10.0, -10.0, 10.0, 0.0, 7.0, -3.0, 3.0, 1.0, WALL);
            b.wall_x_with_door(10.0, -10.0, 10.0, 0.0, 7.0, -3.0, 3.0, -1.0, WALL);
            b.wall_z_with_door(-10.0, -10.0, 10.0, 0.0, 7.0, -3.0, 3.0, 1.0, WALL);
            b.wall_z(10.0, -10.0, 10.0, 0.0, 7.0, -1.0, WALL);

            // Pit lining. Not a floor: falling below FALL_LIMIT respawns.
            b.wall_x(-4.4, -4.4, 4.4, -8.0, 0.0, 1.0, WALL_DARK);
            b.wall_x(4.4, -4.4, 4.4, -8.0, 0.0, -1.0, WALL_DARK);
            b.wall_z(-4.4, -4.4, 4.4, -8.0, 0.0, 1.0, WALL_DARK);
            b.wall_z(4.4, -4.4, 4.4, -8.0, 0.0, -1.0, WALL_DARK);
            b.floor_surface(-4.4, 4.4, -4.4, 4.4, -8.0, [0.035, 0.04, 0.055]);
            b.emissive_box(
                Vec3::new(0.0, -7.85, -4.3),
                Vec3::new(8.4, 0.08, 0.08),
                EMIT_RED,
                5.0,
            );
            b.emissive_box(
                Vec3::new(0.0, -7.85, 4.3),
                Vec3::new(8.4, 0.08, 0.08),
                EMIT_RED,
                5.0,
            );

            // Central service bridge across the pit. It gives the spawn a
            // readable route north and turns the pit into a proper hazard
            // rather than an obstacle with no reason to exist.
            b.solid_box(
                Vec3::new(0.0, 0.09, 0.0),
                Vec3::new(2.2, 0.18, 8.8),
                DARK_METAL,
            );
            for (x, side) in [(-1.02, -1.0), (1.02, 1.0)] {
                b.railing_z(x, -4.25, 4.25, 0.18, HAZARD_YELLOW);
                b.emissive_box(
                    Vec3::new(side * 1.13, 0.13, 0.0),
                    Vec3::new(0.05, 0.035, 8.3),
                    EMIT_CYAN,
                    1.6,
                );
            }
            for z in [-3.0, 0.0, 3.0] {
                b.visual_box(
                    Vec3::new(0.0, -0.38, z),
                    Vec3::new(1.9, 0.7, 0.22),
                    DARK_METAL,
                );
            }

            // Guard rails on three sides; the south side is the hazard lip.
            b.railing_x(-4.35, -4.4, -1.08, 0.0, TRIM);
            b.railing_x(-4.35, 1.08, 4.4, 0.0, TRIM);
            b.railing_z(-4.35, -4.4, 4.4, 0.0, TRIM);
            b.railing_z(4.35, -4.4, 4.4, 0.0, TRIM);
            b.hazard_band_x(4.35, -4.4, 4.4, 0.08);

            // Baseboards and door frames.
            for (x1, x2, z) in [(-10.0, -3.0, -9.78), (3.0, 10.0, -9.78)] {
                b.visual_box(
                    Vec3::new((x1 + x2) * 0.5, 0.2, z),
                    Vec3::new(x2 - x1, 0.4, 0.07),
                    DARK_METAL,
                );
            }
            b.visual_box(
                Vec3::new(0.0, 0.2, 9.78),
                Vec3::new(19.5, 0.4, 0.07),
                DARK_METAL,
            );
            for (z1, z2, x) in [(-10.0, -3.0, -9.78), (3.0, 10.0, -9.78)] {
                b.visual_box(
                    Vec3::new(x, 0.2, (z1 + z2) * 0.5),
                    Vec3::new(0.07, 0.4, z2 - z1),
                    DARK_METAL,
                );
            }
            for (z1, z2, x) in [(-10.0, -3.0, 9.78), (3.0, 10.0, 9.78)] {
                b.visual_box(
                    Vec3::new(x, 0.2, (z1 + z2) * 0.5),
                    Vec3::new(0.07, 0.4, z2 - z1),
                    DARK_METAL,
                );
            }
            door_frame_x(b, -9.8, -3.15, 3.15, 3.2);
            door_frame_x(b, 9.8, -3.15, 3.15, 3.2);
            door_frame_z(b, -9.8, -3.15, 3.15, 3.2);

            // Corner columns with vertical light strips.
            for (cx, cz) in [(-7.9, -7.9), (7.9, -7.9), (-7.9, 7.9), (7.9, 7.9)] {
                b.solid_box(Vec3::new(cx, 3.5, cz), Vec3::new(0.7, 7.0, 0.7), DARK_METAL);
                let sx = -cx.signum();
                let sz = -cz.signum();
                b.emissive_box(
                    Vec3::new(cx + sx * 0.37, 3.4, cz),
                    Vec3::new(0.05, 3.4, 0.16),
                    EMIT_CYAN,
                    3.5,
                );
                b.emissive_box(
                    Vec3::new(cx, 3.4, cz + sz * 0.37),
                    Vec3::new(0.16, 3.4, 0.05),
                    EMIT_CYAN,
                    3.5,
                );
            }

            // Ceiling beams and long warm fixtures.
            for z in [-5.0, 0.0, 5.0] {
                b.visual_box(
                    Vec3::new(0.0, 6.72, z),
                    Vec3::new(19.4, 0.26, 0.18),
                    DARK_METAL,
                );
            }
            for z in [-4.5, 4.5] {
                b.light_panel(
                    Vec3::new(0.0, 6.84, z),
                    Vec3::new(3.8, 0.05, 0.26),
                    EMIT_WARM,
                );
            }

            // Guide strips from the spawn point towards the reactor door.
            for x in [-0.85, 0.85] {
                b.emissive_box(
                    Vec3::new(x, 0.02, 6.9),
                    Vec3::new(0.09, 0.025, 5.4),
                    EMIT_CYAN,
                    2.2,
                );
            }
            b.emissive_box(
                Vec3::new(0.0, 3.85, -9.76),
                Vec3::new(2.6, 0.14, 0.06),
                EMIT_GREEN,
                4.0,
            );

            // Wall vents add scale to the large empty shell.
            for z in [-7.0, 7.0] {
                b.visual_box(
                    Vec3::new(-9.75, 2.0, z),
                    Vec3::new(0.06, 1.1, 1.8),
                    DARK_METAL,
                );
                for index in 0..4 {
                    b.visual_box(
                        Vec3::new(-9.7, 1.65 + index as f32 * 0.23, z),
                        Vec3::new(0.04, 0.06, 1.55),
                        METAL,
                    );
                }
            }
        },
    );
}

fn add_west_corridor(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-16.0, 0.0, -3.0), Vec3::new(-10.0, 4.0, 3.0)),
        |b| {
            b.floor(-16.0, -10.0, -3.0, 3.0, 0.0, FLOOR_DARK);
            b.ceiling(-16.0, -10.0, -3.0, 3.0, 4.0, CEILING);
            b.wall_z(-3.0, -16.0, -10.0, 0.0, 4.0, 1.0, WALL);
            b.wall_z(3.0, -16.0, -10.0, 0.0, 4.0, -1.0, WALL);
            door_frame_x(b, -10.1, -3.15, 3.15, 3.2);
            door_frame_x(b, -15.9, -3.15, 3.15, 3.2);

            for x in [-14.2, -11.8] {
                b.emissive_box(
                    Vec3::new(x, 3.92, 0.0),
                    Vec3::new(1.1, 0.04, 0.24),
                    EMIT_CYAN,
                    4.0,
                );
            }
            b.pipe(
                Vec3::new(-16.0, 3.5, -2.55),
                Vec3::new(-10.0, 3.5, -2.55),
                0.1,
                8,
                PIPE_BLUE,
            );
            b.pipe(
                Vec3::new(-16.0, 3.25, 2.58),
                Vec3::new(-10.0, 3.25, 2.58),
                0.075,
                8,
                PIPE_RUST,
            );
            b.hazard_band_x(0.0, -10.6, -9.4, 0.04);
            for z in [-2.94, 2.94] {
                b.visual_box(
                    Vec3::new(-13.0, 0.24, z),
                    Vec3::new(6.0, 0.36, 0.06),
                    DARK_METAL,
                );
            }
        },
    );
}

fn add_east_corridor(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(10.0, 0.0, -3.0), Vec3::new(16.0, 4.0, 3.0)),
        |b| {
            b.floor(10.0, 16.0, -3.0, 3.0, 0.0, FLOOR_DARK);
            b.ceiling(10.0, 16.0, -3.0, 3.0, 4.0, CEILING);
            b.wall_z(-3.0, 10.0, 16.0, 0.0, 4.0, 1.0, WALL);
            b.wall_z(3.0, 10.0, 16.0, 0.0, 4.0, -1.0, WALL);
            door_frame_x(b, 10.1, -3.15, 3.15, 3.2);
            door_frame_x(b, 15.9, -3.15, 3.15, 3.2);

            for x in [11.8, 14.2] {
                b.emissive_box(
                    Vec3::new(x, 3.92, 0.0),
                    Vec3::new(1.1, 0.04, 0.24),
                    EMIT_CYAN,
                    4.0,
                );
            }
            b.pipe(
                Vec3::new(10.0, 3.5, -2.55),
                Vec3::new(16.0, 3.5, -2.55),
                0.1,
                8,
                PIPE_BLUE,
            );
            b.pipe(
                Vec3::new(10.0, 3.25, 2.58),
                Vec3::new(16.0, 3.25, 2.58),
                0.075,
                8,
                PIPE_RUST,
            );
            for z in [-2.94, 2.94] {
                b.visual_box(
                    Vec3::new(13.0, 0.24, z),
                    Vec3::new(6.0, 0.36, 0.06),
                    DARK_METAL,
                );
            }
        },
    );
}

fn add_north_corridor(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-3.0, 0.0, -22.0), Vec3::new(3.0, 4.0, -10.0)),
        |b| {
            b.floor(-3.0, 3.0, -22.0, -10.0, 0.0, FLOOR_DARK);
            b.ceiling(-3.0, 3.0, -22.0, -10.0, 4.0, CEILING);
            b.wall_x(-3.0, -22.0, -10.0, 0.0, 4.0, 1.0, WALL);
            b.wall_x(3.0, -22.0, -10.0, 0.0, 4.0, -1.0, WALL);
            door_frame_z(b, -10.1, -3.15, 3.15, 3.2);
            door_frame_z(b, -21.9, -3.15, 3.15, 3.2);

            for z in [-19.8, -16.4, -13.0] {
                b.emissive_box(
                    Vec3::new(0.0, 3.92, z),
                    Vec3::new(0.24, 0.04, 1.1),
                    EMIT_WARM,
                    4.0,
                );
            }
            b.pipe(
                Vec3::new(-2.55, 3.55, -22.0),
                Vec3::new(-2.55, 3.55, -10.0),
                0.11,
                8,
                PIPE_BLUE,
            );
            b.pipe(
                Vec3::new(2.55, 3.3, -22.0),
                Vec3::new(2.55, 3.3, -10.0),
                0.08,
                8,
                PIPE_RUST,
            );
            b.hazard_band_x(-10.6, -3.0, 3.0, 0.04);
            b.hazard_band_x(-21.4, -3.0, 3.0, 0.04);
        },
    );
}

fn door_frame_x(b: &mut ChunkBuilder, x: f32, z1: f32, z2: f32, top: f32) {
    let color = TRIM;
    for z in [z1, z2] {
        b.visual_box(
            Vec3::new(x, top * 0.5, z),
            Vec3::new(0.09, top, 0.18),
            color,
        );
    }
    b.visual_box(
        Vec3::new(x, top + 0.1, (z1 + z2) * 0.5),
        Vec3::new(0.09, 0.2, (z2 - z1).abs() + 0.36),
        color,
    );
}

fn door_frame_z(b: &mut ChunkBuilder, z: f32, x1: f32, x2: f32, top: f32) {
    let color = TRIM;
    for x in [x1, x2] {
        b.visual_box(
            Vec3::new(x, top * 0.5, z),
            Vec3::new(0.18, top, 0.09),
            color,
        );
    }
    b.visual_box(
        Vec3::new((x1 + x2) * 0.5, top + 0.1, z),
        Vec3::new((x2 - x1).abs() + 0.36, 0.2, 0.09),
        color,
    );
}

fn add_pump_room(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-36.0, 0.0, -12.0), Vec3::new(-16.0, 6.0, 12.0)),
        |b| {
            b.floor(-36.0, -16.0, -12.0, 12.0, 0.0, FLOOR);
            b.ceiling(-36.0, -16.0, -12.0, 12.0, 6.0, CEILING);
            b.wall_x(-36.0, -12.0, 12.0, 0.0, 6.0, 1.0, WALL);
            b.wall_z(12.0, -36.0, -16.0, 0.0, 6.0, -1.0, WALL);
            b.wall_z_with_door(-12.0, -36.0, -16.0, 0.0, 6.0, -32.0, -26.0, 1.0, WALL);
            b.wall_x_with_door(-16.0, -12.0, 12.0, 0.0, 6.0, -3.0, 3.0, -1.0, WALL);
            door_frame_x(b, -16.2, -3.15, 3.15, 3.2);

            // Central drainage channel running towards the door.
            b.visual_box(
                Vec3::new(-26.0, 0.012, 0.0),
                Vec3::new(18.0, 0.02, 1.0),
                FLOOR_DARK,
            );
            for index in 0..9 {
                b.visual_box(
                    Vec3::new(-34.5 + index as f32 * 2.0, 0.03, 0.0),
                    Vec3::new(0.08, 0.04, 0.95),
                    METAL,
                );
            }

            pump_unit(b, Vec3::new(-34.0, 0.0, -6.0), Vec3::X);
            pump_unit(b, Vec3::new(-27.0, 0.0, -10.0), Vec3::Z);
            pump_unit(b, Vec3::new(-34.0, 0.0, 6.5), Vec3::X);

            // Coolant mains along the west and north walls, with drops to the
            // pump tops and chunky flanges at each joint.
            b.pipe(
                Vec3::new(-35.2, 4.4, -11.0),
                Vec3::new(-35.2, 4.4, 11.0),
                0.27,
                10,
                PIPE_BLUE,
            );
            b.pipe(
                Vec3::new(-35.2, 4.4, -11.0),
                Vec3::new(-17.0, 4.4, -11.0),
                0.27,
                10,
                PIPE_BLUE,
            );
            for (x, z) in [(-35.2, -6.0), (-35.2, 6.5), (-27.0, -11.0)] {
                b.pipe(
                    Vec3::new(x, 2.0, z),
                    Vec3::new(x, 4.4, z),
                    0.18,
                    8,
                    PIPE_BLUE,
                );
                b.cylinder_y(Vec3::new(x, 4.28, z), 0.34, 0.24, 10, DARK_METAL, 0.0);
            }
            b.pipe(
                Vec3::new(-35.0, 3.85, 11.2),
                Vec3::new(-17.0, 3.85, 11.2),
                0.14,
                8,
                PIPE_RUST,
            );
            b.pipe(
                Vec3::new(-17.0, 3.85, 11.2),
                Vec3::new(-17.0, 3.85, 4.0),
                0.14,
                8,
                PIPE_RUST,
            );

            // Wall-mounted control console by the entrance.
            b.solid_box(
                Vec3::new(-18.6, 0.55, 4.6),
                Vec3::new(1.0, 1.1, 2.2),
                DARK_METAL,
            );
            b.emissive_box(
                Vec3::new(-19.12, 1.35, 4.6),
                Vec3::new(0.05, 0.7, 1.7),
                EMIT_CYAN,
                3.5,
            );
            b.visual_box(
                Vec3::new(-18.6, 1.22, 4.6),
                Vec3::new(1.05, 0.12, 2.3),
                METAL,
            );

            // Crate stacks and spares.
            crate_stack(b, Vec3::new(-19.3, 0.0, -9.6), 2, PIPE_RUST);
            crate_stack(b, Vec3::new(-23.0, 0.0, 9.8), 2, METAL);
            crate_stack(b, Vec3::new(-20.5, 0.0, -6.4), 1, DARK_METAL);

            // Baseboards and overhead beams.
            b.visual_box(
                Vec3::new(-26.0, 0.2, 11.76),
                Vec3::new(19.4, 0.4, 0.07),
                DARK_METAL,
            );
            b.visual_box(
                Vec3::new(-35.76, 0.2, 0.0),
                Vec3::new(0.07, 0.4, 23.4),
                DARK_METAL,
            );
            for z in [-6.0, 0.0, 6.0] {
                b.visual_box(
                    Vec3::new(-26.0, 5.72, z),
                    Vec3::new(19.4, 0.26, 0.2),
                    DARK_METAL,
                );
            }

            // Cyan warning strips along the west wall.
            b.emissive_box(
                Vec3::new(-35.78, 2.6, 0.0),
                Vec3::new(0.05, 0.07, 22.8),
                EMIT_CYAN,
                4.0,
            );
            for z in [-8.0, -2.0, 4.0, 9.0] {
                b.light_panel(
                    Vec3::new(-29.0, 5.94, z),
                    Vec3::new(2.6, 0.05, 0.32),
                    if (z as i32) % 2 == 0 {
                        EMIT_CYAN
                    } else {
                        EMIT_WARM
                    },
                );
            }
        },
    );
}

fn add_relay_room(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(16.0, 0.0, -12.0), Vec3::new(36.0, 6.0, 12.0)),
        |b| {
            b.floor(16.0, 36.0, -12.0, 12.0, 0.0, FLOOR);
            b.ceiling(16.0, 36.0, -12.0, 12.0, 6.0, CEILING);
            b.wall_x(36.0, -12.0, 12.0, 0.0, 6.0, -1.0, WALL);
            b.wall_z(12.0, 16.0, 36.0, 0.0, 6.0, -1.0, WALL);
            b.wall_z_with_door(-12.0, 16.0, 36.0, 0.0, 6.0, 26.0, 32.0, 1.0, WALL);
            b.wall_x_with_door(16.0, -12.0, 12.0, 0.0, 6.0, -3.0, 3.0, 1.0, WALL);
            door_frame_x(b, 16.2, -3.15, 3.15, 3.2);

            // Raised floor tiles with a glowing central aisle.
            b.visual_box(
                Vec3::new(26.0, 0.012, 0.0),
                Vec3::new(18.0, 0.02, 2.2),
                FLOOR_DARK,
            );
            for x in [-7.0, -3.5, 0.0, 3.5, 7.0] {
                b.emissive_box(
                    Vec3::new(26.0 + x, 0.03, 0.0),
                    Vec3::new(0.08, 0.025, 2.0),
                    EMIT_GREEN,
                    2.0,
                );
            }

            for x in [18.2, 20.4, 22.6, 24.8] {
                server_rack(b, Vec3::new(x, 0.0, -10.6), Vec3::Z);
            }
            for z in [-8.0, -5.6, 7.0, 9.4] {
                server_rack(b, Vec3::new(34.4, 0.0, z), -Vec3::X);
            }

            // Holographic operations table.
            holo_table(b, Vec3::new(26.0, 0.0, 1.5));

            // Operator desk facing the table.
            b.solid_box(
                Vec3::new(21.0, 0.55, 6.8),
                Vec3::new(2.6, 1.1, 1.1),
                DARK_METAL,
            );
            for x in [20.2, 21.8] {
                b.emissive_box(
                    Vec3::new(x, 1.45, 6.8),
                    Vec3::new(0.9, 0.55, 0.06),
                    EMIT_CYAN,
                    3.0,
                );
                b.visual_box(Vec3::new(x, 1.15, 6.8), Vec3::new(0.95, 0.6, 0.1), METAL);
            }

            // Cable trays across the ceiling.
            for z in [-6.0, 6.0] {
                b.visual_box(
                    Vec3::new(26.0, 5.25, z),
                    Vec3::new(18.6, 0.16, 0.7),
                    DARK_METAL,
                );
                for x in [17.5, 22.0, 26.5, 31.0, 35.0] {
                    b.visual_box(Vec3::new(x, 5.12, z), Vec3::new(0.08, 0.12, 0.8), METAL);
                }
            }
            b.pipe(
                Vec3::new(17.0, 4.7, -9.5),
                Vec3::new(35.0, 4.7, -9.5),
                0.1,
                8,
                PIPE_BLUE,
            );
            b.pipe(
                Vec3::new(35.0, 4.7, -9.5),
                Vec3::new(35.0, 4.7, 10.5),
                0.1,
                8,
                PIPE_BLUE,
            );

            crate_stack(b, Vec3::new(31.5, 0.0, 9.2), 2, METAL);
            crate_stack(b, Vec3::new(29.0, 0.0, 10.0), 1, PIPE_RUST);

            b.visual_box(
                Vec3::new(26.0, 0.2, 11.76),
                Vec3::new(19.4, 0.4, 0.07),
                DARK_METAL,
            );
            b.visual_box(
                Vec3::new(35.76, 0.2, 0.0),
                Vec3::new(0.07, 0.4, 23.4),
                DARK_METAL,
            );
            for z in [-6.0, 0.0, 6.0] {
                b.visual_box(
                    Vec3::new(26.0, 5.72, z),
                    Vec3::new(19.4, 0.26, 0.2),
                    DARK_METAL,
                );
            }
            b.emissive_box(
                Vec3::new(35.78, 2.6, 0.0),
                Vec3::new(0.05, 0.07, 22.8),
                EMIT_GREEN,
                4.0,
            );
            for z in [-8.0, -2.0, 4.0, 9.0] {
                b.light_panel(
                    Vec3::new(29.0, 5.94, z),
                    Vec3::new(2.6, 0.05, 0.32),
                    if (z as i32) % 2 == 0 {
                        EMIT_GREEN
                    } else {
                        EMIT_WARM
                    },
                );
            }
        },
    );
}

fn pump_unit(b: &mut ChunkBuilder, center: Vec3, front: Vec3) {
    let along_x = front.x.abs() > front.z.abs();
    let (sx, sz) = if along_x { (1.8, 1.6) } else { (1.6, 1.8) };
    b.solid_box(
        Vec3::new(center.x, 0.18, center.z),
        Vec3::new(sx + 0.7, 0.36, sz + 0.7),
        DARK_METAL,
    );
    b.solid_box(
        Vec3::new(center.x, 1.15, center.z),
        Vec3::new(sx, 1.7, sz),
        METAL,
    );
    b.cylinder_y(
        Vec3::new(center.x, 1.9, center.z),
        0.65,
        0.34,
        12,
        DARK_METAL,
        0.0,
    );
    b.cylinder_y(
        Vec3::new(center.x, 2.24, center.z),
        0.3,
        0.55,
        10,
        PIPE_RUST,
        0.0,
    );
    let panel_center = if along_x {
        Vec3::new(center.x + front.x * (sx * 0.5 + 0.03), 1.5, center.z)
    } else {
        Vec3::new(center.x, 1.5, center.z + front.z * (sz * 0.5 + 0.03))
    };
    let panel_size = if along_x {
        Vec3::new(0.06, 0.34, 0.9)
    } else {
        Vec3::new(0.9, 0.34, 0.06)
    };
    b.emissive_box(panel_center, panel_size, EMIT_CYAN, 4.5);
    // Valve wheel on the body front.
    let wheel_base = if along_x {
        Vec3::new(center.x + front.x * (sx * 0.5 + 0.08), 0.95, center.z)
    } else {
        Vec3::new(center.x, 0.95, center.z + front.z * (sz * 0.5 + 0.08))
    };
    let wheel_out = if along_x {
        Vec3::X * front.x
    } else {
        Vec3::Z * front.z
    };
    b.cylinder_between(
        wheel_base,
        wheel_base + wheel_out * 0.08,
        0.28,
        12,
        PIPE_RUST,
        0.0,
    );
    b.visual_box(
        Vec3::new(center.x, 2.72, center.z),
        Vec3::new(0.5, 0.24, 0.5),
        TRIM,
    );
}

fn server_rack(b: &mut ChunkBuilder, base: Vec3, front: Vec3) {
    let along_x = front.x.abs() > front.z.abs();
    let (sx, sz) = if along_x { (1.2, 1.8) } else { (1.8, 1.2) };
    b.solid_box(
        Vec3::new(base.x, 1.5, base.z),
        Vec3::new(sx, 3.0, sz),
        DARK_METAL,
    );
    b.visual_box(
        Vec3::new(base.x, 3.05, base.z),
        Vec3::new(sx + 0.14, 0.1, sz + 0.14),
        METAL,
    );
    let face = if along_x {
        Vec3::new(base.x + front.x * (sx * 0.5 + 0.02), 0.0, base.z)
    } else {
        Vec3::new(base.x, 0.0, base.z + front.z * (sz * 0.5 + 0.02))
    };
    for index in 0..6 {
        let y = 0.55 + index as f32 * 0.4;
        let size = if along_x {
            Vec3::new(0.04, 0.16, sz * 0.72)
        } else {
            Vec3::new(sx * 0.72, 0.16, 0.04)
        };
        let color = if index % 3 == 0 {
            EMIT_GREEN
        } else {
            EMIT_CYAN
        };
        b.emissive_box(Vec3::new(face.x, y, face.z), size, color, 2.6);
    }
}

fn holo_table(b: &mut ChunkBuilder, base: Vec3) {
    b.cylinder_y(base, 1.55, 0.85, 16, DARK_METAL, 0.0);
    b.cylinder_y(base + Vec3::Y * 0.85, 1.2, 0.08, 16, METAL, 0.0);
    b.cylinder_y(base + Vec3::Y * 0.94, 1.0, 0.04, 16, EMIT_CYAN, 3.2);
    b.cylinder_y(base + Vec3::Y * 0.98, 0.86, 0.05, 16, DARK_METAL, 0.0);
    b.cylinder_y(base + Vec3::Y * 1.03, 0.18, 0.04, 12, EMIT_CYAN, 3.0);
    for index in 0..4 {
        let angle = TAU * index as f32 / 4.0 + PI * 0.25;
        let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * 2.05;
        b.solid_box(
            Vec3::new(base.x + offset.x, 0.55, base.z + offset.z),
            Vec3::new(0.28, 1.1, 0.28),
            METAL,
        );
        b.emissive_box(
            Vec3::new(base.x + offset.x, 1.16, base.z + offset.z),
            Vec3::new(0.2, 0.08, 0.2),
            EMIT_CYAN,
            4.0,
        );
    }
    b.colliders.push(Aabb::new(
        Vec3::new(base.x - 1.55, 0.0, base.z - 1.55),
        Vec3::new(base.x + 1.55, 0.93, base.z + 1.55),
    ));
}

fn crate_stack(b: &mut ChunkBuilder, base: Vec3, levels: usize, color: [f32; 3]) {
    for level in 0..levels {
        let y = 0.6 + level as f32 * 1.24;
        let offset = if level % 2 == 0 { 0.0 } else { 0.18 };
        b.solid_box(
            Vec3::new(base.x + offset, y, base.z),
            Vec3::splat(1.2),
            color,
        );
        b.visual_box(
            Vec3::new(base.x + offset, y + 0.62, base.z),
            Vec3::new(1.22, 0.05, 1.22),
            DARK_METAL,
        );
    }
}

fn add_reactor_chamber(assembly: &mut Assembly) {
    assembly.chunk(
        Aabb::new(Vec3::new(-16.0, 0.0, -48.0), Vec3::new(16.0, 12.0, -22.0)),
        |b| {
            b.floor(-16.0, 16.0, -48.0, -22.0, 0.0, FLOOR_DARK);
            b.ceiling(-16.0, 16.0, -48.0, -22.0, 12.0, CEILING);
            b.wall_x_with_door(-16.0, -48.0, -22.0, 0.0, 12.0, -22.0, -16.0, 1.0, WALL);
            b.wall_x_with_door(16.0, -48.0, -22.0, 0.0, 12.0, -22.0, -16.0, -1.0, WALL);
            b.wall_z(-48.0, -16.0, 16.0, 0.0, 12.0, 1.0, WALL);
            b.wall_z_with_door(-22.0, -16.0, 16.0, 0.0, 12.0, -3.0, 3.0, -1.0, WALL);
            door_frame_x(b, -15.8, -22.15, -15.85, 3.2);
            door_frame_x(b, 15.8, -22.15, -15.85, 3.2);
            door_frame_z(b, -21.8, -3.15, 3.15, 3.2);

            // Raised reactor plinth and steps.
            b.solid_box(
                Vec3::new(0.0, 0.3, -35.0),
                Vec3::new(10.0, 0.6, 10.0),
                METAL,
            );
            b.stairs_z(-2.6, 2.6, -29.2, 2, 0.3, 0.45, true, DARK_METAL);
            b.hazard_band_x(-30.02, -5.0, -2.7, 0.62);
            b.hazard_band_x(-30.02, 2.7, 5.0, 0.62);

            // Reactor core.
            b.cylinder_y(Vec3::new(0.0, 0.6, -35.0), 2.7, 6.4, 14, DARK_METAL, 0.0);
            b.colliders.push(Aabb::new(
                Vec3::new(-2.7, 0.6, -37.7),
                Vec3::new(2.7, 7.0, -32.3),
            ));
            for y in [2.35, 4.1, 5.85] {
                b.cylinder_y(Vec3::new(0.0, y, -35.0), 2.74, 0.14, 14, METAL, 0.0);
            }
            for y in [1.7, 3.3, 4.9] {
                b.cylinder_y(Vec3::new(0.0, y, -35.0), 2.8, 0.12, 14, EMIT_RED, 3.6);
            }
            for index in 0..8 {
                let angle = TAU * index as f32 / 8.0;
                let x = angle.cos() * 2.7;
                let z = -35.0 + angle.sin() * 2.7;
                b.visual_box(Vec3::new(x, 3.8, z), Vec3::new(0.12, 6.1, 0.12), DARK_METAL);
                b.emissive_box(
                    Vec3::new(x * 1.015, 3.8, -35.0 + (z + 35.0) * 1.015),
                    Vec3::new(0.065, 5.3, 0.065),
                    EMIT_RED,
                    1.5,
                );
            }
            b.cylinder_y(Vec3::new(0.0, 0.68, -35.0), 2.86, 0.1, 14, EMIT_RED, 2.4);
            b.cylinder_y(Vec3::new(0.0, 6.75, -35.0), 2.35, 0.12, 14, EMIT_CYAN, 2.6);
            b.cylinder_y(Vec3::new(0.0, 7.0, -35.0), 1.4, 0.5, 12, METAL, 0.0);
            b.cylinder_y(Vec3::new(0.0, 7.5, -35.0), 0.4, 0.9, 8, EMIT_RED, 3.0);

            // Core support pylons.
            for (x, z) in [(-5.6, -29.4), (5.6, -29.4), (-5.6, -40.6), (5.6, -40.6)] {
                b.solid_box(Vec3::new(x, 0.3, z), Vec3::new(1.1, 0.6, 1.1), DARK_METAL);
                b.pipe(Vec3::new(x, 0.6, z), Vec3::new(x, 11.9, z), 0.32, 10, METAL);
                b.cylinder_y(Vec3::new(x, 2.5, z), 0.42, 0.22, 10, EMIT_RED, 1.8);
            }

            // Coolant lines enter the core from both sides, but rise above the
            // catwalk headroom and drop through elbows so players never clip
            // through a pipe at eye height.
            for x in [-15.8_f32, 15.8] {
                let inward = x.signum();
                b.pipe(
                    Vec3::new(x, 6.5, -35.0),
                    Vec3::new(inward * 6.2, 6.5, -35.0),
                    0.22,
                    10,
                    PIPE_BLUE,
                );
                b.pipe(
                    Vec3::new(inward * 6.2, 6.5, -35.0),
                    Vec3::new(inward * 6.2, 5.2, -35.0),
                    0.22,
                    10,
                    PIPE_BLUE,
                );
                b.pipe(
                    Vec3::new(inward * 6.2, 5.2, -35.0),
                    Vec3::new(inward * 2.8, 5.2, -35.0),
                    0.22,
                    10,
                    PIPE_BLUE,
                );
            }

            // Catwalks along both walls plus a north bridge.
            b.solid_box(
                Vec3::new(-14.0, 3.2, -35.0),
                Vec3::new(4.0, 0.4, 22.0),
                DARK_METAL,
            );
            b.solid_box(
                Vec3::new(14.0, 3.2, -35.0),
                Vec3::new(4.0, 0.4, 22.0),
                DARK_METAL,
            );
            b.solid_box(
                Vec3::new(0.0, 3.2, -44.0),
                Vec3::new(24.0, 0.4, 4.0),
                DARK_METAL,
            );
            b.stairs_x(-30.5, -28.0, 8.2, 10, 0.34, 0.42, true, METAL);

            // The west catwalk opens onto the north bridge; the east catwalk
            // opens both onto the bridge and onto the stair landing.
            b.railing_z(-12.0, -42.0, -24.0, 3.4, HAZARD_YELLOW);
            b.railing_z(12.0, -42.0, -30.5, 3.4, HAZARD_YELLOW);
            b.railing_z(12.0, -28.0, -24.0, 3.4, HAZARD_YELLOW);
            b.railing_x(-42.0, -12.0, 12.0, 3.4, HAZARD_YELLOW);
            b.railing_x(-24.0, -16.0, -12.0, 3.4, HAZARD_YELLOW);
            b.railing_x(-24.0, 12.0, 16.0, 3.4, HAZARD_YELLOW);

            // Edge lighting makes the catwalks readable from the dark floor
            // below even where no local light reaches the deck.
            b.emissive_box(
                Vec3::new(-12.06, 3.46, -33.0),
                Vec3::new(0.05, 0.035, 18.0),
                EMIT_CYAN,
                1.8,
            );
            b.emissive_box(
                Vec3::new(12.06, 3.46, -36.25),
                Vec3::new(0.05, 0.035, 11.5),
                EMIT_WARM,
                1.8,
            );
            b.emissive_box(
                Vec3::new(12.06, 3.46, -26.0),
                Vec3::new(0.05, 0.035, 4.0),
                EMIT_WARM,
                1.8,
            );
            b.emissive_box(
                Vec3::new(0.0, 3.46, -41.94),
                Vec3::new(24.0, 0.035, 0.05),
                EMIT_CYAN,
                1.8,
            );

            // Ceiling fixtures and core glow.
            for (x, z) in [(-6.0, -35.0), (6.0, -35.0), (0.0, -41.0), (0.0, -29.0)] {
                b.light_panel(
                    Vec3::new(x, 11.92, z),
                    Vec3::new(3.0, 0.06, 0.35),
                    EMIT_WARM,
                );
            }
            b.emissive_box(
                Vec3::new(0.0, 11.86, -35.0),
                Vec3::new(2.6, 0.08, 2.6),
                EMIT_RED,
                1.3,
            );

            // Wall greebles, vents and warning lights.
            for z in [-44.0, -26.0] {
                for x in [-15.75, 15.75] {
                    b.visual_box(Vec3::new(x, 5.6, z), Vec3::new(0.05, 2.2, 2.6), DARK_METAL);
                    for index in 0..5 {
                        b.visual_box(
                            Vec3::new(x - x.signum() * 0.03, 4.85 + index as f32 * 0.36, z),
                            Vec3::new(0.04, 0.07, 2.2),
                            METAL,
                        );
                    }
                }
            }
            for x in [-11.0, 11.0] {
                b.emissive_box(
                    Vec3::new(x, 8.4, -47.78),
                    Vec3::new(0.7, 0.16, 0.06),
                    EMIT_RED,
                    2.0,
                );
            }
            b.pipe(
                Vec3::new(-15.0, 10.8, -47.5),
                Vec3::new(15.0, 10.8, -47.5),
                0.17,
                10,
                PIPE_RUST,
            );
        },
    );
}

fn add_loop_corridors(assembly: &mut Assembly) {
    // North-west loop: pump room -> reactor chamber.
    assembly.chunk(
        Aabb::new(Vec3::new(-32.0, 0.0, -22.0), Vec3::new(-16.0, 4.0, -12.0)),
        |b| {
            b.floor(-32.0, -26.0, -22.0, -12.0, 0.0, FLOOR_DARK);
            b.floor(-26.0, -16.0, -22.0, -16.0, 0.0, FLOOR_DARK);
            b.ceiling(-32.0, -26.0, -22.0, -12.0, 4.0, CEILING);
            b.ceiling(-26.0, -16.0, -22.0, -16.0, 4.0, CEILING);
            b.wall_x(-32.0, -22.0, -12.0, 0.0, 4.0, 1.0, WALL);
            b.wall_z(-22.0, -32.0, -16.0, 0.0, 4.0, 1.0, WALL);
            b.wall_z(-16.0, -26.0, -16.0, 0.0, 4.0, -1.0, WALL);
            b.wall_x(-26.0, -16.0, -12.0, 0.0, 4.0, -1.0, WALL);
            door_frame_z(b, -12.2, -32.15, -25.85, 3.2);
            door_frame_x(b, -16.2, -22.15, -15.85, 3.2);

            b.emissive_box(
                Vec3::new(-29.0, 3.92, -17.0),
                Vec3::new(0.5, 0.06, 1.6),
                EMIT_CYAN,
                4.5,
            );
            b.emissive_box(
                Vec3::new(-21.0, 3.92, -19.0),
                Vec3::new(1.6, 0.06, 0.5),
                EMIT_CYAN,
                4.5,
            );
            b.pipe(
                Vec3::new(-31.6, 3.55, -21.6),
                Vec3::new(-16.4, 3.55, -21.6),
                0.1,
                8,
                PIPE_BLUE,
            );
            b.hazard_band_x(-21.4, -31.8, -26.2, 0.04);
        },
    );

    // North-east loop: relay room -> reactor chamber.
    assembly.chunk(
        Aabb::new(Vec3::new(16.0, 0.0, -22.0), Vec3::new(32.0, 4.0, -12.0)),
        |b| {
            b.floor(26.0, 32.0, -22.0, -12.0, 0.0, FLOOR_DARK);
            b.floor(16.0, 26.0, -22.0, -16.0, 0.0, FLOOR_DARK);
            b.ceiling(26.0, 32.0, -22.0, -12.0, 4.0, CEILING);
            b.ceiling(16.0, 26.0, -22.0, -16.0, 4.0, CEILING);
            b.wall_x(32.0, -22.0, -12.0, 0.0, 4.0, -1.0, WALL);
            b.wall_z(-22.0, 16.0, 32.0, 0.0, 4.0, 1.0, WALL);
            b.wall_z(-16.0, 16.0, 26.0, 0.0, 4.0, -1.0, WALL);
            b.wall_x(26.0, -16.0, -12.0, 0.0, 4.0, 1.0, WALL);
            door_frame_z(b, -12.2, 25.85, 32.15, 3.2);
            door_frame_x(b, 16.2, -22.15, -15.85, 3.2);

            b.emissive_box(
                Vec3::new(29.0, 3.92, -17.0),
                Vec3::new(0.5, 0.06, 1.6),
                EMIT_GREEN,
                4.5,
            );
            b.emissive_box(
                Vec3::new(21.0, 3.92, -19.0),
                Vec3::new(1.6, 0.06, 0.5),
                EMIT_GREEN,
                4.5,
            );
            b.pipe(
                Vec3::new(16.4, 3.55, -21.6),
                Vec3::new(31.6, 3.55, -21.6),
                0.1,
                8,
                PIPE_BLUE,
            );
            b.hazard_band_x(-21.4, 26.2, 31.8, 0.04);
        },
    );
}

fn create_lights() -> Vec<Light> {
    // Exactly eight local lights: the forward shader uploads the nearest
    // eight, so matching the budget prevents visible popping when the camera
    // moves through the facility.
    vec![
        Light::new(Vec3::new(0.0, 6.4, 0.0), Vec3::new(1.0, 0.83, 0.66), 21.0).with_intensity(1.05),
        Light::new(Vec3::new(-26.0, 5.3, 0.0), Vec3::new(0.62, 0.84, 1.0), 23.0)
            .with_intensity(1.15),
        Light::new(Vec3::new(26.0, 5.3, 0.0), Vec3::new(0.62, 1.0, 0.82), 23.0).with_intensity(1.0),
        Light::new(Vec3::new(0.0, 8.2, -35.0), Vec3::new(1.0, 0.3, 0.14), 16.0)
            .with_intensity(1.25),
        Light::new(Vec3::new(0.0, 3.6, -16.0), Vec3::new(1.0, 0.86, 0.7), 15.0).with_intensity(0.8),
        Light::new(
            Vec3::new(-13.5, 6.2, -35.0),
            Vec3::new(1.0, 0.8, 0.58),
            15.0,
        )
        .with_intensity(1.15),
        Light::new(Vec3::new(13.5, 6.2, -35.0), Vec3::new(0.7, 0.9, 1.0), 15.0)
            .with_intensity(1.15),
        Light::new(Vec3::new(0.0, 6.4, -27.8), Vec3::new(0.8, 0.88, 1.0), 24.0).with_intensity(1.0),
    ]
}

fn add_checkpoints(assembly: &mut Assembly) {
    assembly.checkpoint(
        "Coolant pumps",
        Aabb::new(Vec3::new(-18.7, 0.0, -2.6), Vec3::new(-16.5, 2.6, 2.6)),
        Vec3::new(-18.0, 0.002, 0.0),
        PI,
    );
    assembly.checkpoint(
        "Relay room",
        Aabb::new(Vec3::new(16.5, 0.0, -2.6), Vec3::new(18.7, 2.6, 2.6)),
        Vec3::new(18.0, 0.002, 0.0),
        0.0,
    );
    assembly.checkpoint(
        "Reactor access",
        Aabb::new(Vec3::new(-2.6, 0.0, -25.7), Vec3::new(2.6, 2.6, -22.5)),
        Vec3::new(0.0, 0.002, -24.5),
        -PI * 0.5,
    );
    assembly.checkpoint(
        "Reactor catwalk",
        Aabb::new(Vec3::new(12.2, 3.1, -40.0), Vec3::new(15.8, 5.6, -33.0)),
        Vec3::new(14.0, 3.41, -35.0),
        PI * 0.5,
    );
}

fn add_interactables(assembly: &mut Assembly) {
    assembly.interactable(
        Device::CoolantPump(0),
        Vec3::new(-31.9, 1.4, -6.0),
        2.5,
        "Start coolant pump A",
    );
    assembly.interactable(
        Device::CoolantPump(1),
        Vec3::new(-26.9, 1.4, -8.3),
        2.5,
        "Start coolant pump B",
    );
    assembly.interactable(
        Device::CoolantPump(2),
        Vec3::new(-31.9, 1.4, 6.5),
        2.5,
        "Start coolant pump C",
    );
    assembly.interactable(
        Device::Reactor,
        Vec3::new(0.0, 2.0, -31.9),
        4.2,
        "Engage reactor core",
    );
}

/// Deterministic industrial panel texture. The mesh UVs tile in world space,
/// so the texture reproduces a two-metre panel grid across every surface.
fn generate_panel_texture() -> Vec<u8> {
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
            let mut value = 0.52 + hash(x, y) * 0.1;
            value += hash(x / 3, y / 3) * 0.06;
            value -= (hash(x / 24, y / 24) - 0.42).max(0.0) * 0.24;

            if cell_x < 2 || cell_y < 2 {
                value = 0.27 + hash(x, y) * 0.06;
            } else {
                if cell_x < 5 || cell_y < 5 {
                    value += 0.075;
                }
                if cell_x > 122 || cell_y > 122 {
                    value -= 0.09;
                }
            }

            for (rx, ry) in [(12, 12), (116, 12), (12, 116), (116, 116)] {
                let dx = cell_x as i32 - rx;
                let dy = cell_y as i32 - ry;
                if dx * dx + dy * dy < 18 {
                    value += if dx + dy < 0 { 0.18 } else { -0.1 };
                }
            }

            if hash(x / 2 + 71, y / 9 + 13) > 0.988 {
                value -= 0.12;
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

    #[test]
    fn facility_respects_light_budget() {
        let facility = build_facility();
        assert_eq!(facility.scene.lights.len(), 8);
        assert_eq!(facility.interactables.len(), 4);
        assert!(facility.checkpoints.len() >= 3);
        assert!(!facility.scene.meshes.is_empty());
    }

    #[test]
    fn spawn_stands_on_the_atrium_floor() {
        let facility = build_facility();
        let mut character = Character::new(SPAWN);
        for _ in 0..90 {
            character.update(
                1.0 / 60.0,
                &facility.physics,
                Vec3::ZERO,
                false,
                false,
                false,
            );
        }
        assert!(character.on_ground);
        assert!(character.position.y.abs() < 0.01);
    }

    #[test]
    fn reactor_platform_is_reachable() {
        let facility = build_facility();
        let mut character = Character::new(Vec3::new(0.0, 0.002, -27.0));
        for _ in 0..180 {
            character.update(1.0 / 60.0, &facility.physics, -Vec3::Z, false, false, false);
        }
        assert!(
            character.position.y > 0.55,
            "player should climb the plinth"
        );
        assert!(character.position.z < -31.0);
    }
}
