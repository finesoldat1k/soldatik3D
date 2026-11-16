# Game Loop

The runtime owns the event loop, window, renderer and console. A game provides
three required pieces of behavior through the `Game` trait:

```rust
use runtime::{Game, GameConfig, UpdateContext, World};

pub trait Game {
    fn config(&self) -> GameConfig;
    fn create_world(&mut self) -> World;
    fn update(&mut self, ctx: &mut UpdateContext<'_>);
}
```

Optional methods:

| Method | Purpose |
|--------|---------|
| `debug_view(&self) -> DebugView` | Lit, normals or depth visualization |
| `input_map(&self) -> InputMap` | Named keyboard action bindings |
| `command_help(&self) -> &'static [(&str, &str)]` | Commands shown by `help` |
| `command(&mut self, ctx: &mut CommandContext<'_>)` | Handle game console commands |

## Lifecycle

1. `runtime::run(game)` creates a `winit` event loop and the engine `App`.
2. On `resumed`, the engine creates the window, builds `WindowSurface`, asks the
   game for `GameConfig`, and creates the renderer.
3. `game.create_world()` is called once. The `Scene` part of the world is
   uploaded to GPU resources and remains immutable for the renderer lifetime.
4. Every frame:
   - input events are collected,
   - `UpdateContext` is constructed,
   - `game.update(ctx)` runs,
   - the console consumes text input if it is open,
   - the UI is laid out/painted, its glyph atlas is uploaded if needed, and the frame is rendered,
   - frame statistics are recorded.
5. On window close, the renderer is idled and dropped before the window.

On Android, `run_android(game, android_app)` does the same, but builds the event
loop with the `AndroidApp` supplied by `android-activity`.

## UpdateContext

```rust
pub struct UpdateContext<'a> {
    pub dt: f32,
    pub input: &'a Input,
    pub ui: &'a mut Ui,
    pub stats: RenderStats,
    pub viewport: (u32, u32),
    pub world: &'a mut World,
    pub console_open: bool,
}
```

- `dt` is the time since the previous frame in seconds, clamped to avoid huge
  steps after suspend.
- `input` is a snapshot for this frame. Held keys are persistent; `pressed`
  states, mouse delta, scroll and clicks reset at the start of the next frame.
- `ui` is begun before `update`, laid out and painted after the game has
  built its widgets, and its glyph atlas is uploaded with the frame.
- `stats` describes the **previous** rendered frame.
- `viewport` is the framebuffer size in physical pixels.
- `console_open` lets gameplay suspend controls while the console is open.

## Scene ownership

`World` combines a `Scene` with a `CameraRig`:

```rust
fn create_world(&mut self) -> World {
    let scene = Scene::new(Material::default())
        .with_meshes(load_level())
        .with_lights(load_lights())
        .with_primary_light(PrimaryLight::default());

    let mut cameras = CameraRig::empty();
    cameras.spawn("player", Camera::at(Vec3::new(0.0, 2.0, 8.0)));
    World::with_cameras(scene, cameras)
}
```

The renderer uploads meshes and the material once. The camera is per-frame data,
so you can spawn, select, move and remove cameras without rebuilding GPU
resources. `ctx.world.scene_mut()` allows gameplay to read or modify scene data,
but geometry changes are not re-uploaded automatically; rebuild the world only
by recreating the game/renderer.

## A frame in practice

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    self.time += ctx.dt;

    if ctx.input.action_down("sprint") {
        self.speed = 8.0;
    } else {
        self.speed = 4.0;
    }

    let movement = ctx.input.action_down("forward") as i32 as f32
        - ctx.input.action_down("backward") as i32 as f32;
    if let Some(camera) = ctx.world.active_camera_mut() {
        camera.camera.position += camera.camera.forward_flat() * movement * self.speed * ctx.dt;
    }

    ctx.ui.text("F1: console", 20.0, 20.0, 1.2, [1.0; 4]);
}
```

Camera movement is gameplay code. `Camera` only provides `forward`,
`forward_flat`, `right_flat`, view/projection matrices and the yaw/pitch fields.

## Input timing

| Method | Meaning |
|--------|---------|
| `key_down(code)` | Key is currently held |
| `key_pressed(code)` | Key transitioned to pressed during this frame |
| `action_down(name)` | Any key bound to the action is held |
| `action_pressed(name)` | Any bound key was pressed this frame |
| `mouse_delta()` | Raw motion while the cursor is captured |
| `scroll_delta()` | Scroll ticks this frame |
| `primary_clicked()` | Left click this frame |

`InputMap` is applied once at startup. Rebindable controls are not implemented;
rebuild the game to change bindings.

## Console integration

Pressing `F1` or `` ` `` toggles the console. While it is open, text input is
routed to the console, the cursor is released, and `ctx.console_open` is true.
Built-in commands are handled by the engine; unknown commands are passed to
`Game::command`.

```rust
fn command_help(&self) -> &'static [(&'static str, &'static str)] {
    &[("spawn_enemy", "spawn_enemy <x> <z>")]
}

fn command(&mut self, ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
    if ctx.name != "spawn_enemy" {
        return None;
    }
    if ctx.args.len() != 2 {
        return Some(Err("usage: spawn_enemy <x> <z>".into()));
    }
    let x: f32 = match ctx.args[0].parse() {
        Ok(value) => value,
        Err(_) => return Some(Err("invalid x".into())),
    };
    let z: f32 = match ctx.args[1].parse() {
        Ok(value) => value,
        Err(_) => return Some(Err("invalid z".into())),
    };
    self.enemies.push([x, z]);
    Some(Ok(format!("spawned enemy at {x}, {z}")))
}
```

## Frame budget and shutdown

`SOLDATIK3D_MAX_FRAMES=N` exits after N rendered frames. It is intended for
smoke tests and benchmarks:

```bash
SOLDATIK3D_VSYNC=off SOLDATIK3D_MAX_FRAMES=100 cargo run --release -p demo
```

`Esc` closes the console, then releases the cursor, then exits the application.
`Tab` toggles cursor capture.

## Vulkan validation

Desktop Vulkan builds can enable validation layers:

```bash
SOLDATIK3D_VALIDATION=1 cargo run -p demo
```

Validation is a Vulkan feature; the OpenGL backend ignores it.
