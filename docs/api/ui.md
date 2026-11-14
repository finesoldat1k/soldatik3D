# UI & Console API Reference

Complete API documentation for the immediate-mode UI system and developer console.

## Console

The in-game developer console provides command execution, logging, and debugging.

### Console Struct

```rust
pub struct Console {
    visible: bool,
    history: Vec<ConsoleEntry>,
    input_buffer: String,
    cursor_position: usize,
    scroll_offset: usize,
    max_history: usize,
}

pub struct ConsoleEntry {
    pub timestamp: f32,
    pub level: LogLevel,
    pub message: String,
}

pub enum LogLevel {
    Info,
    Warning,
    Error,
    Command,
    Result,
}
```

### Methods

```rust
impl Console {
    /// Print info message (white)
    pub fn print(&mut self, msg: impl Into<String>);
    
    /// Print warning (yellow)
    pub fn warning(&mut self, msg: impl Into<String>);
    
    /// Print error (red)
    pub fn error(&mut self, msg: impl Into<String>);
    
    /// Print command execution (cyan)
    pub fn command(&mut self, msg: impl Into<String>);
    
    /// Print result (green)
    pub fn result(&mut self, msg: impl Into<String>);
    
    /// Clear all history
    pub fn clear(&mut self);
    
    /// Get history entries
    pub fn history(&self) -> &[ConsoleEntry];
    
    /// Set visibility
    pub fn set_visible(&mut self, visible: bool);
    
    /// Get visibility
    pub fn is_visible(&self) -> bool;
    
    /// Toggle visibility
    pub fn toggle(&mut self);
    
    /// Set max history entries
    pub fn set_max_history(&mut self, max: usize);
}
```

### Usage from UpdateContext

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Simple messages
    ctx.console.print("Game started");
    ctx.console.warning("Low health!");
    ctx.console.error("Failed to load asset");
    
    // Formatted output
    ctx.console.print(format!("Score: {}", self.score));
    ctx.console.print(format!("Position: {:?}", player_pos));
    
    // Toggle with key
    if ctx.input.is_key_just_pressed(KeyCode::Backquote) { // `
        ctx.console.toggle();
    }
}
```

### Console Rendering

The console renders automatically when visible:
- Semi-transparent background
- Command history with timestamps
- Input line with cursor
- Tab completion
- Up/down arrow history navigation

---

## Command System

Commands allow runtime interaction and debugging.

### Command Registry

```rust
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    /// Register a new command
    pub fn register(&mut self, command: Command);
    
    /// Get command by name
    pub fn get(&self, name: &str) -> Option<&Command>;
    
    /// Execute command by name with args
    pub fn execute(&mut self, name: &str, args: &[&str]) -> CommandResult;
    
    /// List all command names
    pub fn list(&self) -> Vec<&str>;
    
    /// Remove command
    pub fn remove(&mut self, name: &str) -> bool;
}
```

### Command Struct

```rust
pub struct Command {
    pub name: String,
    pub description: String,
    pub callback: Box<dyn Fn(&mut CommandContext<'_>) -> CommandResult + Send + Sync>,
}

impl Command {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        callback: impl Fn(&mut CommandContext<'_>) -> CommandResult + Send + Sync + 'static,
    ) -> Self;
}
```

### CommandContext

```rust
pub struct CommandContext<'a> {
    pub world: &'a mut World,
    pub renderer: &'a mut dyn Renderer,
    pub console: &'a mut Console,
    pub commands: &'a CommandRegistry,
    pub args: &'a [&'a str],
}
```

### CommandResult

```rust
pub enum CommandResult {
    Ok(Option<String>),   // Success with optional output
    Err(String),          // Error message
}
```

### Built-in Commands

| Command | Description |
|---------|-------------|
| `help` | List all commands |
| `clear` | Clear console history |
| `quit` / `exit` | Exit game |
| `backend` | Show current backend |
| `backend vulkan\|opengl` | Switch backend |
| `vsync.on\|off\|adaptive\|mailbox` | Set VSync mode |
| `vsync.toggle` | Toggle VSync |
| `vsync.current` | Show current VSync |
| `screenshot` | Save PNG |
| `camera.list` | List cameras |
| `camera.select <name>` | Switch camera |
| `camera.spawn <name> [x y z] [yaw] [pitch]` | Create camera |
| `camera.remove <name>` | Delete camera |
| `debug.lit` | Normal rendering |
| `debug.normals` | Show normals |
| `debug.depth` | Show depth |
| `debug.wireframe` | Wireframe mode |

### Registering Custom Commands

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Simple command
    ctx.commands.register(Command::new(
        "hello",
        "Print greeting",
        |ctx| {
            ctx.console.print("Hello, world!");
            CommandResult::Ok(None)
        },
    ));
    
    // Command with arguments
    ctx.commands.register(Command::new(
        "teleport",
        "Teleport to coordinates: teleport <x> <y> <z>",
        |ctx| {
            let args = ctx.args;
            if args.len() != 3 {
                return CommandResult::Err("Usage: teleport <x> <y> <z>".into());
            }
            
            let x = args[0].parse::<f32>().map_err(|_| "Invalid X")?;
            let y = args[1].parse::<f32>().map_err(|_| "Invalid Y")?;
            let z = args[2].parse::<f32>().map_err(|_| "Invalid Z")?;
            
            if let Some(player) = ctx.world.entity_mut(PLAYER_ID) {
                player.transform.translation = Vec3::new(x, y, z);
            }
            
            CommandResult::Ok(Some(format!("Teleported to ({}, {}, {})", x, y, z)))
        },
    ));
    
    // Command with optional args
    ctx.commands.register(Command::new(
        "spawn",
        "Spawn entity: spawn [type] [x] [y] [z]",
        |ctx| {
            let args = ctx.args;
            let entity_type = args.get(0).unwrap_or(&"cube");
            let pos = Vec3::new(
                args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5.0),
                args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            );
            
            let mesh = match *entity_type {
                "cube" => Mesh::cube(1.0),
                "sphere" => Mesh::sphere(1.0, 16),
                _ => Mesh::cube(1.0),
            };
            
            ctx.world.spawn_mesh(mesh, Transform::from_translation(pos), Material::default());
            
            CommandResult::Ok(Some(format!("Spawned {} at {:?}", entity_type, pos)))
        },
    ));
}
```

