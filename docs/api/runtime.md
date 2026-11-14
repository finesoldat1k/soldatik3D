# Runtime API Reference

Complete API documentation for the `soldatik3d` crate (runtime facade).

## Module Structure

```rust
use soldatik3d::{
    // Entry points
    run,
    run_android,
    
    // Core traits
    Game,
    GameConfig,
    UpdateContext,
    World,
    
    // Rendering
    Camera,
    Light,
    Material,
    Mesh,
    Renderer,
    BackendKind,
    VsyncMode,
    DebugView,
    FrameData,
    RenderStats,
    
    // Scene
    Entity,
    Transform,
    Scene,
    
    // Input
    Input,
    KeyCode,
    MouseButton,
    GamepadAxis,
    GamepadButton,
    TouchPhase,
    
    // UI/Console
    Console,
    CommandRegistry,
    Command,
    CommandResult,
    
    // Platform
    WindowHints,
    PlatformCapabilities,
    TargetOS,
    TargetArch,
    
    // Math (re-exported from glam)
    Vec2, Vec3, Vec4,
    Mat4, Quat,
    Color,
    
    // Error handling
    anyhow::Result,
};
```

---

## Entry Points

### `run<G>(game: G) -> Result<()>`

Runs the game on desktop platforms (Linux, Windows).

```rust
fn main() -> anyhow::Result<()> {
    soldatik3d::run(MyGame)
}
```

**Requirements:**
- `G: Game + 'static`
- Creates `winit` event loop
- Initializes renderer based on `GameConfig::backend`
- Runs until `ctx.request_exit()` or window close

### `run_android<G>(game: G, android_app: AndroidApp) -> Result<()>`

Runs the game on Android via `NativeActivity`.

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(MyGame, app).unwrap();
}
```

**Requirements:**
- `G: Game + 'static`
- `android_app: winit::platform::android::activity::AndroidApp`
- Must be exported as `#[unsafe(no_mangle)] pub fn android_main`

---

## Game Trait

```rust
pub trait Game: 'static {
    fn config(&self) -> GameConfig;
    fn create_world(&mut self) -> World;
    fn update(&mut self, ctx: &mut UpdateContext<'_>);
}
```

### `config(&self) -> GameConfig`

Called once at startup. Returns engine configuration.

```rust
fn config(&self) -> GameConfig {
    GameConfig::default()
        .with_title("My Game")
        .with_window_size(1920, 1080)
        .with_fullscreen(false)
        .with_vsync(true)
        .with_backend(BackendKind::Vulkan)
        .with_window_hints(WindowHints {
            resizable: true,
            vsync: VsyncMode::On,
            ..Default::default()
        })
}
```

### `create_world(&mut self) -> World`

Called once after renderer initialization. Create initial scene.

```rust
fn create_world(&mut self) -> World {
    let mut world = World::default();
    
    // Add camera
    let camera = Camera::new()
        .with_position(Vec3::new(0.0, 2.0, 5.0))
        .with_target(Vec3::ZERO)
        .with_fov_y(75.0);
    world.add_camera("main".to_string(), camera);
    
    // Add lights
    world.add_light(Light::directional()
        .with_direction(Vec3::new(-0.5, -1.0, -0.3).normalize())
        .with_intensity(3.0));
    
    // Spawn entities
    world.spawn_mesh(mesh, transform, material);
    
    world
}
```

### `update(&mut self, ctx: &mut UpdateContext<'_>)`

