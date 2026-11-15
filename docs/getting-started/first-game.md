# Your First Game

This tutorial builds a small complete game: three colored quads, an orbiting
camera, keyboard input, a HUD and a custom console command. It uses only the
public `runtime` API.

## Create the project

```bash
cd soldatik3D
cargo build --release -p s3d
./target/release/s3d new first_game
cd first_game
```

The generated project has a desktop binary (`src/main.rs`) and a library
(`src/lib.rs`) that also exports `android_main` for Android.

## Write the game

Replace `src/lib.rs` with the following code:

```rust
//! First Soldatik3D game: three quads, a moving camera and a HUD.

use runtime::glam::Vec3;
use runtime::{
    run, Aabb, Camera, CameraRig, CommandContext, Game, GameConfig, InputMap, Material, Mesh,
    PrimaryLight, Scene, UpdateContext, Vertex, World,
};
use runtime::winit::keyboard::KeyCode;

#[cfg(target_os = "android")]
use runtime::run_android;

pub struct FirstGame {
    time: f32,
    height: f32,
    camera: Option<runtime::CameraId>,
}

impl Default for FirstGame {
    fn default() -> Self {
        Self {
            time: 0.0,
            height: 0.0,
            camera: None,
        }
    }
}

fn quad(center: Vec3, size: f32, color: [f32; 3]) -> Mesh {
    let half = size * 0.5;
    let positions = [
        [center.x - half, center.y - half, center.z],
        [center.x + half, center.y - half, center.z],
        [center.x + half, center.y + half, center.z],
        [center.x - half, center.y + half, center.z],
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut vertices = Vec::with_capacity(6);
    for index in [0, 1, 2, 0, 2, 3] {
        vertices.push(Vertex::new(positions[index], color, [0.0, 0.0, 1.0], uvs[index]));
    }
    Mesh::new(
        vertices,
        Aabb::new(
            center - Vec3::new(half, half, 0.01),
            center + Vec3::new(half, half, 0.01),
        ),
    )
}

impl Game for FirstGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("First Soldatik3D Game")
            .with_size(1280, 720)
    }

    fn create_world(&mut self) -> World {
        let scene = Scene::new(Material::default())
            .with_primary_light(PrimaryLight::default())
            .with_clear_color([0.03, 0.04, 0.06, 1.0])
            .with_meshes(vec![
                quad(Vec3::new(-1.4, 0.0, 0.0), 1.6, [0.9, 0.2, 0.2]),
                quad(Vec3::new(0.0, 0.0, -1.0), 1.6, [0.2, 0.8, 0.3]),
                quad(Vec3::new(1.4, 0.0, 0.0), 1.6, [0.2, 0.4, 0.9]),
            ]);

        let mut cameras = CameraRig::empty();
        let id = cameras.spawn("player", Camera::at(Vec3::new(0.0, 0.0, 5.0)));
        self.camera = Some(id);
        World::with_cameras(scene, cameras)
    }

    fn input_map(&self) -> InputMap {
        InputMap::new()
            .bind("up", &[KeyCode::KeyW, KeyCode::Space])
            .bind("down", &[KeyCode::KeyS, KeyCode::ControlLeft])
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        self.time += ctx.dt;

        if ctx.input.action_down("up") {
            self.height += ctx.dt * 2.0;
        }
        if ctx.input.action_down("down") {
            self.height -= ctx.dt * 2.0;
        }

        let position = Vec3::new(
            (self.time * 0.6).sin() * 2.0,
            self.height + (self.time * 1.3).sin() * 0.25,
            5.0,
        );
        if let Some(camera) = ctx.world.active_camera_mut() {
            camera.camera.position = position;
            let direction = (Vec3::ZERO - position).normalize();
            camera.camera.pitch = direction.y.asin();
            camera.camera.yaw = direction.z.atan2(direction.x);
        }

        ctx.ui.rect(16.0, 16.0, 260.0, 58.0, [0.0, 0.0, 0.0, 0.55]);
        ctx.ui.text("FIRST GAME", 28.0, 28.0, 1.3, [1.0, 1.0, 1.0, 1.0]);
        let stats = ctx.stats;
        ctx.ui.text(
            &format!(
                "{} tris  {} draws",
                stats.triangles, stats.draw_calls
            ),
            28.0,
            52.0,
            1.0,
            [0.7, 0.8, 0.9, 1.0],
        );
        ctx.ui.crosshair();
    }

    fn command_help(&self) -> &'static [(&'static str, &'static str)] {
        &[("respawn", "reset the camera animation")]
    }

    fn command(&mut self, ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
        match ctx.name {
            "respawn" => {
                self.time = 0.0;
                self.height = 0.0;
                if let (Some(id), Some(camera)) = (self.camera, ctx.world.active_camera_mut()) {
                    let _ = id;
                    camera.camera.position = Vec3::new(0.0, 0.0, 5.0);
                }
                Some(Ok("camera reset".to_owned()))
            }
            _ => None,
        }
    }
}

pub fn run_desktop() -> anyhow::Result<()> {
    run(FirstGame::default())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    if let Err(error) = run_android(FirstGame::default(), app) {
        eprintln!("game failed: {error:?}");
    }
}
```

`src/main.rs` only needs:

```rust
fn main() -> anyhow::Result<()> {
    first_game::run_desktop()
}
```

The package name is `first_game`, so the generated `main.rs` already calls
`first_game::run_desktop()`.

## What the code does

- `GameConfig` selects the window title and size.
- `create_world` builds a static `Scene` and a `CameraRig`, then returns a
  `World`. The scene is uploaded to the GPU once.
- `input_map` binds logical actions (`up`, `down`) to keys. `UpdateContext`
  exposes `input`, `dt`, `world`, `ui`, `stats` and `viewport`.
- `update` moves the camera and draws a HUD. The engine clears the UI draw list
  every frame and uploads it after `update`.
- `command_help` and `command` add a game command to the developer console.
  Open the console with `F1` or `` ` `` and type `respawn`.

## Run it

```bash
cargo run
```

Try:

```bash
SOLDATIK3D_BACKEND=opengl cargo run
SOLDATIK3D_CONSOLE=1 cargo run
SOLDATIK3D_COMMANDS="respawn" SOLDATIK3D_MAX_FRAMES=2 cargo run
```

## Build an APK

```bash
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
export ANDROID_SDK_ROOT=$HOME/Android/Sdk
./build_apk.sh arm64-v8a
adb install -r apk/app-arm64-v8a-debug.apk
```

## Next steps

- [Game loop](../guides/game-loop.md)
- [Scene API](../api/scene.md)
- [Input API](../api/input.md)
- [Console commands](../reference/cli-commands.md)