### Executing Commands Programmatically

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Execute single command
    ctx.commands.execute("screenshot");
    ctx.commands.execute("vsync.off");
    
    // Execute with args
    ctx.commands.execute("teleport", &["10.0", "5.0", "0.0"]);
    
    // Run multiple from string (startup)
    // Set via SOLDATIK3D_COMMANDS="cmd1;cmd2"
    // Or programmatically:
    for cmd in ["vsync.off", "debug.normals", "camera.spawn top 0 10 0"] {
        ctx.commands.execute(cmd);
    }
}
```

### Startup Commands

```bash
# Via environment variable
SOLDATIK3D_COMMANDS="vsync.off;debug.wireframe;camera.spawn overview 0 10 0" cargo run

# Or in GameConfig
GameConfig::default()
    .with_startup_commands(vec![
        "vsync.off".into(),
        "debug.normals".into(),
    ])
```

---

## Immediate-Mode UI (UiDrawList)

Low-level UI rendering for custom interfaces.

### UiDrawList

```rust
pub struct UiDrawList {
    vertices: Vec<UiVertex>,
    texture: Option<TextureHandle>,
}

impl UiDrawList {
    pub fn new() -> Self;
    
    /// Set texture for subsequent draws
    pub fn set_texture(&mut self, texture: Option<TextureHandle>);
    
    /// Draw text
    pub fn text(&mut self, text: &str, pos: Vec2, size: f32, color: Color);
    
    /// Draw rectangle
    pub fn rect(&mut self, pos: Vec2, size: Vec2, color: Color);
    
    /// Draw rectangle outline
    pub fn rect_outline(&mut self, pos: Vec2, size: Vec2, thickness: f32, color: Color);
    
    /// Draw line
    pub fn line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color);
    
    /// Draw circle
    pub fn circle(&mut self, center: Vec2, radius: f32, color: Color);
    
    /// Draw circle outline
    pub fn circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: Color);
    
    /// Draw triangle
    pub fn triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color);
    
    /// Draw image
    pub fn image(&mut self, texture: TextureHandle, pos: Vec2, size: Vec2, uv: Rect, color: Color);
    
    /// Push scissor rect
    pub fn push_scissor(&mut self, rect: Rect);
    pub fn pop_scissor(&mut self);
    
    /// Get vertices for rendering
    pub fn vertices(&self) -> &[UiVertex];
    
    /// Clear all commands
    pub fn clear(&mut self);
}
```

### UiVertex

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct UiVertex {
    pub position: Vec2,      // Screen pixels
    pub uv: Vec2,            // Texture coordinates
    pub color: Color,        // Vertex color
}
```