Called every frame. Main game logic.

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let dt = ctx.dt;
    let input = ctx.input;
    
    // Handle input
    if input.is_key_just_pressed(KeyCode::Escape) {
        ctx.request_exit();
    }
    
    // Update world
    for entity in ctx.world.entities_mut() {
        entity.transform.translation += velocity * dt;
    }
    
    // Renderer control
    ctx.renderer.set_debug_view(DebugView::Normals);
    ctx.renderer.set_vsync(VsyncMode::Off)?;
    
    // Console output
    ctx.console.print("Frame update");
}
```

---

## GameConfig

```rust
pub struct GameConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub backend: BackendKind,
    pub window_hints: WindowHints,
    pub max_frames: Option<u64>,
    pub startup_commands: Vec<String>,
    pub open_console: bool,
}
```

### Builder Methods

```rust
GameConfig::default()
    .with_title("My Game")
    .with_window_size(1280, 720)
    .with_fullscreen(false)
    .with_vsync(true)
    .with_backend(BackendKind::Vulkan)
    .with_window_hints(WindowHints { ... })
    .with_max_frames(1000)          // Auto-exit after N frames
    .with_startup_commands(vec!["vsync.off".into()])
    .with_open_console(true)
```

### Default Values

| Field | Default |
|-------|---------|
| title | "Soldatik3D Game" |
| width | 1280 |
| height | 720 |
| fullscreen | false |
| vsync | true |
| backend | BackendKind::Vulkan |
| max_frames | None |
| open_console | false |

---

## UpdateContext

```rust
pub struct UpdateContext<'a> {
    pub time: f32,           // Total time in seconds
    pub dt: f32,             // Delta time since last frame
    pub input: &'a Input,    // Input state
    pub world: &'a mut World, // Mutable world
    pub renderer: &'a mut dyn Renderer, // Renderer control
    pub console: &'a mut Console,       // Console output
    pub commands: &'a CommandRegistry,  // Command execution
}
```

### Methods

```rust
impl UpdateContext<'_> {
    /// Request clean engine shutdown
    pub fn request_exit(&mut self);
    
    /// Execute a console command by name
    pub fn execute_command(&mut self, name: &str) -> CommandResult;
}
```

### Usage Examples

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Time
    println!("Time: {:.2}s, DT: {:.4}s", ctx.time, ctx.dt);
    
    // Exit
    if ctx.input.is_key_pressed(KeyCode::Escape) {
        ctx.request_exit();
    }
    
    // World access
    let player = ctx.world.entity_mut(player_id)?;
    player.transform.translation += vel * ctx.dt;
    
    // Renderer
    ctx.renderer.set_debug_view(DebugView::Wireframe);
    ctx.renderer.set_vsync(VsyncMode::Mailbox)?;
    ctx.renderer.screenshot(); // Deferred
    
    // Console
    ctx.console.print("Score: 100");
    ctx.console.error("Failed to load asset");
    
    // Commands
    ctx.commands.execute("vsync.off");
}
```

---

## World

```rust
pub struct World {
    cameras: HashMap<String, Camera>,
    active_camera: Option<String>,
    entities: SlotMap<EntityId, Entity>,
    lights: Vec<Light>,
}
```

### Cameras

```rust
impl World {
    /// Add a camera, returns its name
    pub fn add_camera(&mut self, name: String, camera: Camera) -> String;
    
    /// Get active camera name
    pub fn active_camera_name(&self) -> &str;
    
    /// Get active camera ID
    pub fn active_camera_id(&self) -> Option<CameraId>;
    
    /// Get active camera mutable
    pub fn active_camera_mut(&mut self) -> Option<&mut Camera>;
    
    /// Set active camera by name
    pub fn select_camera(&mut self, name: &str) -> bool;
    
    /// Select next camera
    pub fn select_next_camera(&mut self) -> Option<CameraId>;
    
    /// List all cameras
    pub fn cameras(&self) -> impl Iterator<Item = (&String, &Camera)>;
}
```

### Entities

