//! Procedural demo level: eight rooms/corridors, colliders, lights and a
//! generated wall texture.

use runtime::glam::Vec3;
use runtime::{
    Aabb, Camera, Light, Material, Mesh, PrimaryLight, Scene, ShadowSettings, TextureData, Vertex,
};

const FLOOR_COLOR: [f32; 3] = [0.5, 0.5, 0.5];
const CEILING_COLOR: [f32; 3] = [0.9, 0.9, 0.9];
const WALL_COLOR: [f32; 3] = [0.95, 0.95, 0.9];
const POOL_COLOR: [f32; 3] = [0.3, 0.5, 0.6];
const PILLAR_COLOR: [f32; 3] = [0.7, 0.7, 0.7];

/// Builds the demo scene and returns its gameplay colliders.
pub fn build_scene() -> (Scene, Vec<Aabb>) {
    let (chunks, colliders) = create_room_meshes();
    let material = Material {
        albedo: TextureData::new(256, 256, generate_wall_texture()),
        metallic: 0.0,
        roughness: 0.95,
    };

    let mut scene = Scene::new(material)
        .with_meshes(chunks)
        .with_lights(create_lights())
        .with_primary_light(PrimaryLight {
            position: Vec3::new(10.0, -1.5, 50.0),
            target: Vec3::new(10.0, -10.0, 50.0),
            up: Vec3::Z,
            color: Vec3::ONE,
            intensity: 1.0,
            shadow: ShadowSettings {
                ortho_half_extent: 50.0,
                near: 0.1,
                far: 100.0,
            },
        });
    scene.clear_color = [0.015, 0.017, 0.02, 1.0];
    (scene, colliders)
}

/// Named camera presets shipped by the demo.
///
/// The engine does not know about these viewpoints: they are ordinary
/// [`Camera`] values spawned into the [`runtime::World`] camera rig. Players
/// switch between them with `camera.select`, `camera.next` or the console.
pub fn camera_presets() -> Vec<(&'static str, Camera)> {
    vec![
        (
            "corridor",
            Camera::at(Vec3::new(0.0, -5.6, 11.0))
                .with_yaw_degrees(90.0)
                .with_pitch_degrees(-2.0)
                .with_fov_degrees(55.0)
                .with_range(0.1, 150.0),
        ),
        (
            "left-hall",
            Camera::at(Vec3::new(-12.0, -3.0, 42.0))
                .with_yaw_degrees(90.0)
                .with_pitch_degrees(-3.0)
                .with_fov_degrees(60.0)
                .with_range(0.1, 150.0),
        ),
        (
            "right-hall",
            Camera::at(Vec3::new(12.0, -3.0, 42.0))
                .with_yaw_degrees(90.0)
                .with_pitch_degrees(-3.0)
                .with_fov_degrees(60.0)
                .with_range(0.1, 150.0),
        ),
        (
            "pool",
            Camera::at(Vec3::new(58.0, -2.5, 42.0))
                .with_yaw_degrees(52.0)
                .with_pitch_degrees(-16.0)
                .with_fov_degrees(65.0)
                .with_range(0.1, 150.0),
        ),
        (
            "final-hall",
            Camera::at(Vec3::new(60.0, 2.0, 112.0))
                .with_yaw_degrees(149.0)
                .with_pitch_degrees(-13.0)
                .with_fov_degrees(62.0)
                .with_range(0.1, 200.0),
        ),
        (
            "overview",
            Camera::at(Vec3::new(70.0, 25.0, 150.0))
                .with_yaw_degrees(241.0)
                .with_pitch_degrees(-26.0)
                .with_fov_degrees(60.0)
                .with_range(0.1, 300.0),
        ),
    ]
}

