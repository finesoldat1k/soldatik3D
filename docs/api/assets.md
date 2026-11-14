# Assets API Reference

Complete API documentation for asset loading and management.

## Texture Loading

```rust
impl Renderer {
    /// Load texture from file (PNG, JPG, KTX)
    pub fn load_texture(&mut self, path: &str) -> Result<TextureHandle>;
    
    /// Create texture from raw data
    pub fn create_texture(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Result<TextureHandle>;
    
    /// Update existing texture
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

### TextureHandle

```rust
pub struct TextureHandle(pub u32);

impl TextureHandle {
    pub fn is_valid(&self) -> bool { self.0 != 0 }
}
```

### TextureInfo

```rust
pub struct TextureInfo {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mip_levels: u32,
}
```

### Usage

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Load from file (relative to working directory or assets/)
    let texture = ctx.renderer.load_texture("assets/textures/albedo.png")?;
    
    // Or create from raw data
    let texture = ctx.renderer.create_texture(
        &raw_pixels,
        512, 512,
        TextureFormat::RGBA8,
    )?;
    
    // Use in material
    let material = Material::default()
        .with_albedo_texture(texture);
}
```

---

## Model Loading

```rust
impl Renderer {
    /// Load model from GLTF/GLB file
    pub fn load_model(&mut self, path: &str) -> Result<ModelHandle>;
    
    /// Load model with materials
    pub fn load_model_with_materials(&mut self, path: &str) 
        -> Result<(ModelHandle, Vec<Material>)>;
}

pub struct ModelHandle(pub u32);

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

### Usage

```rust
fn create_world(&mut self) -> World {
    let mut world = World::default();
    
    // Load model
    let model = ctx.renderer.load_model("assets/models/character.glb")?;
    
    // Or with materials
    let (model_handle, materials) = ctx.renderer.load_model_with_materials(
        "assets/models/scene.gltf"
    )?;
    
    // Spawn each mesh
    let model = ctx.renderer.get_model(model_handle)?;
    for node in &model.nodes {
        if let Some(mesh_handle) = node.mesh {
            let material = node.mesh.map(|m| materials[m]).unwrap_or_default();
            world.spawn_mesh(
                mesh_handle,
                node.transform,
                material,
            );
        }
    }
    
    world
}
```

---

## Shader Loading

```rust
impl Renderer {
    /// Load shader from file
    pub fn load_shader(&mut self, path: &str) -> Result<ShaderHandle>;
    
    /// Create shader from source strings
    pub fn create_shader(
        &mut self,
        vertex_src: &str,
        fragment_src: &str,
    ) -> Result<ShaderHandle>;
}

pub struct ShaderHandle(pub u32);
```

### Shader Sources

**Vulkan (SPIR-V):** Pre-compiled `.spv` files or GLSL compiled at runtime via `shaderc`.

**OpenGL:** GLSL source files (`.vert`, `.frag`).

```rust
// Load from file (auto-detects backend)
let shader = ctx.renderer.load_shader("assets/shaders/custom.frag")?;

// Or create from source
let shader = ctx.renderer.create_shader(
    include_str!("shaders/custom.vert"),
    include_str!("shaders/custom.frag"),
)?;
```

---

## Asset Directory Structure

```
assets/
├── textures/
│   ├── albedo.png           # Base color
│   ├── normal.png           # Normal map
│   ├── roughness.png        # Roughness map
│   ├── metallic.png         # Metallic map
│   ├── ao.png               # Ambient occlusion
│   ├── emissive.png         # Emission map
│   └── skybox/
│       ├── px.ktx, nx.ktx
│       ├── py.ktx, ny.ktx
│       ├── pz.ktx, nz.ktx
├── models/
│   ├── character.glb        # GLTF binary (recommended)
│   ├── character.gltf       # GLTF JSON + .bin + textures
│   ├── environment.glb
│   └── props/
│       ├── crate.glb
│       └── barrel.glb
├── shaders/
│   ├── custom.vert          # Vertex shader
│   ├── custom.frag          # Fragment shader
│   └── post/
│       ├── bloom.frag
│       └── ssao.frag
├── fonts/
│   └── font.png             # Bitmap font atlas
├── audio/                   # Not yet supported in engine
│   ├── music.ogg
│   └── sfx/
└── data/
    ├── level1.json          # Level data
    └── config.toml          # Game config
