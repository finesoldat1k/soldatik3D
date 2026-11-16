# Rendering API

Games do not create renderers or talk to Vulkan/OpenGL directly. The runtime
selects a backend, owns the `Renderer` and passes a `FrameData` value every
frame. This page documents the types a game may touch.

```rust
use runtime::{
    BackendKind, CameraFrame, DebugView, FrameData, RenderStats, Renderer,
    VsyncMode, WindowSurface,
};
```

## `BackendKind`

```rust
#[non_exhaustive]
pub enum BackendKind {
    Vulkan,
    OpenGl,
}
```

| Method | Description |
|--------|-------------|
| `name()` | `"vulkan"` or `"opengl"` |
| `from_name(&str)` | Parses `vulkan`/`vk`/`opengl`/`gl` |
| `from_env()` | Reads `SOLDATIK3D_BACKEND` |

`BackendKind::default()` is `Vulkan`. On Android the runtime overrides the
default to `OpenGl` because the Vulkan backend is not compiled for the target.

Selection order:

1. `SOLDATIK3D_BACKEND` if set to a known value.
2. `GameConfig::backend` (usually from `with_backend`).
3. Platform default: Vulkan on desktop, OpenGL ES on Android.

An unavailable backend causes renderer creation to fail. Use
`SOLDATIK3D_BACKEND=opengl` to isolate driver problems.

## `VsyncMode`

```rust
#[non_exhaustive]
pub enum VsyncMode {
    Off,
    On,        // default
    Adaptive,
    Mailbox,
}
```

| Mode | Behavior |
|------|----------|
| `Off` | Present immediately; may tear |
| `On` | Wait for vertical blank; caps at refresh rate |
| `Adaptive` | Wait for blank when on time, present immediately when late; falls back to `On` if unsupported |
| `Mailbox` | Triple buffering; Vulkan only, OpenGL falls back to `On` |

`VsyncMode::from_name` accepts `off`/`0`/`false`/`disabled`, `on`/`1`/`true`/
`enabled`, `adaptive`/`adapt`, `mailbox`/`triple`/`triple_buffer`.
`VsyncMode::from_env` reads `SOLDATIK3D_VSYNC`.

The demo default is `On`. Use `SOLDATIK3D_VSYNC=off SOLDATIK3D_MAX_FRAMES=...`
for benchmarks.

## `DebugView`

```rust
pub enum DebugView {
    Lit,      // default
    Normals,
    Depth,
}
```

Returned from `Game::debug_view` every frame. The demo switches it with F2/F3/F4
and the `SOLDATIK3D_DEBUG_VIEW` environment variable (`lit`, `normals`,
`depth`). It is a forward-pass visualization, not a separate render path.

```rust
fn debug_view(&self) -> DebugView {
    self.view
}
```

## `RenderStats`

```rust
pub struct RenderStats {
    pub vertices: u64,
    pub triangles: u64,
    pub draw_calls: u32,
}
```

`UpdateContext::stats` contains the counters of the previous submitted frame.
The values include shadow, SSAO, bloom and UI work performed by the backend, so
they describe GPU submission rather than gameplay mesh counts.

```rust
let fps = if ctx.dt > 0.0 { 1.0 / ctx.dt } else { 0.0 };
ctx.ui.text(
    &format!("{:.0} FPS  {} tris  {} draws", fps, ctx.stats.triangles, ctx.stats.draw_calls),
    20.0,
    20.0,
    1.0,
    [1.0; 4],
);
```

## `CameraFrame` and `FrameData`

These are backend-facing types. A game only reads `CameraFrame` if it writes
custom rendering code against the `Renderer` trait.

```rust
pub struct CameraFrame {
    pub view: Mat4,
    pub projection: Mat4,
    pub position: Vec3,
}

impl CameraFrame {
    pub fn view_projection(&self) -> Mat4;
}

pub struct FrameData<'a> {
    pub camera: CameraFrame,
    pub ui_vertices: &'a [UiVertex],
    pub ui_atlas: Option<UiAtlasFrame<'a>>,
    pub debug_view: DebugView,
}
```

`projection` already includes the current viewport aspect ratio.

`ui_atlas` is `Some` only when the UI rasterized new glyphs. It carries the
single-channel R8 coverage texture plus dirty rectangles, so backends can
skip the upload on most frames. `UiAtlasFrame` deliberately uses plain pixels
instead of depending on the `text` crate.

## `Renderer`

The trait every backend implements:

```rust
pub trait Renderer {
    fn backend(&self) -> BackendKind;
    fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<()>;
    fn render(&mut self, frame: &FrameData<'_>) -> anyhow::Result<()>;
    fn wait_idle(&self) -> anyhow::Result<()>;
    fn stats(&self) -> RenderStats;
    fn extent(&self) -> (u32, u32);
    fn screenshot(&mut self) -> anyhow::Result<()>;
    fn set_vsync(&mut self, vsync: VsyncMode) -> anyhow::Result<()>;
    fn vsync(&self) -> VsyncMode;
}
```

Games never call these methods: the runtime handles resize, render, idle and
screenshot. The trait exists so both backends are interchangeable and so new
backends can be added without changing game code.

`Renderer::screenshot` writes `screenshot_<unix_seconds>.png` into the current
working directory. Vulkan captures the swapchain image before present; OpenGL
reads the default framebuffer and flips it vertically. Both include the UI.

## `WindowSurface`

```rust
pub struct WindowSurface {
    pub window: RawWindowHandle,
    pub display: RawDisplayHandle,
    pub width: u32,
    pub height: u32,
}
```

A backend-neutral bundle of raw handles. `platform` creates it from the live
`winit` window; backend crates do not depend on `winit`.

## Rendering pipeline

Both backends render the same sequence:

1. **Shadow pass** — render meshes from the `PrimaryLight` into a 2048²
   orthographic depth map.
2. **Depth prepass** — Vulkan fills depth for the camera view; OpenGL uses the
   same geometry path.
3. **Forward PBR** — metallic/roughness shading with one primary directional
   light, up to eight nearest point lights, IBL from a procedural environment
   cubemap and shadow PCF.
4. **SSAO** — half-resolution screen-space ambient occlusion from depth.
5. **Bloom** — bright-pass extraction and separable Gaussian blur.
6. **Composite** — ACES tone mapping and gamma correction.
7. **UI pass** — `Ui` vertices with the R8 glyph atlas (solid geometry uses
   a reserved transparent UV, glyphs sample coverage and tint it).

Camera and shadow frustum culling skip meshes outside the view. The Vulkan HDR
target is `R16G16B16A16_SFLOAT`; OpenGL ES falls back to RGBA8 when
`EXT_color_buffer_half_float` is unavailable.

## Backend availability

| Target | Vulkan | OpenGL |
|--------|--------|--------|
| Linux x86_64 | yes | yes |
| Windows MSVC x86_64 | yes | yes |
| Windows GNU x86_64 | no | yes |
| Android | no | OpenGL ES 3.0 |

Check at runtime with `TargetOS` and `PlatformCapabilities`:

```rust
use runtime::TargetOS;

if TargetOS::current().supports_vulkan() {
    // select Vulkan
}
```

## Related

- [Runtime API](runtime.md)
- [Platform API](platform.md)
- [Architecture](../guides/architecture.md)
