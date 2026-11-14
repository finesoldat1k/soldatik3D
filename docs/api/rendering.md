# Rendering API Reference

Complete API documentation for rendering-related types and functions.

## Renderer Trait

```rust
pub trait Renderer: Send + Sync {
    /// Render a frame with the given data
    fn render(&mut self, frame: &FrameData<'_>) -> Result<()>;
    
    /// Handle window resize
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    
    /// Request screenshot (executed next frame)
    fn screenshot(&mut self) -> Result<()>;
    
    /// Wait for GPU to finish all work
    fn wait_idle(&self) -> Result<()>;
    
    /// Get render statistics
    fn stats(&self) -> RenderStats;
    
    /// Get current render extent
    fn extent(&self) -> (u32, u32);
    
    /// Get current VSync mode
    fn vsync(&self) -> VsyncMode;
    
    /// Set VSync mode
    fn set_vsync(&mut self, mode: VsyncMode) -> Result<()>;
    
    /// Set debug visualization mode
    fn set_debug_view(&mut self, view: DebugView);
}
```

## FrameData

Per-frame data passed to `Renderer::render()`:

```rust
pub struct FrameData<'a> {
    /// Camera view and projection matrices
    pub camera: CameraFrame,
    
    /// Active lights (max 8 directional/point, unlimited ambient)
    pub lights: &'a [Light],
    
    /// UI vertices for immediate-mode GUI
    pub ui_vertices: &'a [UiVertex],
    
    /// Debug visualization mode
    pub debug_view: DebugView,
    
    /// Total time since start (seconds)
    pub time: f32,
    
    /// Delta time since last frame (seconds)
    pub dt: f32,
}
```

## CameraFrame

Camera matrices for the current frame:

```rust
pub struct CameraFrame {
    /// View matrix (world → view)
    pub view: Mat4,
    
    /// Projection matrix (view → clip)
    pub projection: Mat4,
    
    /// Combined view-projection (clip ← world)
    pub view_projection: Mat4,
    
    /// Inverse view-projection (world ← clip)
    pub inv_view_projection: Mat4,
    
    /// Camera world position
    pub position: Vec3,
    
    /// Camera forward vector
    pub forward: Vec3,
    
    /// Camera up vector
    pub up: Vec3,
    
    /// Camera right vector
    pub right: Vec3,
}
```

## Light

```rust
pub struct Light {
    /// Light type
    pub light_type: LightType,
    
    /// Light color (linear RGB)
    pub color: Color,
    
    /// Intensity multiplier
    pub intensity: f32,
    
    /// Position (for Point/Spot)
    pub position: Vec3,
    
    /// Direction (for Directional/Spot), normalized
    pub direction: Vec3,
    
    /// Range/attenuation distance (Point/Spot)
    pub range: f32,
    
    /// Inner cone angle in radians (Spot)
    pub inner_angle: f32,
    
    /// Outer cone angle in radians (Spot)
    pub outer_angle: f32,
    
    /// Cast shadows (Directional only)
    pub cast_shadows: bool,
    
    /// Shadow map bias
    pub shadow_bias: f32,
    
    /// Shadow map normal offset
    pub shadow_normal_bias: f32,
}

pub enum LightType {
    Directional,  // Sun-like, infinite distance
    Point,        // Omnidirectional, attenuated
    Spot,         // Cone-shaped, attenuated
    Ambient,      // Global constant illumination
}
```

### Light Builders

```rust
// Directional (sun)
Light::directional()
    .with_direction(Vec3::new(-0.5, -1.0, -0.3).normalize())
    .with_color(Color::rgb(1.0, 0.95, 0.85))
    .with_intensity(3.0)
    .with_cast_shadows(true)
    .with_shadow_bias(0.005)
    .with_shadow_normal_bias(0.01)

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

## Material

```rust
pub struct Material {
    /// Base color (tint)
    pub albedo: Color,
    
    /// Albedo texture
    pub albedo_texture: Option<TextureHandle>,
    
    /// Normal map texture
    pub normal_texture: Option<TextureHandle>,
    
    /// Roughness map texture
    pub roughness_texture: Option<TextureHandle>,
    
    /// Metallic map texture
    pub metallic_texture: Option<TextureHandle>,
    
    /// Emissive (color, intensity)
    pub emissive: (Color, f32),
    
    /// Metallic factor [0, 1]
    pub metallic: f32,
    
    /// Roughness factor [0, 1]
    pub roughness: f32,
    
    /// Alpha blending mode
    pub alpha_mode: AlphaMode,
    
