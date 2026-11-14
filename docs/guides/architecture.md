# Architecture Overview

Deep dive into Soldatik3D's internal architecture and design decisions.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application                               │
│  (Your Game implementing `Game` trait)                          │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Runtime Facade (`runtime`)                 │
│  • `run()` / `run_android()` - Entry points                     │
│  • `App` - Main loop, window, input, timing                     │
│  • `CommandRegistry` - Console commands                         │
│  • `Backend` selection (Vulkan/OpenGL)                          │
└──────────────────────────┬──────────────────────────────────────┘
                           │
         ┌─────────────────┴─────────────────┐
         ▼                                   ▼
┌─────────────────────────┐       ┌─────────────────────────┐
│   Vulkan Backend        │       │   OpenGL Backend        │
│   (`crates/vulkan`)     │       │   (`crates/opengl`)     │
│                         │       │                         │
│ • `VulkanRenderer`      │       │ • `OpenGlRenderer`      │
│ • `ash` + `shaderc`     │       │ • `glutin` + `glow`     │
│ • SPIR-V shaders        │       │ • GLSL 330/300 ES       │
│ • Explicit sync         │       │ • Implicit sync         │
└───────────┬─────────────┘       └───────────┬─────────────┘
            │                                   │
            └─────────────────┬─────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Shared Render Abstraction (`render`)         │
│  • `Renderer` trait - Common interface                          │
│  • `FrameData` - Per-frame render data                          │
│  • `CameraFrame`, `RenderStats`, `DebugView`                    │
│  • `BackendKind`, `VsyncMode`, `WindowSurface`                  │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Scene & Assets                           │
│  • `Scene`, `World` - ECS-lite entity management                │
│  • `Mesh`, `Material`, `Transform` - Renderables                │
│  • `Camera`, `Light` - View and lighting                        │
│  • `assets` - Texture/model/shader loading                      │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Platform Abstraction (`platform`)          │
│  • `winit` - Window/events                                      │
│  • `Input`, `InputMap` - Unified input                          │
│  • `TargetOS`, `TargetArch` - Platform detection                │
│  • Android: `android-activity` + `NativeActivity`               │
└─────────────────────────────────────────────────────────────────┘
```

## Crate Responsibilities

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `runtime` | Public facade, main loop, backend selection | `run`, `Game`, `App`, `CommandRegistry` |
| `vulkan` | Vulkan 1.2 renderer | `VulkanRenderer`, `VulkanContext`, `Swapchain` |
| `opengl` | OpenGL 3.3 / ES 3.0 renderer | `OpenGlRenderer`, `GlContext`, `ShaderProgram` |
| `render` | Renderer abstraction | `Renderer` trait, `FrameData`, `CameraFrame` |
| `scene` | Scene graph, ECS-lite | `World`, `Entity`, `Mesh`, `Camera`, `Light` |
| `assets` | Asset loading | `TextureData`, `load_gltf`, `font_atlas` |
| `ui` | Immediate-mode GUI | `UiDrawList`, `UiVertex`, font rendering |
| `platform` | Window/input abstraction | `Input`, `InputMap`, `WindowHints` |

## Data Flow

### Frame Loop

```
┌────────────────────────────────────────────────────────────┐
│ App::run()                                                 │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ Event Loop (winit)                                     │ │
│ │   ┌──────────────────────────────────────────────────┐ │ │
│ │   │ Event::WindowEvent → Input::process_event()     │ │ │
│ │   │ Event::RedrawRequested → App::render_frame()    │ │ │
│ │   └──────────────────────────────────────────────────┘ │ │
│ └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│ App::render_frame()                                        │
│ 1. Update input state                                      │
│ 2. Game::update(&mut UpdateContext)                        │
│    ├─ User logic runs                                      │
│    ├─ Input/World/Renderer/Commands accessible             │
│    └─ Returns when frame logic done                        │
│ 3. Renderer::render(&FrameData)                            │
│    ├─ Shadow pass                                          │
│    ├─ HDR scene pass                                       │
│    ├─ SSAO pass (half-res)                                 │
│    ├─ Bloom pass (half-res)                                │
│    ├─ Composite + Tone mapping                             │
│    ├─ UI pass                                              │
│    └─ Present / Swap                                       │
└────────────────────────────────────────────────────────────┘
```

### Renderer Trait

Both backends implement `render::Renderer`:

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
}
```

### FrameData

Per-frame data passed to renderer:

```rust
pub struct FrameData<'a> {
    pub camera: CameraFrame,           // View/projection matrices
    pub lights: &'a [Light],           // Active lights (max 8)
    pub ui_vertices: &'a [UiVertex],   // UI vertex buffer
    pub debug_view: DebugView,         // Lit/Normals/Depth/Wireframe
    pub time: f32,                     // Total time
    pub dt: f32,                       // Delta time
}
```

## Backend Differences

