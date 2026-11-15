# Runtime API

The `runtime` crate is the public facade of the engine. A game depends on
`runtime` only and never imports a backend crate.

```toml
[dependencies]
runtime = { path = "../soldatik3D/crates/runtime" }
anyhow = "1"
```

Every type mentioned below is re-exported at the crate root, for example:

```rust
use runtime::{
    run, Aabb, BackendKind, Camera, CameraId, CameraRig, CommandContext, DebugView, Game,
    GameConfig, Input, InputMap, Light, Material, Mesh, PrimaryLight, RenderStats, Scene,
    TextureData, UpdateContext, Vertex, VsyncMode, World,
};
```

## `run`

```rust
pub fn run<G: Game + 'static>(game: G) -> anyhow::Result<()>
```

Creates the platform event loop, window, renderer and console, then runs until
the window is closed or the frame budget is exhausted. On desktop this is the
only entry point a game needs.

```rust
fn main() -> anyhow::Result<()> {
    runtime::run(MyGame::default())
}
```

## `run_android`

```rust
#[cfg(target_os = "android")]
pub fn run_android<G: Game + 'static>(
    game: G,
    app: runtime::winit::platform::android::activity::AndroidApp,
) -> anyhow::Result<()>
```

Android equivalent of `run`. It uses the `AndroidApp` supplied by
`android-activity` to create the event loop. The game must export
`android_main` so `NativeActivity` can call it:

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    if let Err(error) = runtime::run_android(MyGame::default(), app) {
        eprintln!("game failed: {error:?}");
    }
}
```

## `Game`

```rust
pub trait Game {
    fn config(&self) -> GameConfig;
    fn create_world(&mut self) -> World;
    fn debug_view(&self) -> DebugView { DebugView::Lit }
    fn update(&mut self, ctx: &mut UpdateContext<'_>);
    fn input_map(&self) -> InputMap { InputMap::new() }
    fn command_help(&self) -> &'static [(&'static str, &'static str)] { &[] }
    fn command(&mut self, _ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
        None
    }
}
```

| Method | When it is called | Notes |
|--------|-------------------|-------|
| `config` | Once, when the window is created | Title, size, cursor lock, backend, VSync |
| `create_world` | Once, when the renderer is created | The scene is uploaded to the GPU and is immutable afterwards |
| `debug_view` | Every frame | `Lit`, `Normals` or `Depth` |
| `update` | Every frame | Gameplay, camera movement and UI |
| `input_map` | Once at startup | Applies named key bindings |
| `command_help` | On `help` | Game command descriptions |
| `command` | On an unknown console command | Return `Some(Ok)`, `Some(Err)` or `None` |

The trait has no `Send`/`Sync` requirement. All engine and game code runs on the
platform main thread.

## `GameConfig`

```rust
pub struct GameConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub cursor_locked: bool,
    pub backend: BackendKind,
    pub vsync: VsyncMode,
}
```

Default: `"Soldatik3D"`, 1280×720, cursor locked, platform default backend,
`VsyncMode::On`.

Builder methods:

| Method | Effect |
|--------|--------|
| `with_title(impl Into<String>)` | Window title |
| `with_size(width, height)` | Initial logical size |
| `with_cursor_locked(bool)` | Capture and hide the cursor on startup |
| `with_backend(BackendKind)` | Preferred backend |
| `with_vsync(VsyncMode)` | Swapchain pacing |

```rust
let config = GameConfig::default()
    .with_title("My Game")
    .with_size(1600, 900)
    .with_backend(BackendKind::OpenGl)
    .with_vsync(VsyncMode::On);