### Usage from UpdateContext

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let ui = ctx.renderer.ui();
    
    // HUD
    ui.text(&format!("FPS: {:.0}", 1.0 / ctx.dt), Vec2::new(10.0, 10.0), 24.0, Color::WHITE);
    ui.text(&format!("Score: {}", self.score), Vec2::new(10.0, 40.0), 24.0, Color::YELLOW);
    ui.text(&format!("Health: {}", self.health), Vec2::new(10.0, 70.0), 24.0, Color::RED);
    
    // Health bar
    let bar_width = 200.0 * (self.health as f32 / self.max_health as f32);
    ui.rect(Vec2::new(10.0, 100.0), Vec2::new(200.0, 20.0), Color::rgba(0.0, 0.0, 0.0, 0.5));
    ui.rect(Vec2::new(10.0, 100.0), Vec2::new(bar_width, 20.0), Color::GREEN);
    
    // Crosshair
    let center = Vec2::new(ctx.renderer.extent().0 as f32 / 2.0, ctx.renderer.extent().1 as f32 / 2.0);
    ui.line(Vec2::new(center.x - 10.0, center.y), Vec2::new(center.x + 10.0, center.y), 2.0, Color::WHITE);
    ui.line(Vec2::new(center.x, center.y - 10.0), Vec2::new(center.x, center.y + 10.0), 2.0, Color::WHITE);
}
```

---

## Font System

```rust
pub fn font_atlas_rgba8() -> Vec<u8> {
    // Returns 128x128 RGBA8 bitmap font atlas
    // Contains ASCII 32-126 (95 chars) in 16x6 grid
    // Each glyph: 8x16 pixels with 1px padding
}

pub fn generate_environment_cubemap(size: u32) -> Vec<f32> {
    // Procedural sky cubemap (RGBA32F)
    // 6 faces, size x size each
}
```

### Custom Font Loading

```rust
// Not directly supported in core API
// Use external crate (ab_glyph, fontdue) to rasterize
// Then create texture via Renderer::create_texture
```

---

## Debug Overlay

Built-in debug information overlay.

### DebugView

```rust
pub enum DebugView {
    Lit = 0,         // Normal rendering
    Normals = 1,     // RGB = normal XYZ
    Depth = 2,       // Linearized depth
    Wireframe = 3,   // Wireframe overlay
}
```

### Usage

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Toggle with keys
    if ctx.input.is_key_just_pressed(KeyCode::F1) {
        ctx.renderer.set_debug_view(DebugView::Wireframe);
    }
    if ctx.input.is_key_just_pressed(KeyCode::F2) {
        ctx.renderer.set_debug_view(DebugView::Normals);
    }
    if ctx.input.is_key_just_pressed(KeyCode::F3) {
        ctx.renderer.set_debug_view(DebugView::Depth);
    }
    if ctx.input.is_key_just_pressed(KeyCode::F4) {
        ctx.renderer.set_debug_view(DebugView::Lit);
    }
    
    // Or via console
    // > debug.wireframe
    // > debug.normals
}
```

---

## Performance Overlay

```rust
// RenderStats from Renderer::stats()
pub struct RenderStats {
    pub vertices: u32,
    pub triangles: u32,
    pub draw_calls: u32,
}

// Display in UI
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let stats = ctx.renderer.stats();
    let ui = ctx.renderer.ui();
    
    ui.text(&format!("Draw calls: {}", stats.draw_calls), Vec2::new(10.0, 130.0), 18.0, Color::CYAN);
    ui.text(&format!("Triangles: {}", stats.triangles), Vec2::new(10.0, 150.0), 18.0, Color::CYAN);
    ui.text(&format!("Vertices: {}", stats.vertices), Vec2::new(10.0, 170.0), 18.0, Color::CYAN);
}
```

---

*See [Runtime API](runtime.md) for `Console` and `CommandRegistry` in `UpdateContext`, or [Getting Started](getting-started/first-game.md) for console usage examples.*