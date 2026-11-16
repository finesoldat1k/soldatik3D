# Architecture

This is the deeper design document. For a shorter overview, see
[Architecture Overview](guides/architecture.md).

## Goals

Soldatik3D is a small, readable 3D engine for Linux, Windows and Android. It
prioritizes:

1. **One public facade.** Games depend on `runtime`; backends are internal.
2. **Static scene, dynamic camera.** GPU resources are built once, cameras and
   UI are per-frame data.
3. **Two interchangeable backends.** Vulkan and OpenGL implement one trait.
4. **Explicit ownership.** No global state, no hidden threads.
5. **Portable tooling.** Local scripts and CI use the same Gradle project and
   the same build commands.

## Workspace

```text
examples/demo ──► runtime ──► scene ──► assets
                     │           ▲
                     ├─► ui ──► text
                     ├─► render  │
                     ├─► platform│
                     ├─► vulkan ─┘   (optional, desktop)
                     └─► opengl ──   (optional)
```

Dependency rules:

- `assets` depends on `glam` only.
- `scene` depends on `assets`, `glam`, `bytemuck`.
- `text` depends on `glam` only: SFNT/TTF parsing, rasterization, layout and
  the dynamic glyph atlas.
- `ui` depends on `text`, `glam`, `bytemuck`. It no longer depends on `assets`.
- `render` depends on `assets`, `scene`, `ui`, `glam`, `raw-window-handle`.
- `platform` depends on `winit`, `render`, `glam`.
- `vulkan` and `opengl` depend on `render`, `assets`, `scene`, `ui` and their
  graphics API crates. They do **not** depend on `winit`.
- `runtime` depends on all of the above and is the only crate that instantiates
  a concrete backend.
- `s3d` depends on no engine crate; it invokes Cargo and the project scripts.

`render::WindowSurface` is the boundary: `platform` converts a live `winit`
window into raw handles, and each backend creates its surface/context from
those handles. This keeps the backend crates testable without a windowing
system.

## Runtime ownership

```rust
pub(crate) struct App<G: Game> {
    game: G,
    config: GameConfig,
    renderer: Option<Box<dyn Renderer>>,
    window: Option<PlatformWindow>,
    world: Option<World>,
    input: Input,
    console: Console,
    ui: Ui,
    ...
}
```

Lifetime rules:

- `App` owns the game, window, renderer, world, input, console and `Ui`.
- The renderer is declared before the window in the struct and dropped before
  it, because the surface borrows the window's native handles.
- The game never receives a renderer reference. It can request a screenshot or
  VSync change only through console commands, which the runtime executes.
- The world is created after the renderer because the scene is uploaded during
  renderer creation.

## Startup sequence

1. `run(game)` creates the `winit` event loop and `App`.
2. `resumed` creates the window from `GameConfig`.
3. `WindowSurface` is built from raw handles.
4. The backend is chosen: `SOLDATIK3D_BACKEND`, then `GameConfig::backend`,
   then platform default.
5. The renderer is created and uploads the scene, lights, material and
   pipelines. The renderer starts with a 1×1 white UI texture; the real glyph
   atlas arrives with the first `FrameData`.
6. `game.create_world()` is called once. If it returns before the renderer
   exists, the scene is uploaded when the world is set.
7. `InputMap` from `game.input_map()` is applied once.
8. Startup commands from `SOLDATIK3D_COMMANDS` run.

On Android, `run_android(game, app)` builds the event loop from the
`AndroidApp` created by `android-activity`. Everything after step 2 is the same.

## Frame sequence

```text
window event
   │
   ├─ resize            → Renderer::resize
   ├─ keyboard/mouse    → Input
   └─ redraw requested
          │
          ├─ build UpdateContext { dt, input, ui, stats, viewport, world, console_open }
          ├─ Ui::begin_frame(input, viewport)
          ├─ Game::update (widget calls build the retained tree)
          ├─ execute pending console command (built-in → game)
          ├─ Ui::end_frame (layout + paint retained widgets)
          ├─ console draws raw text/rectangles on top
          ├─ renderer.render(FrameData { camera, ui_vertices, ui_atlas, debug_view })
          ├─ Ui::mark_atlas_uploaded after a successful render
          ├─ stats recorded
          └─ frame budget / exit checks
```

