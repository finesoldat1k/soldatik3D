# Advanced Example

A procedural scene with several meshes, two cameras, runtime camera switching,
debug views and game-defined console commands.

## Goal

- Build a small room from generated quads.
- Add a point light and a shadow-casting primary light.
- Spawn a static `overview` camera and a moving `orbit` camera.
- Switch cameras with `1` / `2`, cycle with `C`.
- Toggle lit/normals/depth with `F2`/`F3`/`F4`.
- Add `spawn_light` and `reset` console commands.

## Create the project

```bash
cargo build --release -p s3d
./target/release/s3d new advanced_game
cd advanced_game
```

## `src/lib.rs`

```rust
use runtime::glam::Vec3;
use runtime::winit::keyboard::KeyCode;
use runtime::{
    run, Aabb, Camera, CameraId, CameraRig, CommandContext, DebugView, Game,
    GameConfig, InputMap, Light, Material, Mesh, PrimaryLight, Scene,
    ShadowSettings, TextureData, UpdateContext, Vertex, World,
};

pub struct AdvancedGame {
    time: f32,
    overview: CameraId,
    orbit: CameraId,
    view: DebugView,
}

impl AdvancedGame {
    fn new() -> Self {
        Self {
            time: 0.0,
            overview: CameraId::from_raw(0),
            orbit: CameraId::from_raw(0),
            view: DebugView::Lit,
        }
    }

    fn set_view(&mut self, view: DebugView) -> String {
        self.view = view;
        match view {
            DebugView::Lit => "view: lit".to_owned(),
            DebugView::Normals => "view: normals".to_owned(),
            DebugView::Depth => "view: depth".to_owned(),
        }
    }
}

fn quad(
    center: Vec3,
    right: Vec3,
    up: Vec3,
    half_width: f32,
    half_height: f32,
    color: [f32; 3],
    normal: Vec3,
    uvs: [[f32; 2]; 4],
) -> Mesh {
    let corners = [
        center - right * half_width - up * half_height,
        center + right * half_width - up * half_height,
        center + right * half_width + up * half_height,
        center - right * half_width + up * half_height,
    ];
    let mut vertices = Vec::with_capacity(6);
    for index in [0, 1, 2, 0, 2, 3] {
        let position = corners[index];
        vertices.push(Vertex::new(
            [position.x, position.y, position.z],
            color,
            [normal.x, normal.y, normal.z],
            uvs[index],
        ));
    }
    Mesh::new(
        vertices,
        Aabb::new(
            corners[0].min(corners[2]),
            corners[0].max(corners[2]),
        ),
    )
}

fn build_room() -> Vec<Mesh> {
    let uv = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
    let mut meshes = Vec::new();

    // Floor and ceiling.
    meshes.push(quad(
        Vec3::new(0.0, -2.0, 0.0),
        Vec3::X,
        Vec3::Z,
        6.0,
        6.0,
        [0.45, 0.45, 0.5],
        Vec3::Y,
        uv,
    ));
    meshes.push(quad(
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::X,
        Vec3::Z,
        6.0,
        6.0,
        [0.75, 0.75, 0.8],
        -Vec3::Y,
        uv,
    ));

    // Walls. Normals point into the room.
    let wall_uv = [[0.0, 0.0], [3.0, 0.0], [3.0, 3.0], [0.0, 3.0]];
    for (center, right, normal, color) in [
        (Vec3::new(0.0, 0.0, -6.0), Vec3::X, Vec3::Z, [0.8, 0.5, 0.4]),
        (Vec3::new(0.0, 0.0, 6.0), -Vec3::X, -Vec3::Z, [0.4, 0.6, 0.8]),
        (Vec3::new(-6.0, 0.0, 0.0), Vec3::Z, Vec3::X, [0.6, 0.7, 0.4]),
        (Vec3::new(6.0, 0.0, 0.0), -Vec3::Z, -Vec3::X, [0.7, 0.4, 0.7]),
    ] {
        meshes.push(quad(
            center,
            right,
            Vec3::Y,
            6.0,
            2.0,
            color,
            normal,
            wall_uv,
        ));
    }

    // Three pillars.
    for offset in [-3.0_f32, 0.0, 3.0] {
        meshes.push(quad(
            Vec3::new(offset, 0.0, 0.0),
            Vec3::X,
            Vec3::Y,
            0.3,
            1.8,
            [0.9, 0.85, 0.7],
            -Vec3::Z,
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ));
    }

    meshes
}

impl Game for AdvancedGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("Advanced Soldatik3D Example")
            .with_size(1280, 720)
    }

    fn create_world(&mut self) -> World {
        let scene = Scene::new(Material {
                albedo: TextureData::white(),
                metallic: 0.0,
                roughness: 0.85,
            })
            .with_meshes(build_room())
            .with_lights(vec![
                Light::new(Vec3::new(-3.0, 1.0, -2.0), Vec3::new(1.0, 0.5, 0.3), 10.0)
                    .with_intensity(3.0),
                Light::new(Vec3::new(3.0, 1.0, 2.0), Vec3::new(0.3, 0.6, 1.0), 10.0)
                    .with_intensity(3.0),
            ])
            .with_primary_light(PrimaryLight {
                position: Vec3::new(4.0, 8.0, 6.0),
                target: Vec3::ZERO,
                up: Vec3::Y,
                color: Vec3::ONE,
                intensity: 1.2,
                shadow: ShadowSettings {
                    ortho_half_extent: 8.0,
                    near: 0.1,
                    far: 40.0,
                },
            })
            .with_clear_color([0.01, 0.012, 0.02, 1.0]);

        let mut cameras = CameraRig::empty();
        self.overview = cameras.spawn(
            "overview",
            Camera::at(Vec3::new(0.0, 7.0, 9.0))
                .with_pitch_degrees(-35.0)
                .with_fov_degrees(55.0),
        );
        self.orbit = cameras.spawn(
            "orbit",
            Camera::at(Vec3::new(0.0, 0.0, 5.0))
                .with_yaw_degrees(0.0)
                .with_pitch_degrees(0.0),
        );
        cameras.select(self.overview);
        World::with_cameras(scene, cameras)
    }

    fn input_map(&self) -> InputMap {
        InputMap::new().bind("cycle_camera", &[KeyCode::KeyC])
    }

    fn debug_view(&self) -> DebugView {
        self.view
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        self.time += ctx.dt;

        if ctx.input.key_pressed(KeyCode::Digit1) {
            ctx.world.select_camera_name("overview");
        }
        if ctx.input.key_pressed(KeyCode::Digit2) {
            ctx.world.select_camera_name("orbit");
        }
        if ctx.input.action_pressed("cycle_camera") {
            ctx.world.select_next_camera();
        }
        if ctx.input.key_pressed(KeyCode::F2) {
            self.view = DebugView::Lit;
        }
        if ctx.input.key_pressed(KeyCode::F3) {
            self.view = DebugView::Normals;
        }
        if ctx.input.key_pressed(KeyCode::F4) {
            self.view = DebugView::Depth;
        }

        let angle = self.time * 0.4;
        if let Some(orbit) = ctx.world.cameras_mut().get_mut(self.orbit) {
            orbit.camera.position = Vec3::new(angle.sin() * 5.0, 0.0, angle.cos() * 5.0);
            orbit.camera.yaw = (-orbit.camera.position.z)
                .atan2(-orbit.camera.position.x);
        }

        ctx.ui.rect(16.0, 16.0, 380.0, 96.0, [0.0, 0.0, 0.0, 0.6]);
        ctx.ui.text("ADVANCED EXAMPLE", 30.0, 28.0, 1.3, [1.0; 4]);
        ctx.ui.text(
            &format!(
                "CAMERA: {}   VIEW: {:?}",
                ctx.world.active_camera_name(),
                self.view
            ),
            30.0,
            56.0,
            1.0,
            [0.7, 0.85, 1.0, 1.0],
        );
        ctx.ui.text(
            "1/2 SELECT   C CYCLE   F2/F3/F4 VIEW",
            30.0,
            80.0,
            1.0,
            [0.6, 0.6, 0.65, 1.0],
        );
    }

    fn command_help(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("views", "list debug views"),
            ("look", "look <yaw> <pitch>"),
        ]
    }

    fn command(&mut self, ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
        match ctx.name {
            "views" => Some(Ok("lit, normals, depth".to_owned())),
            "look" => {
                if ctx.args.len() != 2 {
                    return Some(Err("usage: look <yaw> <pitch>".into()));
                }
                let yaw: f32 = match ctx.args[0].parse() {
                    Ok(value) => value,
                    Err(_) => return Some(Err("invalid yaw".into())),
                };
                let pitch: f32 = match ctx.args[1].parse() {
                    Ok(value) => value,
                    Err(_) => return Some(Err("invalid pitch".into())),
                };
                if let Some(camera) = ctx.world.active_camera_mut() {
                    camera.camera.yaw = yaw.to_radians();
                    camera.camera.pitch = pitch.to_radians();
                }
                Some(Ok(format!("look: yaw {yaw}, pitch {pitch}")))
            }
            _ => None,
        }
    }
}

pub fn run_desktop() -> anyhow::Result<()> {
    run(AdvancedGame::new())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    if let Err(error) = runtime::run_android(AdvancedGame::new(), app) {
        eprintln!("game failed: {error:?}");
    }
}
```

