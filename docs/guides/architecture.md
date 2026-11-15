# Architecture

Soldatik3D is split into small crates with a strict dependency direction. Games
see the `runtime` facade; graphics APIs are implementation details.

## Crates

```text
examples/demo ──► runtime ──► scene ──► assets
                     │           ▲
                     ├─► ui ─────┤
                     ├─► render  │
                     ├─► platform│
                     ├─► vulkan ─┘   (feature)
                     └─► opengl ──   (feature)
```

### `assets`

CPU-side data shared by all backends:

- `TextureData` (RGBA8 pixels, `white()`, `from_rgb8`)
- a 128×128 bitmap font atlas (`font_atlas_rgba8`)
- a procedural environment cubemap (`generate_environment_cubemap`)

No graphics API or windowing dependency.

### `scene`

Platform-independent world model:

- `World`: a `Scene` plus a `CameraRig`
- `Scene`: meshes, local lights, primary light, material, clear color
- `Camera`, `CameraId`, `CameraRig`, `NamedCamera`
- `Mesh`, `Vertex`, `Aabb`
- `Light`, `PrimaryLight`, `ShadowSettings`
- `Material` and the re-exported `glam` math types

The scene is CPU data. It contains no GPU handles.

### `ui`

`UiDrawList` converts screen-space rectangles and bitmap text into
backend-neutral `UiVertex` data. Coordinates are physical pixels with the origin
in the top-left corner. The runtime clears the list before `Game::update` and
uploads it after the game and console have drawn.

### `render`

The backend contract:

```rust
pub trait Renderer {
    fn backend(&self) -> BackendKind;
    fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<()>;
    fn render(&mut self, frame: &FrameData<'_>) -> anyhow::Result<()>;
    fn wait_idle(&self) -> anyhow::Result<()>;
    fn stats(&self) -> RenderStats;
    fn extent(&self) -> (u32, u32);
    fn screenshot(&mut self) -> anyhow::Result<()> { /* ... */ }
    fn set_vsync(&mut self, vsync: VsyncMode) -> anyhow::Result<()> { /* ... */ }
    fn vsync(&self) -> VsyncMode { VsyncMode::On }
}
```

`FrameData` contains only per-frame state: camera matrices, UI vertices and the
debug view. Everything static (meshes, lights, material, pipelines) is built
once when the renderer is created.

`WindowSurface` wraps `raw_window_handle` handles plus width/height. It is the
only bridge between `platform` and a backend; backend crates do not depend on
`winit`.

### `platform`

Owns `winit`:

- creates and owns the window,
- captures keyboard, mouse, text and cursor state into `Input`,
- exposes `WindowSurface` to the runtime,
- provides `TargetOS`, `TargetArch`, `PlatformCapabilities` and `WindowHints`.

`winit` is re-exported through `runtime::winit` so games can use key codes and
the Android activity type without adding a direct dependency.

### `vulkan` and `opengl`

Each crate implements `render::Renderer` and owns its GPU resources. Both
backends use the same scene and UI data but differ in API details:

- **Vulkan**: `ash`, HDR `R16G16B16A16_SFLOAT`, depth prepass, shadow pass,
  SSAO, bloom, ACES composite, UI pass, optional validation layers.
- **OpenGL**: `glutin` + `glow`, 3.3 core on desktop and ES 3.0 on Android,
  shadow pass, half-resolution SSAO and bloom, ACES composite, UI pass. On ES
  devices without `EXT_color_buffer_half_float`, HDR targets fall back to RGBA8.

Both backends perform camera and shadow frustum culling, upload at most eight
nearest local lights, and use a 2048×2048 shadow map.

### `runtime`

The public facade and engine loop:

- `Game`, `GameConfig`, `UpdateContext`, `CommandContext`
- event loop and window lifecycle
- renderer selection and creation
- world ownership
- input aggregation
- developer console and command registry
- UI submission and screenshots
- `run` for desktop, `run_android` for Android

`runtime` is the only crate that knows the concrete backend crates. Game code
never imports `vulkan` or `opengl`.

### `s3d`

Host tool for project creation, builds, APK packaging and exports. It is not a
dependency of a game; it invokes Cargo and the project's `build_apk.sh`.

## Frame flow

```text
winit event
   │
   ├─ keyboard/mouse → Input
   ├─ resize         → Renderer::resize
   └─ redraw
        │
        ├─ Input::begin_frame
        ├─ Game::update(UpdateContext { input, world, ui, stats, dt, ... })
        ├─ console update + submit
        ├─ UiDrawList → renderer
        ├─ FrameData { camera, ui_vertices, debug_view }
        └─ Renderer::render
```

The active camera is read from `World`. Switching cameras requires no GPU work.

## Static scene, dynamic camera

The renderer builds GPU buffers from `Scene` exactly once. Gameplay can freely
mutate the camera every frame. Mesh/material changes at runtime are not
re-uploaded; dynamic geometry is a planned backend extension.

## Android

On Android the game is a `cdylib` loaded by `android.app.NativeActivity`.
`android-activity` starts a dedicated thread and calls the exported
`android_main`. The game forwards the `AndroidApp` to `run_android`, which builds
a `winit` event loop with the Android backend. Only OpenGL ES 3.0 is compiled;
Vulkan dependencies are target-gated out.

## Threading

The engine runs on the platform main thread. Rendering, input and gameplay are
single-threaded. The renderer waits for GPU idle before shutdown. No engine type
is `Send`/`Sync` across the game/runtime boundary by design.

## Error handling

Fallible engine operations return `anyhow::Result`. The runtime logs window and
renderer failures and exits the event loop. A game's `run`/`run_desktop` return
`anyhow::Result<()>`; `android_main` cannot return an error, so it should log it.