```rust
impl World {
    /// Spawn a mesh with transform and material
    pub fn spawn_mesh(&mut self, mesh: Mesh, transform: Transform, material: Material) -> EntityId;
    
    /// Get entity by ID
    pub fn entity(&self, id: EntityId) -> Option<&Entity>;
    
    /// Get mutable entity
    pub fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity>;
    
    /// Remove entity
    pub fn remove_entity(&mut self, id: EntityId) -> Option<Entity>;
    
    /// Iterate all entities
    pub fn entities(&self) -> impl Iterator<Item = (EntityId, &Entity)>;
    
    /// Iterate mutable entities
    pub fn entities_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Entity)>;
    
    /// Query entities with component
    pub fn entities_with<T: Component>(&self) -> impl Iterator<Item = (EntityId, &T)>;
}
```

### Lights

```rust
impl World {
    pub fn add_light(&mut self, light: Light) -> LightId;
    pub fn remove_light(&mut self, id: LightId) -> Option<Light>;
    pub fn lights(&self) -> &[Light];
    pub fn lights_mut(&mut self) -> &mut [Light];
    pub fn light(&self, id: LightId) -> Option<&Light>;
    pub fn light_mut(&mut self, id: LightId) -> Option<&mut Light>;
}
```

---

## Camera

```rust
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}
```

### Builder Methods

```rust
Camera::new()
    .with_position(Vec3::new(0.0, 2.0, 5.0))
    .with_target(Vec3::ZERO)
    .with_up(Vec3::Y)
    .with_fov_y(75.0)
    .with_aspect(16.0/9.0)
    .with_near_far(0.1, 1000.0)
```

### Projection Matrix

```rust
impl Camera {
    /// Returns OpenGL-compatible projection matrix (Y-up, depth -1..1)
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y.to_radians(),
            aspect,
            self.near,
            self.far,
        )
    }
    
    /// View matrix
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }
    
    /// View-projection matrix
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }
}
```

---

## Light

```rust
pub enum LightType {
    Directional,
    Point,
    Spot,
    Ambient,
}

pub struct Light {
    pub light_type: LightType,
    pub color: Color,
    pub intensity: f32,
    pub position: Vec3,      // Point/Spot
    pub direction: Vec3,     // Directional/Spot
    pub range: f32,          // Point/Spot
    pub inner_angle: f32,    // Spot
    pub outer_angle: f32,    // Spot
    pub cast_shadows: bool,  // Directional only
}
```

### Builders

```rust
// Directional (sun)
Light::directional()
    .with_direction(Vec3::new(-0.5, -1.0, -0.3).normalize())
    .with_color(Color::rgb(1.0, 0.95, 0.8))
    .with_intensity(3.0)
    .with_cast_shadows(true)

// Point light
Light::point()
    .with_position(Vec3::new(0.0, 2.0, 0.0))
    .with_color(Color::rgb(1.0, 0.5, 0.2))
    .with_intensity(10.0)
    .with_range(20.0)

// Spot light
Light::spot()
    .with_position(Vec3::new(0.0, 5.0, 0.0))
    .with_direction(Vec3::NEG_Y)
    .with_color(Color::WHITE)
    .with_intensity(15.0)
    .with_range(30.0)
    .with_angles(15.0_f32.to_radians(), 30.0_f32.to_radians())

// Ambient
Light::ambient()
    .with_color(Color::rgb(0.2, 0.2, 0.3))
    .with_intensity(0.5)
```

---

## Material

```rust
pub struct Material {
    pub albedo: Color,
    pub albedo_texture: Option<TextureHandle>,
    pub normal_texture: Option<TextureHandle>,
    pub roughness_texture: Option<TextureHandle>,
    pub metallic_texture: Option<TextureHandle>,
    pub emissive: (Color, f32), // (color, intensity)
    pub metallic: f32,
    pub roughness: f32,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
}
```

### Alpha Modes

```rust
pub enum AlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
}
```

### Builders

```rust
Material::default()
    .with_albedo(Color::WHITE)
    .with_albedo_texture(texture_handle)
    .with_normal_texture(normal_handle)
    .with_roughness(0.5)
    .with_metallic(0.0)
    .with_emissive(Color::RED, 2.0)
    .with_alpha_mode(AlphaMode::Blend)
```

