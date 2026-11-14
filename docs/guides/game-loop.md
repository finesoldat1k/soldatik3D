# Game Loop

Understanding the Soldatik3D game loop and how to structure your game logic.

## The Game Trait

Your game implements the `Game` trait with three methods:

```rust
pub trait Game: 'static {
    /// Engine configuration
    fn config(&self) -> GameConfig;

    /// Called once at startup - create initial world state
    fn create_world(&mut self) -> World;

    /// Called every frame - main game logic
    fn update(&mut self, ctx: &mut UpdateContext<'_>);
}
```

## Frame Timeline

```
┌─────────────────────────────────────────────────────────────────────┐
│ FRAME N                                                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. EVENT PROCESSING                                                │
│     ┌─────────────────────────────────────────────────────────┐    │
│     │ winit EventLoop polls OS events                         │    │
│     │   → WindowEvent::Resized → App::resize()                │    │
│     │   → WindowEvent::KeyboardInput → Input::press/release  │    │
│     │   → WindowEvent::MouseInput → Input::press/release     │    │
│     │   → WindowEvent::CursorMoved → Input::mouse_move       │    │
│     │   → DeviceEvent::MouseMotion → Input::mouse_delta      │    │
│     │   → Event::AboutToWait → Request redraw                │    │
│     └─────────────────────────────────────────────────────────┘    │
│                              │                                      │
│                              ▼                                      │
│  2. GAME UPDATE (Your Code)                                         │
│     ┌─────────────────────────────────────────────────────────┐    │
│     │ Game::update(&mut UpdateContext)                        │    │
│     │   Input available: ctx.input                            │    │
│     │   World mutable: ctx.world                              │    │
│     │   Renderer control: ctx.renderer                        │    │
│     │   Console: ctx.console                                  │    │
│     │   Commands: ctx.commands                                │    │
│     │   Time: ctx.time, ctx.dt                                │    │
│     └─────────────────────────────────────────────────────────┘    │
│                              │                                      │
│                              ▼                                      │
│  3. RENDERING                                                       │
│     ┌─────────────────────────────────────────────────────────┐    │
│     │ Renderer::render(&FrameData)                            │    │
│     │   Shadow map pass                                       │    │
│     │   HDR scene pass                                        │    │
│     │   SSAO (half-res)                                       │    │
│     │   Bloom (half-res)                                      │    │
│     │   Composite + Tone mapping + Gamma                      │    │
│     │   UI overlay                                            │    │
│     │   Present / Swap                                        │    │
│     └─────────────────────────────────────────────────────────┘    │
│                              │                                      │
│                              ▼                                      │
│  4. FRAME END                                                       │
│     • Swap buffers                                              │    │
│     • Update timing (dt calculation)                            │    │
│     • Check exit conditions                                     │    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## UpdateContext - Your Frame Interface

```rust
pub struct UpdateContext<'a> {
    /// Game time in seconds since start
    pub time: f32,
    /// Delta time since last frame
    pub dt: f32,
    /// Unified input state
    pub input: &'a Input,
    /// Mutable world access
    pub world: &'a mut World,
    /// Renderer control
    pub renderer: &'a mut dyn Renderer,
    /// Console for output/commands
    pub console: &'a mut Console,
    /// Command registry
    pub commands: &'a CommandRegistry,
}
```

## Common Patterns

### Time-Based Movement

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let dt = ctx.dt; // Always use dt for frame-rate independent movement
    
    const SPEED: f32 = 5.0;
    if ctx.input.is_key_pressed(KeyCode::W) {
        player.pos.z -= SPEED * dt;
    }
}
```

### Fixed Timestep (Physics)

```rust
const PHYSICS_DT: f32 = 1.0 / 60.0;
const MAX_SUBSTEPS: u32 = 5;

fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    self.accumulator += ctx.dt;
    
    let mut steps = 0;
    while self.accumulator >= PHYSICS_DT && steps < MAX_SUBSTEPS {
        self.physics_step(PHYSICS_DT);
        self.accumulator -= PHYSICS_DT;
        steps += 1;
    }
    
    // Interpolate for rendering
    let alpha = self.accumulator / PHYSICS_DT;
    self.interpolate_render_state(alpha);
}
```