    /// Render both sides
    pub double_sided: bool,
}

pub enum AlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
}
```

### Material Builder

```rust
Material::default()
    .with_albedo(Color::WHITE)
    .with_albedo_texture(texture_handle)
    .with_normal_texture(normal_handle)
    .with_roughness_texture(roughness_handle)
    .with_metallic_texture(metallic_handle)
    .with_emissive(Color::rgb(1.0, 0.5, 0.0), 5.0)
    .with_metallic(0.0)
    .with_roughness(0.5)
    .with_alpha_mode(AlphaMode::Opaque)
    .with_double_sided(false)
```

### AlphaMode

```rust
pub enum AlphaMode {
    /// Fully opaque, depth write enabled
    Opaque,
    
    /// Alpha test with cutoff
    Mask { cutoff: f32 },  // cutoff ∈ [0, 1]
    
    /// Alpha blending
    Blend,
}
```

## Mesh

```rust
pub struct Mesh {
    /// Vertex buffer
    pub vertices: Vec<Vertex>,
    
    /// Index buffer (optional)
    pub indices: Option<Vec<u32>>,
    
    /// Bounding box
    pub bounds: Aabb,
}

impl Mesh {
    /// Unit cube centered at origin
    pub fn cube(size: f32) -> Self;
    
    /// UV sphere
    pub fn sphere(radius: f32, segments: u32) -> Self;
    
    /// Cylinder
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self;
    
    /// Plane
    pub fn plane(size: f32, subdivisions: u32) -> Self;
    
    /// Quad
    pub fn quad(size: f32) -> Self;
    
    /// Load from GLTF binary data
    pub fn from_gltf(data: &[u8], mesh_index: usize) -> Result<Self>;
    
    /// Generate tangent space
    pub fn generate_tangents(&mut self) -> Result<()>;
}
```

## Vertex

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Position (world space)
    pub position: Vec3,
    
    /// Normal (world space, normalized)
    pub normal: Vec3,
    
    /// Tangent (world space, normalized)
    pub tangent: Vec3,
    
    /// Bitangent sign (-1 or 1)
    pub bitangent_sign: f32,
    
    /// Texture coordinates
    pub uv: Vec2,
    
    /// Vertex color (linear RGB)
    pub color: Color,
}
```

## Transform

```rust
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn identity() -> Self;
    
    pub fn from_translation(translation: Vec3) -> Self;
    pub fn from_rotation(rotation: Quat) -> Self;
    pub fn from_scale(scale: Vec3) -> Self;
    
    pub fn matrix(&self) -> Mat4;
    pub fn inverse(&self) -> Self;
    
    pub fn translate(&mut self, v: Vec3);
    pub fn rotate(&mut self, q: Quat);
    pub fn scale(&mut self, v: Vec3);
}
```

## Entity

```rust
pub struct Entity {
    pub id: EntityId,
    pub transform: Transform,
    pub mesh: Option<MeshHandle>,
    pub material: Material,
    pub visible: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    pub tags: HashSet<String>,
}
```

## Scene/World

```rust
pub struct World {
    // Cameras
    pub fn add_camera(&mut self, name: String, camera: Camera) -> String;
    pub fn active_camera(&self) -> Option<&Camera>;
    pub fn active_camera_mut(&mut self) -> Option<&mut Camera>;
    pub fn select_camera(&mut self, name: &str) -> bool;
    pub fn cameras(&self) -> impl Iterator<Item = (&String, &Camera)>;
    
    // Entities
    pub fn spawn_mesh(&mut self, mesh: Mesh, transform: Transform, material: Material) -> EntityId;
    pub fn entity(&self, id: EntityId) -> Option<&Entity>;
    pub fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity>;
    pub fn remove_entity(&mut self, id: EntityId) -> Option<Entity>;
    pub fn entities(&self) -> impl Iterator<Item = (EntityId, &Entity)>;
    pub fn entities_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Entity)>;
    
    // Lights
    pub fn add_light(&mut self, light: Light) -> LightId;
    pub fn light(&self, id: LightId) -> Option<&Light>;
    pub fn light_mut(&mut self, id: LightId) -> Option<&mut Light>;
    pub fn remove_light(&mut self, id: LightId) -> Option<Light>;
    pub fn lights(&self) -> &[Light];
    pub fn lights_mut(&mut self) -> &mut [Light];
    
    // Queries
    pub fn entities_with<T: Component>(&self) -> impl Iterator<Item = (EntityId, &T)>;
}
```