---

## Mesh

```rust
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub bounds: Aabb,
}

impl Mesh {
    pub fn cube(size: f32) -> Self;
    pub fn sphere(radius: f32, segments: u32) -> Self;
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self;
    pub fn plane(size: f32, subdivisions: u32) -> Self;
    pub fn quad(size: f32) -> Self;
    pub fn from_gltf(data: &[u8], mesh_index: usize) -> Result<Self>;
}
```

---

## Renderer Trait

```rust
pub trait Renderer: Send + Sync {
    fn render(&mut self, frame: &FrameData<'_>) -> Result<()>;
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    fn screenshot(&mut self) -> Result<()>;
    fn wait_idle(&self) -> Result<()>;
    fn stats(&self) -> RenderStats;
    fn extent(&self) -> (u32, u32);
    fn vsync(&self) -> VsyncMode;
    fn set_vsync(&mut self, mode: VsyncMode) -> Result<()>;
    fn set_debug_view(&mut self, view: DebugView);
}
```

### Debug Views

```rust
pub enum DebugView {
    Lit = 0,        // Normal rendering
    Normals = 1,    // Visualize normals
    Depth = 2,      // Visualize depth
    Wireframe = 3,  // Wireframe mode
}
```

### RenderStats

```rust
pub struct RenderStats {
    pub vertices: u32,
    pub triangles: u32,
    pub draw_calls: u32,
}
```

---

## FrameData

```rust
pub struct FrameData<'a> {
    pub camera: CameraFrame,
    pub lights: &'a [Light],
    pub ui_vertices: &'a [UiVertex],
    pub debug_view: DebugView,
    pub time: f32,
    pub dt: f32,
}
```

---

## Input

```rust
pub struct Input {
    // Keyboard
    pub fn is_key_pressed(&self, key: KeyCode) -> bool;
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool;
    pub fn is_key_just_released(&self, key: KeyCode) -> bool;
    
    // Mouse
    pub fn mouse_position(&self) -> Vec2;
    pub fn mouse_delta(&self) -> Vec2;
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool;
    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool;
    pub fn scroll_delta(&self) -> Vec2;
    
    // Gamepad
    pub fn gamepad(&self, id: usize) -> Option<&GamepadState>;
    pub fn gamepads(&self) -> &[GamepadState];
    
    // Touch (Android)
    pub fn touches(&self) -> &[TouchState];
}

pub struct GamepadState {
    pub fn axis(&self, axis: GamepadAxis) -> f32;
    pub fn is_button_pressed(&self, button: GamepadButton) -> bool;
    pub fn is_button_just_pressed(&self, button: GamepadButton) -> bool;
}
```

### Key Codes

```rust
pub enum KeyCode {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Special
    Escape, Space, Enter, Tab, Backspace, Delete,
    Insert, Home, End, PageUp, PageDown,
    Left, Right, Up, Down,
    LeftShift, RightShift, LeftCtrl, RightCtrl,
    LeftAlt, RightAlt, LeftMeta, RightMeta,
    // ... more
}
```

### Mouse Buttons

```rust
pub enum MouseButton {
    Left, Right, Middle, Back, Forward, Other(u16)
}
```

### Gamepad

```rust
pub enum GamepadAxis {
    LeftStickX, LeftStickY,
    RightStickX, RightStickY,
    LeftTrigger, RightTrigger,
}

pub enum GamepadButton {
    South, East, North, West,  // A, B, Y, X (cross-platform)
    C, Z,                      // Extra
    LeftTrigger, RightTrigger,
    LeftTrigger2, RightTrigger2,
    Select, Start, Mode,
    LeftThumb, RightThumb,
    DPadUp, DPadDown, DPadLeft, DPadRight,
}
```

---

## Console