```

---

## Supported Formats

### Textures

| Format | Support | Notes |
|--------|---------|-------|
| PNG | ✅ Full | Lossless, alpha support |
| JPEG | ✅ Full | Lossy, no alpha |
| KTX/KTX2 | ✅ Full | GPU-compressed (BC7, ASTC, ETC2) |
| DDS | ⚠️ Partial | Limited support |
| TGA | ❌ | Use PNG |
| BMP | ❌ | Use PNG |
| HDR/EXR | ⚠️ Partial | For HDR skyboxes |

### Models

| Format | Support | Notes |
|--------|---------|-------|
| GLTF 2.0 (.gltf) | ✅ Full | JSON + .bin + textures |
| GLB 2.0 (.glb) | ✅ Full | Binary, self-contained |
| OBJ | ❌ | Use GLTF |
| FBX | ❌ | Use GLTF |
| COLLADA | ❌ | Use GLTF |
| STL | ❌ | Use GLTF |

### GLTF Features Supported

| Feature | Support |
|---------|---------|
| Meshes (primitives) | ✅ |
| Materials (PBR) | ✅ |
| Textures (baseColor, normal, metallicRoughness, occlusion, emissive) | ✅ |
| Skinning/Animation | ⚠️ Partial |
| Morph Targets | ❌ |
| Cameras | ❌ |
| Lights | ❌ |
| Extensions (KHR_*) | ⚠️ Partial |

---

## Font Atlas

```rust
pub fn font_atlas_rgba8() -> Vec<u8> {
    // Returns 128x128 RGBA8 bitmap
    // 95 printable ASCII chars (32-126)
    // 16x6 grid, 8x16 per glyph + 1px padding
}
```

### Custom Font

```rust
// Use ab_glyph or fontdue to rasterize TTF/OTF
// Then create texture:
let texture = ctx.renderer.create_texture(
    &atlas_pixels,
    512, 512,
    TextureFormat::RGBA8,
)?;
```

---

## Environment Cubemap

```rust
pub fn generate_environment_cubemap(size: u32) -> Vec<f32> {
    // Procedural sky (sun, atmosphere)
    // Returns RGBA32F: 6 * size * size * 4 floats
    // Face order: +X, -X, +Y, -Y, +Z, -Z
}
```

### Usage

```rust
fn create_world(&mut self) -> World {
    let mut world = World::default();
    
    // Generate procedural sky
    let cubemap_data = generate_environment_cubemap(256);
    
    // Create cubemap texture (handled internally by renderer)
    // Used for IBL (Image-Based Lighting)
    
    world
}
```

---

## Asset Loading Best Practices

### 1. Load Once, Use Many

```rust
struct MyGame {
    textures: HashMap<String, TextureHandle>,
    models: HashMap<String, ModelHandle>,
}

impl Game for MyGame {
    fn create_world(&mut self) -> World {
        // Load all assets once
        self.textures.insert("player".into(), ctx.renderer.load_texture("assets/player.png")?);
        self.models.insert("enemy".into(), ctx.renderer.load_model("assets/enemy.glb")?);
        
        // Reuse handles
        world.spawn_mesh(
            self.models["enemy"].meshes[0],
            transform,
            Material::default().with_albedo_texture(self.textures["player"]),
        );
        
        World::default()
    }
}
```

### 2. Async Loading (Advanced)

```rust
use std::sync::mpsc;

fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    if self.loading {
        if let Ok(asset) = self.rx.try_recv() {
            self.assets.push(asset);
            self.loading = false;
        }
    } else if ctx.input.is_key_just_pressed(KeyCode::L) {
        let (tx, rx) = mpsc::channel();
        self.rx = rx;
        std::thread::spawn(move || {
            let texture = heavy_load();
            tx.send(texture).unwrap();
        });
        self.loading = true;
    }
}
```

### 3. Hot Reloading (Development)

```bash
# Not built-in, but can be implemented:
# Watch assets/ directory for changes
# Reload texture/model on file modification
```

---

## Asset Path Resolution

| Path Type | Resolution |
|-----------|------------|
| `assets/texture.png` | Relative to working directory |
| `./texture.png` | Relative to working directory |
| `/absolute/path.png` | Absolute path |
| `texture.png` | Working directory |

### Android Assets

```bash
# Assets packaged in APK automatically
# Access same way:
ctx.renderer.load_texture("assets/textures/albedo.png")?
```

---

*See [Rendering API](rendering.md) for `Material` and `TextureHandle` usage, or [Getting Started](getting-started/first-game.md) for asset loading examples.*