### Input Handling

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let input = ctx.input;
    
    // Continuous (held)
    if input.is_key_pressed(KeyCode::W) { ... }
    if input.is_mouse_button_pressed(MouseButton::Left) { ... }
    
    // Single-frame (just pressed/released)
    if input.is_key_just_pressed(KeyCode::Space) { jump(); }
    if input.is_key_just_released(KeyCode::E) { interact(); }
    
    // Mouse
    let mouse_pos = input.mouse_position();
    let mouse_delta = input.mouse_delta(); // For camera rotation
    
    // Gamepad
    if let Some(pad) = input.gamepad(0) {
        let stick = pad.axis(GamepadAxis::LeftStickX);
        if stick.abs() > 0.1 { move_x(stick); }
    }
}
```

### World Mutation

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Spawn entity
    let entity = ctx.world.spawn_mesh(mesh, transform, material);
    
    // Modify existing
    if let Some(entity) = ctx.world.entity_mut(entity_id) {
        entity.transform.translation += velocity * ctx.dt;
        entity.visible = true;
    }
    
    // Query
    for entity in ctx.world.entities_with::<Transform>() {
        // ...
    }
    
    // Camera
    if let Some(cam) = ctx.world.active_camera_mut() {
        cam.position = player_pos + Vec3::Y * 1.7;
    }
}
```

### Renderer Control

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let renderer = ctx.renderer;
    
    // Debug views
    renderer.set_debug_view(DebugView::Normals);
    renderer.set_debug_view(DebugView::Depth);
    renderer.set_debug_view(DebugView::Wireframe);
    renderer.set_debug_view(DebugView::Lit); // Default
    
    // VSync
    renderer.set_vsync(VsyncMode::On);
    renderer.set_vsync(VsyncMode::Off);
    renderer.set_vsync(VsyncMode::Adaptive);
    renderer.set_vsync(VsyncMode::Mailbox);
    
    // Screenshot
    renderer.screenshot(); // Deferred to next frame
    
    // Stats
    let stats = renderer.stats();
    println!("FPS: {:.1}", 1.0 / ctx.dt);
}
```

### Console & Commands

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Print to console
    ctx.console.print("Hello");
    ctx.console.error("Error!");
    ctx.console.warning("Warning");
    
    // Execute command programmatically
    ctx.commands.execute("screenshot");
    ctx.commands.execute("vsync.off");
    
    // Register custom command (usually in create_world or first update)
    ctx.commands.register(Command::new(
        "heal",
        "Restore health",
        |ctx| {
            // Access game state via context
            CommandResult::Ok(None)
        },
    ));
}
```

## Frame Timing

### Delta Time Calculation

```rust
// In App::run()
let mut last_frame = Instant::now();
loop {
    let now = Instant::now();
    let dt = (now - last_frame).as_secs_f32();
    last_frame = now;
    
    // Clamp to prevent spiral of death
    let dt = dt.min(0.1);
    
    ctx.dt = dt;
    ctx.time += dt;
}
```

### Frame Budget (Auto-Exit)

```bash
# Exit after N frames (for CI/benchmarking)
SOLDATIK3D_MAX_FRAMES=100 cargo run
```

```rust
// In App - checked after each frame
if let Some(max_frames) = self.frame_budget {
    if self.frames_rendered >= max_frames {
        self.exit_requested = true;
    }
}
```

## VSync Modes

| Mode | Behavior | Use Case |
|------|----------|----------|
| `On` (Default) | Wait for vblank | Normal gameplay |
| `Off` | Immediate present | Benchmarking, max FPS |
| `Adaptive` | VSync when >60fps, off when <60fps | Variable framerate |
| `Mailbox` | Triple buffering, latest frame | Low latency, no tearing |

```rust
// In Game::config
GameConfig::default().with_vsync(true) // Uses VsyncMode::On

// Or change at runtime
ctx.renderer.set_vsync(VsyncMode::Mailbox)?;
```

## Multi-Threading Considerations

### What Runs on Main Thread

- `Game::update()` - Your logic
- `Renderer::render()` - Draw calls
- `winit` event loop
- Console/UI rendering

### What Can Be Offloaded

- Asset loading (use `rayon` or `tokio`)
- Physics simulation (separate thread)
- Pathfinding/AI
- Audio mixing

```rust
// Example: Async asset loading
use std::sync::{Arc, Mutex};

fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    if self.loading {
        if let Ok(asset) = self.load_handle.try_recv() {
            self.assets.push(asset);
            self.loading = false;
        }
    } else if ctx.input.is_key_just_pressed(KeyCode::L) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.load_handle = rx;
        std::thread::spawn(move || {
            let asset = heavy_load();
            tx.send(asset).unwrap();
        });
        self.loading = true;
    }
}
```

## Best Practices

1. **Always use `dt`** for movement/animation
2. **Separate physics** from rendering with fixed timestep
3. **Minimize allocations** in `update()` - reuse buffers
4. **Batch world queries** - avoid repeated `entity_mut()` calls
5. **Use `just_pressed`** for one-time actions (jump, shoot)
6. **Profile with `SOLDATIK3D_MAX_FRAMES`** and `cargo flamegraph`
7. **Handle resize** - renderer does this automatically

---

*See [Runtime API](api/runtime.md) for complete `UpdateContext` documentation.*