fn create_lights() -> Vec<Light> {
    let data = [
        (Vec3::new(0.0, -0.1, 0.0), Vec3::new(1.0, 0.9, 0.7), 14.0),
        (Vec3::new(-5.0, -0.1, -5.0), Vec3::new(1.0, 0.9, 0.7), 14.0),
        (Vec3::new(5.0, -0.1, 5.0), Vec3::new(1.0, 0.9, 0.7), 14.0),
        (Vec3::new(0.0, -4.1, 15.0), Vec3::new(1.0, 0.95, 0.8), 12.0),
        (Vec3::new(0.0, -4.1, 22.0), Vec3::new(1.0, 0.95, 0.8), 12.0),
        (Vec3::new(0.0, -4.1, 30.0), Vec3::new(1.0, 0.95, 0.8), 12.0),
        (Vec3::new(-12.0, 4.9, 50.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(-12.0, 4.9, 65.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(-18.0, 4.9, 75.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(12.0, 4.9, 50.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(12.0, 4.9, 65.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(18.0, 4.9, 75.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(35.0, -6.1, 60.0), Vec3::new(1.0, 1.0, 1.0), 12.0),
        (Vec3::new(45.0, -6.1, 60.0), Vec3::new(1.0, 1.0, 1.0), 12.0),
        (Vec3::new(62.0, 1.9, 45.0), Vec3::new(0.6, 0.8, 1.0), 14.0),
        (Vec3::new(72.0, 1.9, 60.0), Vec3::new(0.6, 0.8, 1.0), 14.0),
        (Vec3::new(85.0, 1.9, 75.0), Vec3::new(0.6, 0.8, 1.0), 14.0),
        (Vec3::new(60.0, -6.1, 90.0), Vec3::new(1.0, 1.0, 1.0), 12.0),
        (Vec3::new(60.0, -6.1, 105.0), Vec3::new(1.0, 1.0, 1.0), 12.0),
        (Vec3::new(20.0, 4.9, 115.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(30.0, 4.9, 125.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
        (Vec3::new(40.0, 4.9, 135.0), Vec3::new(1.0, 0.85, 0.6), 18.0),
    ];
    data.into_iter()
        .map(|(position, color, radius)| Light::new(position, color, radius))
        .collect()
}

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

    fn floor(&mut self, x1: f32, x2: f32, z1: f32, z2: f32, y: f32, color: [f32; 3]) {
        let uv_x = (x2 - x1) * 0.5;
        let uv_z = (z2 - z1) * 0.5;
        let normal = [0.0, 1.0, 0.0];
        self.quad(
            [[x1, y, z1], [x2, y, z1], [x2, y, z2], [x1, y, z2]],
            normal,
            [[0.0, 0.0], [uv_x, 0.0], [uv_x, uv_z], [0.0, uv_z]],
            color,
            0.0,
        );
        self.colliders
            .push(Aabb::new(Vec3::new(x1, y - 1.0, z1), Vec3::new(x2, y, z2)));
    }

    fn ceiling(&mut self, x1: f32, x2: f32, z1: f32, z2: f32, y: f32, color: [f32; 3]) {
        let uv_x = (x2 - x1) * 0.5;
        let uv_z = (z2 - z1) * 0.5;
        let normal = [0.0, -1.0, 0.0];
        self.quad(
            [[x1, y, z1], [x1, y, z2], [x2, y, z2], [x2, y, z1]],
            normal,
            [[0.0, 0.0], [0.0, uv_z], [uv_x, uv_z], [uv_x, 0.0]],
            color,
            0.0,
        );
        self.colliders
            .push(Aabb::new(Vec3::new(x1, y, z1), Vec3::new(x2, y + 1.0, z2)));
    }

    #[allow(clippy::too_many_arguments)]
    fn wall_x(
        &mut self,
        x: f32,
        z1: f32,
        z2: f32,
        y1: f32,
        y2: f32,
        direction: f32,
        color: [f32; 3],
    ) {
        let normal = [direction, 0.0, 0.0];
        let uv_y = (y2 - y1) * 0.5;
        let uv_z = (z2 - z1) * 0.5;
        if direction > 0.0 {
            self.triangle(
                [[x, y1, z1], [x, y1, z2], [x, y2, z2]],
                normal,
                [[0.0, uv_y], [uv_z, uv_y], [uv_z, 0.0]],
                color,
                0.0,
            );
            self.triangle(
                [[x, y1, z1], [x, y2, z2], [x, y2, z1]],
                normal,
                [[0.0, uv_y], [uv_z, 0.0], [0.0, 0.0]],
                color,
                0.0,
            );
        } else {
            self.triangle(
                [[x, y1, z1], [x, y2, z2], [x, y1, z2]],
                normal,
                [[0.0, uv_y], [uv_z, 0.0], [uv_z, uv_y]],
                color,
                0.0,
            );
            self.triangle(
                [[x, y1, z1], [x, y2, z1], [x, y2, z2]],
                normal,
                [[0.0, uv_y], [0.0, 0.0], [uv_z, 0.0]],
                color,
                0.0,
            );
        }
        self.colliders.push(Aabb::new(
            Vec3::new(x - 0.5, y1, z1),
            Vec3::new(x + 0.5, y2, z2),
        ));
    }

    #[allow(clippy::too_many_arguments)]
    fn wall_z(
        &mut self,
        z: f32,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        direction: f32,
        color: [f32; 3],
    ) {
        let normal = [0.0, 0.0, direction];
        let uv_y = (y2 - y1) * 0.5;
        let uv_x = (x2 - x1) * 0.5;
        if direction > 0.0 {
            self.triangle(
                [[x1, y1, z], [x2, y2, z], [x2, y1, z]],
                normal,
                [[0.0, uv_y], [uv_x, 0.0], [uv_x, uv_y]],
                color,
                0.0,
            );
            self.triangle(
                [[x1, y1, z], [x1, y2, z], [x2, y2, z]],
                normal,
                [[0.0, uv_y], [0.0, 0.0], [uv_x, 0.0]],
                color,
                0.0,
            );
        } else {
            self.triangle(
                [[x1, y1, z], [x2, y1, z], [x2, y2, z]],
                normal,
                [[0.0, uv_y], [uv_x, uv_y], [uv_x, 0.0]],
                color,
                0.0,
            );
            self.triangle(
                [[x1, y1, z], [x2, y2, z], [x1, y2, z]],
                normal,
                [[0.0, uv_y], [uv_x, 0.0], [0.0, 0.0]],
                color,
                0.0,
            );
        }
        self.colliders.push(Aabb::new(
            Vec3::new(x1, y1, z - 0.5),
            Vec3::new(x2, y2, z + 0.5),
        ));
    }

    fn light_fixture(&mut self, x: f32, y: f32, z: f32) {
        let half = Vec3::new(1.0, 0.05, 1.0);
        let normal = [0.0, -1.0, 0.0];
        let color = [1.0, 1.0, 1.0];
        let a = [x - half.x, y - half.y, z - half.z];
        let b = [x + half.x, y - half.y, z - half.z];
        let c = [x + half.x, y - half.y, z + half.z];
        let d = [x - half.x, y - half.y, z + half.z];
        self.triangle(
            [a, c, b],
            normal,
            [[0.0, 0.0], [1.0, 1.0], [1.0, 0.0]],
            color,
            15.0,
        );
        self.triangle(
            [a, d, c],
            normal,
            [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            color,
            15.0,
        );
    }

    fn cube(&mut self, center: Vec3, size: Vec3, color: [f32; 3]) {
        let h = size * 0.5;
        let p = center;
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let faces = [
            (
                [
                    [p.x - h.x, p.y - h.y, p.z + h.z],
                    [p.x + h.x, p.y - h.y, p.z + h.z],
                    [p.x + h.x, p.y + h.y, p.z + h.z],
                    [p.x - h.x, p.y + h.y, p.z + h.z],
                ],
                [0.0, 0.0, 1.0],
            ),
            (
                [
                    [p.x + h.x, p.y - h.y, p.z - h.z],
                    [p.x - h.x, p.y - h.y, p.z - h.z],
                    [p.x - h.x, p.y + h.y, p.z - h.z],
                    [p.x + h.x, p.y + h.y, p.z - h.z],
                ],
                [0.0, 0.0, -1.0],
            ),
            (
                [
                    [p.x - h.x, p.y + h.y, p.z + h.z],
                    [p.x + h.x, p.y + h.y, p.z + h.z],
                    [p.x + h.x, p.y + h.y, p.z - h.z],
                    [p.x - h.x, p.y + h.y, p.z - h.z],
                ],
                [0.0, 1.0, 0.0],
            ),
            (
                [
                    [p.x - h.x, p.y - h.y, p.z - h.z],
                    [p.x + h.x, p.y - h.y, p.z - h.z],
                    [p.x + h.x, p.y - h.y, p.z + h.z],
                    [p.x - h.x, p.y - h.y, p.z + h.z],
                ],
                [0.0, -1.0, 0.0],
            ),
            (
                [
                    [p.x + h.x, p.y - h.y, p.z + h.z],
                    [p.x + h.x, p.y - h.y, p.z - h.z],
                    [p.x + h.x, p.y + h.y, p.z - h.z],
                    [p.x + h.x, p.y + h.y, p.z + h.z],
                ],
                [1.0, 0.0, 0.0],
            ),
            (
                [
                    [p.x - h.x, p.y - h.y, p.z - h.z],
                    [p.x - h.x, p.y - h.y, p.z + h.z],
                    [p.x - h.x, p.y + h.y, p.z + h.z],
                    [p.x - h.x, p.y + h.y, p.z - h.z],
                ],
                [-1.0, 0.0, 0.0],
            ),
        ];
        for (positions, normal) in faces {
            self.quad(positions, normal, uv, color, 0.0);
        }
        self.colliders.push(Aabb::new(p - h, p + h));
    }

    fn quad(
        &mut self,
        positions: [[f32; 3]; 4],
        normal: [f32; 3],
        uv: [[f32; 2]; 4],
        color: [f32; 3],
        emission: f32,
    ) {
        for index in [0, 1, 2, 0, 2, 3] {
            self.vertices.push(
                Vertex::new(positions[index], color, normal, uv[index]).with_emission(emission),
            );
        }
    }

    fn triangle(
        &mut self,
        positions: [[f32; 3]; 3],
        normal: [f32; 3],
        uv: [[f32; 2]; 3],
        color: [f32; 3],
        emission: f32,
    ) {
        for index in 0..3 {
            self.vertices.push(
                Vertex::new(positions[index], color, normal, uv[index]).with_emission(emission),
            );
        }
    }
}

fn create_room_meshes() -> (Vec<Mesh>, Vec<Aabb>) {
    let mut meshes = Vec::new();
    let mut all_colliders = Vec::new();

    // Room 1: spawn room.
    {
        let mut b = ChunkBuilder::new();
        b.floor(-10.0, 10.0, -10.0, 10.0, -10.0, FLOOR_COLOR);
        b.ceiling(-10.0, 10.0, -10.0, 10.0, 0.0, CEILING_COLOR);
        b.wall_x(-10.0, -10.0, 10.0, -10.0, 0.0, 1.0, WALL_COLOR);
        b.wall_x(10.0, -10.0, 10.0, -10.0, 0.0, -1.0, WALL_COLOR);
        b.wall_z(-10.0, -10.0, 10.0, -10.0, 0.0, 1.0, WALL_COLOR);
        b.wall_z(10.0, -10.0, -3.0, -10.0, 0.0, -1.0, WALL_COLOR);
        b.wall_z(10.0, 3.0, 10.0, -10.0, 0.0, -1.0, WALL_COLOR);
        b.wall_z(10.0, -3.0, 3.0, -4.0, 0.0, -1.0, WALL_COLOR);
        b.light_fixture(0.0, -0.1, 0.0);
        b.light_fixture(-5.0, -0.1, -5.0);
        b.light_fixture(5.0, -0.1, 5.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(-10.0, -10.0, -10.0), Vec3::new(10.0, 0.0, 10.0)),
        );
    }

    // Corridor 1.
    {
        let mut b = ChunkBuilder::new();
        b.floor(-3.0, 3.0, 10.0, 40.0, -10.0, FLOOR_COLOR);
        b.ceiling(-3.0, 3.0, 10.0, 40.0, -4.0, CEILING_COLOR);
        b.wall_x(-3.0, 10.0, 40.0, -10.0, -4.0, 1.0, WALL_COLOR);
        b.wall_x(3.0, 10.0, 40.0, -10.0, -4.0, -1.0, WALL_COLOR);
        b.light_fixture(0.0, -4.1, 15.0);
        b.light_fixture(0.0, -4.1, 22.0);
        b.light_fixture(0.0, -4.1, 30.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(-3.0, -10.0, 10.0), Vec3::new(3.0, -4.0, 40.0)),
        );
    }

    // Left pillared hall.
    {
        let mut b = ChunkBuilder::new();
        b.floor(-25.0, 0.0, 40.0, 80.0, -10.0, FLOOR_COLOR);
        b.ceiling(-25.0, 0.0, 40.0, 80.0, 5.0, CEILING_COLOR);
        b.wall_x(-25.0, 40.0, 80.0, -10.0, 5.0, 1.0, WALL_COLOR);
        b.wall_z(40.0, -25.0, -3.0, -10.0, 5.0, 1.0, WALL_COLOR);
        b.wall_z(40.0, -3.0, 0.0, -4.0, 5.0, 1.0, WALL_COLOR);
        b.wall_z(80.0, -25.0, 0.0, -10.0, 5.0, -1.0, WALL_COLOR);
        for x in [-15.0, -5.0] {
            for z in [50.0, 60.0, 70.0] {
                b.cube(
                    Vec3::new(x, -2.5, z),
                    Vec3::new(2.0, 15.0, 2.0),
                    PILLAR_COLOR,
                );
            }
        }
        b.light_fixture(-12.0, 4.9, 50.0);
        b.light_fixture(-12.0, 4.9, 65.0);
        b.light_fixture(-18.0, 4.9, 75.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(-25.0, -10.0, 40.0), Vec3::new(0.0, 5.0, 80.0)),
        );
    }

    // Right pillared hall.
    {
        let mut b = ChunkBuilder::new();
        b.floor(0.0, 25.0, 40.0, 80.0, -10.0, FLOOR_COLOR);
        b.ceiling(0.0, 25.0, 40.0, 80.0, 5.0, CEILING_COLOR);
        b.wall_z(40.0, 0.0, 3.0, -4.0, 5.0, 1.0, WALL_COLOR);
        b.wall_z(40.0, 3.0, 25.0, -10.0, 5.0, 1.0, WALL_COLOR);
        b.wall_x(25.0, 40.0, 55.0, -10.0, 5.0, -1.0, WALL_COLOR);
        b.wall_x(25.0, 55.0, 65.0, -6.0, 5.0, -1.0, WALL_COLOR);
        b.wall_x(25.0, 65.0, 80.0, -10.0, 5.0, -1.0, WALL_COLOR);
        b.wall_z(80.0, 0.0, 25.0, -10.0, 5.0, -1.0, WALL_COLOR);
        for x in [5.0, 15.0] {
            for z in [50.0, 60.0, 70.0] {
                b.cube(
                    Vec3::new(x, -2.5, z),
                    Vec3::new(2.0, 15.0, 2.0),
                    PILLAR_COLOR,
                );
            }
        }
        b.light_fixture(12.0, 4.9, 50.0);
        b.light_fixture(12.0, 4.9, 65.0);
        b.light_fixture(18.0, 4.9, 75.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(0.0, -10.0, 40.0), Vec3::new(25.0, 5.0, 80.0)),
        );
    }

    // Corridor 2.
    {
        let mut b = ChunkBuilder::new();
        b.floor(25.0, 55.0, 55.0, 65.0, -10.0, FLOOR_COLOR);
        b.ceiling(25.0, 55.0, 55.0, 65.0, -6.0, CEILING_COLOR);
        b.wall_z(55.0, 25.0, 55.0, -10.0, -6.0, -1.0, WALL_COLOR);
        b.wall_z(65.0, 25.0, 55.0, -10.0, -6.0, 1.0, WALL_COLOR);
        b.light_fixture(35.0, -6.1, 60.0);
        b.light_fixture(45.0, -6.1, 60.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(25.0, -10.0, 55.0), Vec3::new(55.0, -6.0, 65.0)),
        );
    }

    // Pool room.
    {
        let mut b = ChunkBuilder::new();
        b.floor(55.0, 65.0, 40.0, 80.0, -10.0, FLOOR_COLOR);
        b.floor(80.0, 90.0, 40.0, 80.0, -10.0, FLOOR_COLOR);
        b.floor(65.0, 80.0, 40.0, 50.0, -10.0, FLOOR_COLOR);
        b.floor(65.0, 80.0, 70.0, 80.0, -10.0, FLOOR_COLOR);
        b.floor(65.0, 80.0, 50.0, 70.0, -9.0, POOL_COLOR);
        b.ceiling(55.0, 90.0, 40.0, 80.0, 2.0, CEILING_COLOR);
        b.wall_x(90.0, 40.0, 80.0, -10.0, 2.0, -1.0, WALL_COLOR);
        b.wall_x(55.0, 40.0, 55.0, -10.0, 2.0, 1.0, WALL_COLOR);
        b.wall_x(55.0, 55.0, 65.0, -6.0, 2.0, 1.0, WALL_COLOR);
        b.wall_x(55.0, 65.0, 80.0, -10.0, 2.0, 1.0, WALL_COLOR);
        b.wall_z(40.0, 55.0, 90.0, -10.0, 2.0, 1.0, WALL_COLOR);
        b.wall_z(80.0, 55.0, 65.0, -6.0, 2.0, -1.0, WALL_COLOR);
        b.wall_z(80.0, 65.0, 90.0, -10.0, 2.0, -1.0, WALL_COLOR);
        b.light_fixture(62.0, 1.9, 45.0);
        b.light_fixture(72.0, 1.9, 60.0);
        b.light_fixture(85.0, 1.9, 75.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(55.0, -10.0, 40.0), Vec3::new(90.0, 2.0, 80.0)),
        );
    }

    // Corridor 3.
    {
        let mut b = ChunkBuilder::new();
        b.floor(55.0, 65.0, 80.0, 110.0, -10.0, FLOOR_COLOR);
        b.ceiling(55.0, 65.0, 80.0, 110.0, -6.0, CEILING_COLOR);
        b.wall_x(55.0, 80.0, 110.0, -10.0, -6.0, 1.0, WALL_COLOR);
        b.wall_x(65.0, 80.0, 110.0, -10.0, -6.0, -1.0, WALL_COLOR);
        b.light_fixture(60.0, -6.1, 90.0);
        b.light_fixture(60.0, -6.1, 105.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(55.0, -10.0, 80.0), Vec3::new(65.0, -6.0, 110.0)),
        );
    }

    // Final hall.
    {
        let mut b = ChunkBuilder::new();
        b.floor(10.0, 65.0, 110.0, 140.0, -10.0, FLOOR_COLOR);
        b.ceiling(10.0, 65.0, 110.0, 140.0, 5.0, CEILING_COLOR);
        b.wall_x(10.0, 110.0, 140.0, -10.0, 5.0, 1.0, WALL_COLOR);
        b.wall_x(65.0, 110.0, 140.0, -10.0, 5.0, -1.0, WALL_COLOR);
        b.wall_z(140.0, 10.0, 65.0, -10.0, 5.0, -1.0, WALL_COLOR);
        b.wall_z(110.0, 10.0, 55.0, -10.0, 5.0, 1.0, WALL_COLOR);
        b.wall_z(110.0, 55.0, 65.0, -6.0, 5.0, 1.0, WALL_COLOR);
        b.cube(
            Vec3::new(20.0, -9.0, 120.0),
            Vec3::splat(2.0),
            [0.6, 0.6, 0.6],
        );
        b.cube(
            Vec3::new(35.0, -9.0, 125.0),
            Vec3::splat(2.0),
            [0.6, 0.6, 0.6],
        );
        b.cube(
            Vec3::new(40.0, -9.0, 130.0),
            Vec3::splat(2.0),
            [0.6, 0.6, 0.6],
        );
        b.cube(
            Vec3::new(30.0, -9.0, 135.0),
            Vec3::splat(2.0),
            [0.6, 0.6, 0.6],
        );
        b.cube(
            Vec3::new(15.0, -2.5, 120.0),
            Vec3::new(2.0, 15.0, 2.0),
            PILLAR_COLOR,
        );
        b.cube(
            Vec3::new(60.0, -2.5, 120.0),
            Vec3::new(2.0, 15.0, 2.0),
            PILLAR_COLOR,
        );
        b.light_fixture(20.0, 4.9, 115.0);
        b.light_fixture(30.0, 4.9, 125.0);
        b.light_fixture(40.0, 4.9, 135.0);
        push_chunk(
            &mut meshes,
            &mut all_colliders,
            b,
            Aabb::new(Vec3::new(10.0, -10.0, 110.0), Vec3::new(65.0, 5.0, 140.0)),
        );
    }

    (meshes, all_colliders)
}

fn push_chunk(
    meshes: &mut Vec<Mesh>,
    all_colliders: &mut Vec<Aabb>,
    builder: ChunkBuilder,
    bounds: Aabb,
) {
    all_colliders.extend(builder.colliders);
    meshes.push(Mesh::new(builder.vertices, bounds));
}

/// Deterministic procedural wall texture.
pub fn generate_wall_texture() -> Vec<u8> {
    let size = 256u32;
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let noise = |x: u32, y: u32| -> f32 {
        let mut n = x
            .wrapping_mul(374_761_393)
            .wrapping_add(y.wrapping_mul(668_265_263));
        n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
        ((n & 0xFFFF) as f32) / 65535.0
    };
    for y in 0..size {
        for x in 0..size {
            let index = ((y * size + x) * 4) as usize;
            let mut value = 0.0;
            value += noise(x, y) * 0.5;
            value += noise(x * 2, y * 2) * 0.25;
            value += noise(x * 4, y * 4) * 0.125;
            value += noise(x * 8, y * 8) * 0.0625;
            let color = (100.0 + value * 60.0) as u8;
            pixels[index] = color;
            pixels[index + 1] = color;
            pixels[index + 2] = color;
            pixels[index + 3] = 255;
        }
    }
    pixels
}