## Renderer Control (from UpdateContext)

```rust
impl UpdateContext<'_> {
    /// Set debug visualization mode
    pub fn set_debug_view(&mut self, view: DebugView);
    
    /// Get current debug view
    pub fn debug_view(&self) -> DebugView;
    
    /// Set VSync mode
    pub fn set_vsync(&mut self, mode: VsyncMode) -> Result<()>;
    
    /// Get current VSync mode
    pub fn vsync(&self) -> VsyncMode;
    
    /// Request screenshot (next frame)
    pub fn screenshot(&mut self) -> Result<()>;
    
    /// Get render statistics
    pub fn stats(&self) -> RenderStats;
    
    /// Get render extent
    pub fn extent(&self) -> (u32, u32);
    
    /// Wait for GPU idle
    pub fn wait_idle(&self) -> Result<()>;
}
```

## DebugView

```rust
pub enum DebugView {
    /// Normal lit rendering
    Lit = 0,
    
    /// Visualize normals (RGB = XYZ)
    Normals = 1,
    
    /// Visualize depth (linearized)
    Depth = 2,
    
    /// Wireframe overlay
    Wireframe = 3,
}
```

## VsyncMode

```rust
pub enum VsyncMode {
    /// Wait for vblank (default)
    On,
    
    /// Immediate present, no sync
    Off,
    
    /// Adaptive: vsync when > refresh rate
    Adaptive,
    
    /// Mailbox (triple buffering, latest frame)
    Mailbox,
}
```

## RenderStats

```rust
pub struct RenderStats {
    pub vertices: u32,
    pub triangles: u32,
    pub draw_calls: u32,
}
```

## Texture Handling

```rust
impl Renderer {
    /// Load texture from file (PNG, JPG, KTX)
    pub fn load_texture(&mut self, path: &str) -> Result<TextureHandle>;
    
    /// Create texture from raw data
    pub fn create_texture(&mut self, data: &[u8], width: u32, height: u32, format: TextureFormat) -> Result<TextureHandle>;
    
    /// Update texture data
    pub fn update_texture(&mut self, handle: TextureHandle, data: &[u8]) -> Result<()>;
    
    /// Get texture info
    pub fn texture_info(&self, handle: TextureHandle) -> Option<TextureInfo>;
}

pub enum TextureFormat {
    R8,
    RG8,
    RGB8,
    RGBA8,
    RGBA16F,
    RGBA32F,
    Depth24,
    Depth32F,
}
```

## Model Loading

```rust
impl Renderer {
    /// Load mesh from GLTF/GLB
    pub fn load_model(&mut self, path: &str) -> Result<ModelHandle>;
    
    /// Load mesh with materials
    pub fn load_model_with_materials(&mut self, path: &str) -> Result<(ModelHandle, Vec<Material>)>;
}

pub struct Model {
    pub meshes: Vec<MeshHandle>,
    pub materials: Vec<Material>,
    pub nodes: Vec<ModelNode>,
    pub animations: Vec<Animation>,
}

pub struct ModelNode {
    pub name: String,
    pub mesh: Option<MeshHandle>,
    pub transform: Transform,
    pub children: Vec<usize>,
}
```

## Shader Loading

```rust
impl Renderer {
    /// Load custom shader (GLSL for OpenGL, SPIR-V for Vulkan)
    pub fn load_shader(&mut self, path: &str) -> Result<ShaderHandle>;
    
    /// Create shader from source
    pub fn create_shader(&mut self, vertex_src: &str, fragment_src: &str) -> Result<ShaderHandle>;
}
```

## Render Pass Customization

```rust
impl UpdateContext<'_> {
    /// Add custom render pass (advanced)
    pub fn add_render_pass(&mut self, pass: Box<dyn RenderPass>);
}

pub trait RenderPass: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&mut self, renderer: &mut dyn Renderer, frame: &FrameData<'_>) -> Result<()>;
}
```

## Screenshot

```rust
impl UpdateContext<'_> {
    /// Request screenshot (saved next frame)
    pub fn screenshot(&mut self) -> Result<()>;
    
    /// Screenshot with custom filename
    pub fn screenshot_as(&mut self, filename: &str) -> Result<()>;
}

// Output: screenshot_<timestamp>.png in working directory
// Format: PNG, RGBA8, vertical flip applied
```

---

*See [Runtime API](runtime.md) for `Game`, `GameConfig`, `UpdateContext`, and [Game Loop](guides/game-loop.md) for frame timing.*