```rust
pub struct Console {
    pub fn print(&mut self, msg: impl Into<String>);
    pub fn error(&mut self, msg: impl Into<String>);
    pub fn warning(&mut self, msg: impl Into<String>);
    pub fn clear(&mut self);
    pub fn history(&self) -> &[String];
    pub fn set_visible(&mut self, visible: bool);
    pub fn is_visible(&self) -> bool;
}
```

---

## Commands

```rust
pub struct CommandRegistry {
    pub fn register(&mut self, command: Command);
    pub fn get(&self, name: &str) -> Option<&Command>;
    pub fn execute(&mut self, name: &str, args: &[&str]) -> CommandResult;
}

pub struct Command {
    pub name: String,
    pub description: String,
    pub callback: Box<dyn Fn(&mut CommandContext<'_>) -> CommandResult + Send + Sync>,
}

pub enum CommandResult {
    Ok(Option<String>),
    Err(String),
}
```

### Registering Commands

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    ctx.commands.register(Command::new(
        "heal",
        "Restore player health to full",
        |ctx| {
            // Access game state through context
            // (Requires storing game state in World or using a global)
            CommandResult::Ok(Some("Health restored!".into()))
        },
    ));
    
    // With arguments
    ctx.commands.register(Command::new(
        "teleport",
        "Teleport to coordinates: teleport <x> <y> <z>",
        |ctx| {
            let args = ctx.args; // Vec<&str>
            if args.len() != 3 {
                return CommandResult::Err("Usage: teleport <x> <y> <z>".into());
            }
            let pos = Vec3::new(
                args[0].parse()?,
                args[1].parse()?,
                args[2].parse()?,
            );
            // Teleport logic
            CommandResult::Ok(Some(format!("Teleported to {:?}", pos)))
        },
    ));
}
```

---

## Platform Types

```rust
pub struct WindowHints {
    pub resizable: bool,
    pub vsync: VsyncMode,
    pub transparent: bool,
    pub decorations: bool,
    pub maximized: bool,
    pub visible: bool,
}

pub struct PlatformCapabilities {
    pub vulkan: bool,
    pub opengl: bool,
    pub opengl_es: bool,
    pub max_texture_size: u32,
    pub max_anisotropy: f32,
}

pub enum TargetOS {
    Linux,
    Windows,
    Android,
    Unknown,
}

pub enum TargetArch {
    X86_64,
    Aarch64,
    Arm,
    Unknown,
}
```

---

## Math Types (Re-exported from glam)

```rust
// Vectors
pub struct Vec2 { pub x: f32, pub y: f32 }
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
pub struct Vec4 { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }

// Matrices
pub struct Mat4 { /* 4x4 column-major */ }
pub struct Mat3 { /* 3x3 column-major */ }

// Quaternion
pub struct Quat { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }

// Color
pub struct Color {
    pub r: f32, pub g: f32, pub b: f32, pub a: f32,
}
impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub fn rgb(r: f32, g: f32, b: f32) -> Self;
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self;
}
```

---

## Environment Variables

| Variable | Values | Description |
|----------|--------|-------------|
| `SOLDATIK3D_BACKEND` | `vulkan`, `opengl` | Force graphics backend |
| `SOLDATIK3D_VSYNC` | `on`, `off`, `adaptive`, `mailbox` | VSync mode |
| `SOLDATIK3D_MAX_FRAMES` | `<number>` | Auto-exit after N frames |
| `SOLDATIK3D_COMMANDS` | `"cmd1;cmd2"` | Run commands at startup |
| `SOLDATIK3D_CONSOLE` | `1` | Open console at startup |
| `SOLDATIK3D_DEBUG_VIEW` | `normals`, `depth`, `wireframe` | Initial debug view |
| `SOLDATIK3D_VALIDATION` | `1` | Enable Vulkan validation |

---

*See [Game Loop](guides/game-loop.md) for frame timing details, or [Platform Guides](guides/) for platform-specific details.*