# Input API

Input is captured by the `platform` crate and exposed as an immutable snapshot
through `UpdateContext::input`. Games cannot push input events; they query the
snapshot and update their own state.

```rust
use runtime::{Input, InputMap};
use runtime::winit::keyboard::KeyCode;
```

## `Input`

```rust
impl Input {
    pub fn key_down(&self, key: KeyCode) -> bool;
    pub fn key_pressed(&self, key: KeyCode) -> bool;
    pub fn action_down(&self, action: &str) -> bool;
    pub fn action_pressed(&self, action: &str) -> bool;
    pub fn mouse_delta(&self) -> Vec2;
    pub fn scroll_delta(&self) -> f32;
    pub fn cursor_position(&self) -> Vec2;
    pub fn primary_clicked(&self) -> bool;
    pub fn cursor_captured(&self) -> bool;
    pub fn text(&self) -> &str;
}
```

| Method | Meaning | Lifetime |
|--------|---------|----------|
| `key_down` | Key is currently held | Persistent |
| `key_pressed` | Key transitioned to pressed this frame | One frame |
| `action_down` | Any key bound to the action is held | Persistent |
| `action_pressed` | Any bound key was pressed this frame | One frame |
| `mouse_delta` | Raw motion since the previous frame | One frame |
| `scroll_delta` | Wheel ticks since the previous frame | One frame |
| `cursor_position` | Cursor in physical pixels from the top-left | Persistent |
| `primary_clicked` | Left button pressed this frame | One frame |
| `cursor_captured` | Cursor is hidden and locked | Persistent |
| `text` | Typed characters this frame | One frame |

The engine resets `pressed`, `mouse_delta`, `scroll_delta`, `primary_clicked`
and `text` after `Game::update`.

`mouse_delta` is zero unless the cursor is captured. When capture is requested
the engine ignores the first two frames of synthetic motion so the camera does
not snap.

There is **no gamepad and no touch event API**. Android input currently uses
the platform keyboard/mouse events that the OS provides; on-screen touch
controls are not implemented.

## `InputMap`

```rust
pub struct InputMap { /* private */ }

impl InputMap {
    pub fn new() -> Self;
    pub fn bind(self, action: &str, keys: &[KeyCode]) -> Self;
    pub fn keys_for(&self, action: &str) -> Option<&[KeyCode]>;
}
```

A map is a set of named actions, each bound to one or more keys. It is applied
once when the game starts. There is no runtime rebinding UI; create a new map
and restart to change bindings.

```rust
fn input_map(&self) -> InputMap {
    InputMap::new()
        .bind("forward", &[KeyCode::KeyW, KeyCode::ArrowUp])
        .bind("back", &[KeyCode::KeyS, KeyCode::ArrowDown])
        .bind("left", &[KeyCode::KeyA, KeyCode::ArrowLeft])
        .bind("right", &[KeyCode::KeyD, KeyCode::ArrowRight])
        .bind("jump", &[KeyCode::Space])
        .bind("sprint", &[KeyCode::ShiftLeft, KeyCode::ShiftRight])
        .bind("use", &[KeyCode::KeyE])
}
```

Querying an unbound action returns `false`, so typos fail silently. Keep action
names in constants when a project grows.

## Movement example

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    if ctx.console_open {
        return;
    }

    let mut direction = Vec3::ZERO;
    if ctx.input.action_down("forward") {
        direction += Vec3::Z;
    }
    if ctx.input.action_down("back") {
        direction -= Vec3::Z;
    }
    if ctx.input.action_down("left") {
        direction += Vec3::X;
    }
    if ctx.input.action_down("right") {
        direction -= Vec3::X;
    }
    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
    }

    let speed = if ctx.input.action_down("sprint") { 9.0 } else { 4.0 };
    if let Some(camera) = ctx.world.active_camera_mut() {
        let flat = camera.camera.forward_flat();
        let right = camera.camera.right_flat();
        camera.camera.position += (flat * direction.z + right * direction.x)
            * speed
            * ctx.dt;
    }
}
```

## Mouse look

The engine captures the cursor when the window opens if
`GameConfig::cursor_locked` is true. The demo toggles capture with `Tab`.

```rust
const SENSITIVITY: f32 = 0.0025;

fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let delta = ctx.input.mouse_delta();
    if delta != Vec2::ZERO
        && let Some(camera) = ctx.world.active_camera_mut()
    {
        camera.camera.yaw += delta.x * SENSITIVITY;
        camera.camera.pitch = (camera.camera.pitch - delta.y * SENSITIVITY)
            .clamp(-1.54, 1.54);
    }
}
```

`cursor_position` is useful for mouse-driven UI while the cursor is released.
The demo's debug buttons use it together with `primary_clicked`.

## Keys and text

`KeyCode` comes from `winit` and is re-exported through `runtime::winit`:

```rust
use runtime::winit::keyboard::KeyCode;

if ctx.input.key_pressed(KeyCode::F5) {
    self.on_debug_key();
}
```

Use `KeyCode` for physical keys (layout-independent) and `Input::text` for
typed characters. The engine console consumes `text` while it is open; gameplay
should usually ignore it.

## Reserved keys

The runtime handles these before gameplay sees them:

| Key | Engine action |
|-----|---------------|
| `F1` / `` ` `` | Toggle console |
| `Esc` | Close console, release cursor, then exit |
| `Tab` | Toggle cursor capture (or complete in the console) |
| `F12` | Save a screenshot |

Games can still query them, but the engine action always happens.

## Related

- [Runtime API](runtime.md)
- [UI API](ui.md)
- [Configuration](../reference/configuration.md)