`dt` is clamped so a suspended window does not produce a huge simulation step.
`stats` describes the previous frame because the current frame has not been
submitted yet.

## Scene upload model

`Scene` is a CPU description:

- `meshes: Vec<Mesh>`, each a flat `Vec<Vertex>` in world space,
- `lights: Vec<Light>`,
- one `PrimaryLight`,
- one `Material` with one `TextureData`,
- a clear color.

Backends build immutable vertex/index buffers, a material texture, a light
buffer, shadow resources and post-processing targets from this data. There is
no per-object transform, no scene graph, no instancing and no runtime geometry
update. This keeps the backend code and the game contract simple.

Cameras are separate per-frame data. `World` owns a `CameraRig`; each frame the
runtime reads the active camera and builds `CameraFrame { view, projection,
position }` for the current aspect ratio.

## Backend abstraction

```rust
pub trait Renderer {
    fn backend(&self) -> BackendKind;
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    fn render(&mut self, frame: &FrameData<'_>) -> Result<()>;
    fn wait_idle(&self) -> Result<()>;
    fn stats(&self) -> RenderStats;
    fn extent(&self) -> (u32, u32);
    fn screenshot(&mut self) -> Result<()>;
    fn set_vsync(&mut self, vsync: VsyncMode) -> Result<()>;
    fn vsync(&self) -> VsyncMode;
}
```

`FrameData` is intentionally small: camera, UI vertices, an optional R8 glyph
atlas update and the debug view. Static resources live in the backend. This
avoids rebuilding pipelines or descriptor sets per frame and makes the
interface easy to implement for a new API.

### Vulkan

- `ash` for Vulkan 1.2.
- `shaderc` compiles GLSL to SPIR-V at startup.
- HDR target `R16G16B16A16_SFLOAT`.
- Depth prepass, shadow pass, SSAO, bloom, ACES composite, UI pass.
- Validation layers requested in debug builds or with
  `SOLDATIK3D_VALIDATION`.
- Screenshot captures the swapchain image before present.
- The crate is not compiled for Android.

### OpenGL

- `glutin` for context creation and `glow` for commands.
- OpenGL 3.3 core on desktop, OpenGL ES 3.0 on Android.
- Same pass structure, with half-resolution SSAO and bloom.
- Falls back to RGBA8 render targets when `EXT_color_buffer_half_float` is
  unavailable on ES.
- Supports `SOLDATIK3D_BACKEND=opengl` on Windows GNU builds, where Vulkan is
  unavailable.

Both backends use the same `scene` data and `ui` vertices, but differ in
projection conventions and resource binding. The Vulkan backend converts the
engine's OpenGL-style projection at its boundary.

## Culling and lights

- Camera frustum planes are extracted from `view_projection`; each mesh's
  `Aabb` is tested against them.
- The shadow pass uses the same test with the light's view-projection.
- The forward pass uploads the eight point lights nearest to the camera,
  selected by `LightUniformBuffer::from_nearest`.
- The single `PrimaryLight` casts shadows through an orthographic 2048² map with
  PCF filtering.

## UI and console

The UI is hybrid immediate/retained. Games call widgets every frame; the `Ui`
object owns an arena tree keyed by stable ids, so hover/press state, scroll
offsets, dropdown state and text caches survive between frames without
`Rc<RefCell<_>>`.

- `Ui::begin_frame` resets per-frame node flags and builds a pointer snapshot
  from `platform::Input`.
- Widget calls update the tree and return `Response { hovered, pressed,
  clicked, changed, drag_delta, .. }`.
