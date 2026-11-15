# Basic Example

A minimal complete game: one colored quad, a camera, keyboard input and a HUD.
It uses only the public `runtime` API.

## Create the project

```bash
cargo build --release -p s3d
./target/release/s3d new basic_game
cd basic_game
```

Replace `src/lib.rs` with the code below and leave `src/main.rs` as generated.

## Code

```rust
//! Basic Soldatik3D example: one quad, a moving camera and a HUD.

use runtime::glam::Vec3;
use runtime::{
    run, Aabb, Camera, CameraRig, Game, GameConfig, InputMap, Material, Mesh,
    PrimaryLight, Scene, TextureData, UpdateContext, Vertex, World,
};
use runtime::winit::keyboard::KeyCode;

pub struct BasicGame {
    time: f32,
    distance: f32,
}

impl Default for BasicGame {
    fn default() -> Self {
        Self {
            time: 0.0,
            distance: 5.0,
        }
    }
}

fn quad(size: f32) -> Mesh {
    let half = size * 0.5;
    let positions = [
        [-half, -half, 0.0],
        [half, -half, 0.0],
        [half, half, 0.0],
        [-half, half, 0.0],
    ];
    let colors = [
        [0.9, 0.25, 0.2],
        [0.2, 0.8, 0.35],
        [0.25, 0.45, 0.95],
        [0.95, 0.8, 0.2],
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut vertices = Vec::with_capacity(6);
    for index in [0, 1, 2, 0, 2, 3] {
        vertices.push(Vertex::new(
            positions[index],
            colors[index],
            [0.0, 0.0, 1.0],
            uvs[index],
        ));
    }
    Mesh::new(
        vertices,
        Aabb::new(
            Vec3::new(-half, -half, -0.01),
            Vec3::new(half, half, 0.01),
        ),
    )
}

impl Game for BasicGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("Basic Soldatik3D Example")
            .with_size(1280, 720)
    }

    fn create_world(&mut self) -> World {
        let material = Material {
            albedo: TextureData::white(),
            metallic: 0.1,
            roughness: 0.7,
        };
        let scene = Scene::new(material)
            .with_meshes(vec![quad(2.0)])
            .with_primary_light(PrimaryLight::default())
            .with_clear_color([0.02, 0.03, 0.05, 1.0]);

        let mut cameras = CameraRig::empty();
        cameras.spawn("main", Camera::at(Vec3::new(0.0, 0.0, 5.0)));
        World::with_cameras(scene, cameras)
    }

    fn input_map(&self) -> InputMap {
        InputMap::new()
            .bind("closer", &[KeyCode::KeyW, KeyCode::ArrowUp])
            .bind("farther", &[KeyCode::KeyS, KeyCode::ArrowDown])
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        self.time += ctx.dt;

        if ctx.input.action_down("closer") {
            self.distance = (self.distance - ctx.dt * 3.0).max(2.5);
        }
        if ctx.input.action_down("farther") {
            self.distance = (self.distance + ctx.dt * 3.0).min(12.0);
        }

        if let Some(camera) = ctx.world.active_camera_mut() {
            let angle = self.time * 0.5;
            camera.camera.position = Vec3::new(
                angle.sin() * self.distance,
                0.0,
                angle.cos() * self.distance,
            );
            // Look at the origin.
            camera.camera.yaw = (-camera.camera.position.z).atan2(-camera.camera.position.x);
            camera.camera.pitch = 0.0;
        }

        ctx.ui.rect(16.0, 16.0, 320.0, 72.0, [0.0, 0.0, 0.0, 0.55]);
        ctx.ui.text("BASIC EXAMPLE", 30.0, 28.0, 1.4, [1.0; 4]);
        ctx.ui.text(
            "W/S OR UP/DOWN: DISTANCE",
            30.0,
            56.0,
            1.0,
            [0.7, 0.85, 1.0, 1.0],
        );
    }
}

pub fn run_desktop() -> anyhow::Result<()> {
    run(BasicGame::default())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    if let Err(error) = runtime::run_android(BasicGame::default(), app) {
        eprintln!("game failed: {error:?}");
    }
}
```

## Run

```bash
cargo run
```

Try the OpenGL backend and the frame-limited smoke-test mode:

```bash
SOLDATIK3D_BACKEND=opengl cargo run
SOLDATIK3D_BACKEND=opengl SOLDATIK3D_MAX_FRAMES=2 cargo run
```

## What to notice

- `Scene` is built once. The quad is static; only the camera moves.
- `Camera::yaw` uses radians. `with_yaw_degrees` is only a constructor helper.
- `forward()` is not needed here because the camera always looks at the origin
  and pitch stays zero.
- The UI is drawn in physical pixels from the top-left.
- `Aabb` is slightly thicker than the quad so culling does not discard it when
  viewed exactly edge-on.

## Next steps

- [Advanced example](advanced.md) — procedural geometry, multiple cameras and
  game commands.
- [Scene API](../api/scene.md)
- [Game loop](../guides/game-loop.md)
