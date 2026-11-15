# Migration Guide

This guide covers the layout and API changes that occurred during the 0.1
development cycle. There has been no stable public release before 0.1.0, so
there are no long-term compatibility guarantees yet.

## Crate name: `soldatik3d` → `runtime`

Early versions exposed the engine as a single `soldatik3d` crate. The engine is
now a workspace and games depend on the `runtime` facade.

Before:

```toml
[dependencies]
soldatik3d = { path = "../soldatik3D" }
```

After:

```toml
[dependencies]
runtime = { path = "../soldatik3D/crates/runtime", default-features = false }

[features]
default = ["vulkan", "opengl"]
vulkan = ["runtime/vulkan"]
opengl = ["runtime/opengl"]
```

Update imports:

```rust
// before
use soldatik3d::{run, Game, World};
// after
use runtime::{run, Game, World};
```

`runtime::winit`, `runtime::glam` and the scene types are re-exported, so games
do not need direct dependencies on `winit` or `glam`.

## Project template path

The Android Gradle template used to live at `templates/android`. It was merged
into the self-contained game template:

| Before | After |
|--------|-------|
| `templates/android/gradle` | `templates/game/android/gradle` |
| `templates/game-project` | `templates/game` |
| root `cross_build.sh` | `scripts/cross_build.sh` |
| root `build_apk.sh` | `scripts/apk/build_apk.sh` (engine demo) and `<project>/build_apk.sh` (games) |

If you scripted the old paths, update them. For a game, always use the
`build_apk.sh` at the project root.

## Generated project layout

`s3d new` now produces a project with its own workspace and both entry points:

```text
my_game/
├── android/gradle/
├── assets/
├── shaders/
├── src/lib.rs       # game + android_main
├── src/main.rs      # desktop entry
├── build.rs
├── build_apk.sh
└── Cargo.toml
```

The `Cargo.toml` includes:

```toml
[workspace]

[lib]
crate-type = ["cdylib", "rlib"]
```

`[workspace]` keeps a project generated inside the engine repository
standalone. `cdylib` is required for Android; `rlib` lets `src/main.rs` reuse
the same game code.

If your project predates this layout, add both entries manually and move the
game implementation into `src/lib.rs`, exposing a `run_desktop` function.

## Game entry points

Desktop-only:

```rust
fn main() -> anyhow::Result<()> {
    runtime::run(MyGame::default())
}
```

Android support requires exporting `android_main` from the library:

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    if let Err(error) = runtime::run_android(MyGame::default(), app) {
        eprintln!("game failed: {error:?}");
    }
}
```

The Gradle manifest loads the library through the `${soldatik3DLibName}`
placeholder, which comes from `SOLDATIK3D_ANDROID_LIB_NAME`.

## Camera API

Early documentation showed `World::add_camera(name, Camera::new().with_position(...))`.
The current API uses a `CameraRig` and `Camera::at`:

```rust
// before (no longer exists)
world.add_camera("player", Camera::new().with_position(Vec3::ZERO));

// after
let id = world.spawn_camera("player", Camera::at(Vec3::ZERO));
world.select_camera(id);
```

Other renames:

| Before | After |
|--------|-------|
| `Camera::new()` | `Camera::default()` or `Camera::at(position)` |
| `with_position` / `with_target` | Set `camera.position`; yaw/pitch control direction |
| `with_fov_y` | `with_fov_degrees` |
| `world.camera` | `world.active_camera()` / `active_camera_ref()` |
| `world.cameras` | `world.cameras()` |

Camera movement is gameplay code. There is no built-in controller or
`with_direction`; use `forward()`, `forward_flat()`, `right_flat()` and modify
`position`, `yaw`, `pitch`.

## Light and mesh API

The engine has one material per scene and no primitive builders:

| Before (documented, never implemented) | Current |
|----------------------------------------|---------|
| `Mesh::cube()`, `Mesh::sphere()`, `Mesh::plane()` | Build `Vec<Vertex>` and call `Mesh::new(vertices, bounds)` |
| `Light::directional()/point()/spot()` | `Light::new(position, color, radius)`, one `PrimaryLight` |
| Multiple materials, texture slots | One `Material { albedo, metallic, roughness }` |
| `Renderer::render(&World)` | Runtime calls `Renderer::render(&FrameData)` |

See the [Scene API](../api/scene.md) for complete constructors.

## Input API

Gamepads and touch events are not implemented. Remove calls to
`input.gamepad`, `input.touches` or gesture helpers. Supported queries are
`key_down`, `key_pressed`, `action_down`, `action_pressed`, `mouse_delta`,
`scroll_delta`, `cursor_position`, `primary_clicked`, `cursor_captured` and
`text`.

Bindings are still declared with `InputMap`:

```rust
fn input_map(&self) -> InputMap {
    InputMap::new().bind("jump", &[KeyCode::Space])
}
```

## Console commands

The console no longer has `backend vulkan|opengl`, `debug.*` or `reload`
commands. Backend selection happens at startup through
`SOLDATIK3D_BACKEND`; debug views are implemented by the game and returned from
`Game::debug_view`. See [Console commands](cli-commands.md).

## CLI changes

The binary is `s3d`. Old spellings `soldatik3d` and `soldatik3d-cli` are not
installed.

| Before | After |
|--------|-------|
| `s3d build --target windows` | `s3d build windows` |
| `s3d apk --abi arm64-v8a` | `s3d apk arm64-v8a` |
| `s3d export --platform windows` | `s3d export --platforms windows` |
| `./cross_build.sh ...` | `./scripts/cross_build.sh ...` |
| `./build_apk.sh` at repo root | `./scripts/apk/build_apk.sh` |

`s3d export` writes raw binaries/libraries directly to `--out-dir`; it does not
create archives or APKs. `s3d apk` runs the project's `build_apk.sh`.

## CI changes

- CI builds a debug APK on every push and pull request.
- The release workflow builds release APKs on every GitHub Release, including
  prereleases, and attaches them directly.
- The Windows GNU build is OpenGL-only.
- Android uses OpenGL ES 3.0 only; Vulkan is target-gated out.

If you maintain a fork, update any workflow that references
`templates/android`, the root `cross_build.sh` or the old setup-android inputs.

## Documentation changes

The documentation was rewritten against the source. Types that only existed in
old docs — deferred rendering, cascaded shadow maps, wireframe debug view,
gamepad/touch input, audio, physics, scripting, mesh builders, hot reload and
`dist/*.tar.gz` exports — are not part of 0.1.0. Treat the source and the
current [API reference](../api/runtime.md) as authoritative.