- `Ui::end_frame` runs the flexbox-style layout, paints the tree and defers
  dropdown popups so they render above everything.
- The console is drawn after `end_frame` as raw text/rectangles, keeping it on
  top of game HUDs.
- `UiDrawList` converts rectangles and TrueType glyphs into
  `UiVertex { position, color, uv }` in NDC. Solid geometry uses UV `(0, 0)`;
  glyphs sample a single-channel R8 atlas.

The glyph atlas is owned by `text::TextSystem`. It starts at 512×512, grows by
doubling (up to 2048×4096), and tracks dirty rectangles. OpenGL recreates the
texture on growth and uploads only dirty regions afterwards; Vulkan recreates
the image and re-uploads the (small, single-channel) texture on updates. The
runtime marks the atlas as uploaded only after `Renderer::render` succeeds.

Pointer ownership is explicit: a widget (or joystick / touch camera) claims the
pointer that pressed it, and `Ui::captures_pointer` tells the game to suspend
camera look and world clicks for that frame.

The console is a separate state machine (`open`, `input`, `history`,
`completion`, `pending`). It does not execute commands inside the key handler:
it stores a pending line, and the runtime executes it on the next redraw where
the world and renderer are available.

## Input

`platform::Input` aggregates events into sets and deltas. `begin_frame` clears
one-frame state. `InputMap` is a plain `HashMap<String, Vec<KeyCode>>`; actions
are resolved at query time. Touch events are aggregated into `TouchState`
records and exposed through the same `Input` object; `platform::Input`
implements `ui::TouchInputProvider`. There is no event queue exposed to games,
no gamepad support and no runtime rebinding.

## Android

- The game crate is `crate-type = ["cdylib", "rlib"]`.
- `android-activity` with the `native-activity` feature starts a dedicated
  thread and calls the exported `android_main`.
- The library name in the manifest comes from
  `SOLDATIK3D_ANDROID_LIB_NAME`.
- Gradle packages `lib/<abi>/lib<name>.so`.
- `build.rs` copies `assets/` into the Gradle project before packaging.
- The Vulkan dependency is target-gated in `runtime` and its backend arm is
  compiled out; Android always uses OpenGL ES.

## Build and release architecture

- `scripts/apk/gradle` is the canonical engine demo Gradle project. CI copies
  it to a temporary directory, injects the native library and builds.
- `templates/game/android/gradle` is the copy shipped to users. It must stay
  identical to the canonical project.
- `scripts/apk/build_apk.sh` builds the engine demo APK.
- `<project>/build_apk.sh` builds a game APK.
- `scripts/cross_build.sh` builds relocatable local packages.
- `s3d` orchestrates Cargo and the project scripts; it does not duplicate the
  Gradle logic.
- The release workflow packages the CLI together with `templates/game`,
  `crates/`, `examples/demo` and the workspace manifest so `s3d new` can
  resolve both the template and the runtime path.

## Error handling

Fallible engine functions return `anyhow::Result`. The runtime logs renderer
errors and exits after repeated failures. Backend `Drop` implementations wait
for GPU idle where required. `android_main` cannot return an error, so it logs
and exits the thread.

## Extension points

- New graphics API: implement `render::Renderer`, add a `BackendKind` variant,
  select it in `runtime::backend`.
- New game command: implement `Game::command_help` and `Game::command`.
- New input action: add a binding in `Game::input_map`.
- New UI widget: add a module under `crates/ui/src/widgets/`, implement it as
  a method on `Ui`, and export its style from `widgets/mod.rs`.
- New font: call `TextSystem::load_font` at runtime, or replace the embedded
  Open Sans assets in `crates/text/assets/`.
- New platform: add `TargetOS` and `TargetArch` variants, a backend path, and
  build tooling. macOS/iOS/Web are not currently planned.

## Related

- [Architecture overview](guides/architecture.md)
- [Project structure](getting-started/project-structure.md)
- [Rendering API](api/rendering.md)
- [CI and releases](CI.md)