| Aspect | Vulkan | OpenGL |
|--------|--------|--------|
| API | Explicit, verbose | Implicit, stateful |
| Shaders | SPIR-V (compiled at runtime via `shaderc`) | GLSL (compiled by driver) |
| Synchronization | Manual barriers, semaphores, fences | Implicit (`glFinish`/`glMemoryBarrier`) |
| Memory | Explicit `VkDeviceMemory` + `VkBuffer`/`VkImage` | Driver-managed (`glGenBuffers`/`glGenTextures`) |
| Descriptors | `VkDescriptorSet` + layouts | Uniform locations + UBOs |
| Pipeline | `VkPipeline` + `VkPipelineLayout` | Program + VAO state |
| Android | Not supported | OpenGL ES 3.0 via EGL |
| Debug | Validation layers (`VK_LAYER_KHRONOS`) | `GL_KHR_debug` / `glDebugMessageCallback` |

## Platform Abstraction

### Window Creation

```rust
// runtime/backend.rs
pub fn create_window(config: &GameConfig) -> Result<PlatformWindow> {
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title(config.title)
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .build(&event_loop)?;
    
    PlatformWindow { window, event_loop }
}
```

### Android Native Activity

```rust
// runtime/lib.rs
#[cfg(target_os = "android")]
pub fn run_android<G>(game: G, android_app: AndroidApp) -> Result<()> {
    let event_loop = EventLoop::builder()
        .with_android_app(android_app)
        .build()?;
    let mut app = App::new(game);
    event_loop.run_app(&mut app)?;
    Ok(())
}
```

### Input System

Unified input across platforms:

```rust
pub struct Input {
    keys: HashMap<KeyCode, KeyState>,
    mouse_pos: Vec2,
    mouse_delta: Vec2,
    mouse_buttons: HashMap<MouseButton, bool>,
    gamepads: Vec<GamepadState>,
    touch: Vec<TouchState>,  // Android only
}

impl Input {
    pub fn is_key_pressed(&self, key: KeyCode) -> bool;
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool;
    pub fn mouse_position(&self) -> Vec2;
    pub fn mouse_delta(&self) -> Vec2;
    pub fn gamepad(&self, id: usize) -> Option<&GamepadState>;
}
```

## Memory Management

### Resource Ownership

```
Renderer owns:
├── Pipeline objects (immutable after creation)
├── Descriptor pools/layouts
├── Command pools/buffers
├── Swapchain + framebuffers
├── Uniform buffers (per-frame)
├── Vertex/Index buffers (static meshes)
└── Texture/Image views (immutable)

Per-frame (recreated on resize):
├── Swapchain images/views
├── Framebuffers
├── HDR/Bloom/SSAO attachments
└── Uniform buffer contents
```

### Buffer Strategy

| Buffer Type | Update Frequency | Memory Type |
|-------------|------------------|-------------|
| Static meshes | Once | `DEVICE_LOCAL` |
| Uniform buffers | Every frame | `HOST_VISIBLE | HOST_COHERENT` |
| UI vertices | Every frame | `HOST_VISIBLE | HOST_COHERENT` |
| Staging buffers | Screenshot only | `HOST_VISIBLE | HOST_COHERENT` |

## Shader System

### Vulkan (Runtime Compilation)

```rust
// crates/vulkan/src/shader.rs
pub fn compile_glsl(source: &str, stage: ShaderStage) -> Result<Vec<u32>> {
    let mut compiler = shaderc::Compiler::new()?;
    let mut options = shaderc::CompileOptions::new()?;
    options.set_target_env(shaderc::TargetEnv::Vulkan, 0);
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    
    let artifact = compiler.compile_into_spirv(
        source, stage.into(), "shader.glsl", "main", Some(&options)
    )?;
    Ok(artifact.as_binary().to_vec())
}
```

### OpenGL (Driver Compilation)

```rust
// crates/opengl/src/shader.rs
pub fn compile_shader(gl: &glow::Context, source: &str, stage: u32) -> Result<Shader> {
    let shader = unsafe { gl.create_shader(stage)? };
    unsafe {
        gl.shader_source(shader, &translated_source); // #version 300 es on Android
        gl.compile_shader(shader);
    }
    // Check compile status...
}
```

### Android Shader Translation

Desktop GLSL → GLSL ES 300:

```rust
fn gles_source(source: &str, fragment: bool) -> String {
    let body = source.split_once('\n').map_or("", |(_, r)| r); // Skip #version
    let mut out = String::new();
    out.push_str("#version 300 es\n");
    out.push_str("precision highp float;\n");
    out.push_str("precision highp int;\n");
    if fragment {
        out.push_str("precision highp sampler2D;\n");
        out.push_str("precision highp samplerCube;\n");
    }
    out.push_str(body);
    out
}
```

## Extensibility Points

### Custom Render Passes

```rust
// In your Game::update
ctx.renderer.add_custom_pass(|renderer, frame| {
    // Access internal renderer state (if exposed)
    // Submit custom draw calls
    Ok(())
});
```

### Custom Materials

```rust
let material = Material {
    albedo: Color::WHITE,
    normal_map: Some(texture_handle),
    metallic: 0.5,
    roughness: 0.3,
    emissive: (Color::RED, 2.0),
    ..Default::default()
};
world.spawn_mesh(mesh, transform, material);
```

### Console Commands

```rust
ctx.commands.register(Command::new(
    "my_cmd",
    "Description",
    |ctx| {
        ctx.console.print("Hello!");
        CommandResult::Ok(None)
    },
));
```

---

*See [Runtime API](api/runtime.md) for complete trait documentation, or [Custom Shaders](guides/custom-shaders.md) for shader development.*