```

`SOLDATIK3D_BACKEND` overrides `backend` and `SOLDATIK3D_VSYNC` overrides
`vsync` at startup.

On Android the default backend is always OpenGL ES; Vulkan is not compiled for
the target.

## `UpdateContext`

```rust
pub struct UpdateContext<'a> {
    pub dt: f32,
    pub input: &'a Input,
    pub ui: &'a mut UiDrawList,
    pub stats: RenderStats,
    pub viewport: (u32, u32),
    pub world: &'a mut World,
    pub console_open: bool,
}
```

| Field | Meaning |
|-------|---------|
| `dt` | Seconds since the previous frame, clamped against long suspensions |
| `input` | Keyboard/mouse snapshot for this frame |
| `ui` | Immediate-mode draw list, cleared before every update and rendered after the console |
| `stats` | Counters of the **previous** rendered frame |
| `viewport` | Framebuffer size in physical pixels |
| `world` | Engine-owned world; move cameras, spawn and select them here |
| `console_open` | `true` while the console owns text input |

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    if ctx.console_open {
        return; // gameplay input is suspended while typing commands
    }
    ctx.ui.text("HELLO", 20.0, 20.0, 1.0, [1.0; 4]);
}
```

## `CommandContext`

```rust
pub struct CommandContext<'a> {
    pub name: &'a str,
    pub args: &'a [&'a str],
    pub world: &'a mut World,
}
```

Return value convention:

| Return | Console output |
|--------|----------------|
| `Some(Ok(message))` | Message printed normally |
| `Some(Err(message))` | Message printed as an error |
| `None` | Engine reports `unknown command` |

Built-in engine commands are handled before `Game::command` is called.

## `World`

`World` combines the renderable `Scene` with a `CameraRig`. The runtime renders
through the active camera.

```rust
impl World {
    pub fn new(scene: Scene) -> Self;                 // adds a default "main" camera
    pub fn with_cameras(scene: Scene, cameras: CameraRig) -> Self;
    pub fn scene(&self) -> &Scene;
    pub fn scene_mut(&mut self) -> &mut Scene;
    pub fn cameras(&self) -> &CameraRig;
    pub fn cameras_mut(&mut self) -> &mut CameraRig;

    pub fn spawn_camera(&mut self, name: impl Into<String>, camera: Camera) -> CameraId;
    pub fn remove_camera(&mut self, id: CameraId) -> Option<NamedCamera>;
    pub fn select_camera(&mut self, id: CameraId) -> bool;
    pub fn select_camera_name(&mut self, name: &str) -> bool;
    pub fn select_next_camera(&mut self) -> Option<CameraId>;
    pub fn select_previous_camera(&mut self) -> Option<CameraId>;

    pub fn active_camera(&self) -> Camera;
    pub fn active_camera_ref(&self) -> Option<&NamedCamera>;
    pub fn active_camera_mut(&mut self) -> Option<&mut NamedCamera>;
    pub fn active_camera_name(&self) -> &str;
    pub fn active_camera_id(&self) -> Option<CameraId>;
    pub fn view_projection(&self, aspect: f32) -> Mat4;
}
```

`World::default()` is an empty scene with a default `main` camera. The last
remaining camera cannot be removed.

```rust
fn create_world(&mut self) -> World {
    let mut scene = Scene::default();
    scene.clear_color = [0.02, 0.03, 0.05, 1.0];
    let mut world = World::new(scene);
    let overview = world.spawn_camera(
        "overview",
        Camera::at(Vec3::new(0.0, 12.0, 18.0))
            .with_pitch_degrees(-30.0)
            .with_fov_degrees(50.0),
    );
    world.select_camera(overview);
    world
}
```

## `run` return values and errors

`run` and `run_android` return `anyhow::Result<()>`. Window creation, surface
creation, renderer initialization and render errors are propagated. A runtime
render failure is counted; after repeated failures the event loop exits.

`android_main` cannot return an error, so log it as shown above.

## Environment variables

See [Configuration](../reference/configuration.md) for the full table. The
runtime reads:

| Variable | Effect |
|----------|--------|
| `SOLDATIK3D_BACKEND` | `vulkan` / `opengl` override |
| `SOLDATIK3D_VSYNC` | `on`, `off`, `adaptive`, `mailbox` |
| `SOLDATIK3D_MAX_FRAMES` | Exit after N frames |
| `SOLDATIK3D_COMMANDS` | Semicolon-separated startup commands |
| `SOLDATIK3D_CONSOLE` | Open the console on startup |
| `SOLDATIK3D_VALIDATION` | Ask Vulkan for validation layers |

## Related

- [Scene API](scene.md)
- [Input API](input.md)
- [UI API](ui.md)
- [Game loop guide](../guides/game-loop.md)