## Try it

```bash
cargo run
```

In the console (`F1`):

```text
help
camera.list
camera.select orbit
camera.info
look -90 -10
```

Camera switching and debug views do not upload anything to the GPU. Only the
camera matrices and the `DebugView` enum value change.

## Key points

### Camera IDs

`CameraId` is stable across selection and removal. Store IDs for cameras your
game controls and `select_name` for user-facing commands. `CameraId::from_raw`
is only used here to initialize fields before `create_world` runs; a real game
can use `Option<CameraId>` instead.

### `CameraRig::get_mut`

The orbit camera is moved every frame even while it is not active:

```rust
if let Some(orbit) = ctx.world.cameras_mut().get_mut(self.orbit) {
    orbit.camera.position = ...;
}
```

`active_camera_mut()` is for the camera the player controls. Both are cheap
lookups.

### Procedural geometry

Every mesh is a flat vertex list in world space. The helper builds two triangles
per quad, computes corners from a center and two axes, and derives bounds from
the corners. If you generate arbitrary triangles, compute the minimum and
maximum over all vertices for `Aabb`.

### Debug views

`Game::debug_view` is queried every frame. There is no `debug.*` console command
in the engine; a game implements its own, as done here.

### Console commands

`views` and `look` are advertised through `command_help` and handled in
`command`. Built-in commands are handled first, so do not reuse their names.

## Build an APK

```bash
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
export ANDROID_SDK_ROOT=$HOME/Android/Sdk
./build_apk.sh arm64-v8a
```

## Next steps

- [Platform-specific example](platform-specific.md)
- [Input API](../api/input.md)
- [Console commands](../reference/cli-commands.md)
