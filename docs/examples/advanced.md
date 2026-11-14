# Advanced Examples

Complex examples demonstrating advanced engine features.

## Procedural Terrain

```rust
use soldatik3d::{
    run, Game, GameConfig, UpdateContext, World,
    Vec3, Mesh, Material, Transform, Color,
    PerlinNoise,
};

struct TerrainGame {
    terrain_chunks: Vec<TerrainChunk>,
    noise: PerlinNoise,
}

struct TerrainChunk {
    entity: soldatik3d::EntityId,
    position: Vec3,
    size: u32,
}

impl Default for TerrainGame {
    fn default() -> Self {
        Self {
            terrain_chunks: Vec::new(),
            noise: PerlinNoise::new(fastrand::u64(..)),
        }
    }
}

impl Game for TerrainGame {
    fn config(&self) -> GameConfig {
        GameConfig::default().with_title("Procedural Terrain")
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();

        // Camera
        world.add_camera("main".into(), soldatik3d::Camera::new()
            .with_position(soldatik3d::Vec3::new(0.0, 50.0, 100.0))
            .with_target(soldatik3d::Vec3::ZERO)
            .with_fov_y(60.0));

        // Sun
        world.add_light(soldatik3d::Light::directional()
            .with_direction(soldatik3d::Vec3::new(-0.5, -1.0, -0.3).normalize())
            .with_intensity(3.0)
            .with_cast_shadows(true));

        // Generate initial chunks
        let chunk_size = 32;
        let chunk_scale = 2.0;
        for x in -2..=2 {
            for z in -2..=2 {
                let chunk = self.generate_chunk(&mut world, x, z, chunk_size, chunk_scale);
                self.terrain_chunks.push(chunk);
            }
        }

        World::default()
    }

    fn generate_chunk(&mut self, world: &mut World, cx: i32, cz: i32, size: u32, scale: f32) -> TerrainChunk {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let half = size as f32 / 2.0;
        let world_x = cx as f32 * size as f32 * 2.0;
        let world_z = cz as f32 * size as f32 * 2.0;

        for gz in 0..=size {
            for gx in 0..=size {
                let wx = (gx as f32 - size as f32 / 2.0) * scale + world_x;
                let wz = (gz as f32 - size as f32 / 2.0) * scale + world_z;
                
                // Multi-octave noise
                let height = self.noise.get([
                    wx * 0.01, wz * 0.01
                ]) * 10.0 +
                self.noise.get([
                    wx * 0.05, wz * 0.05
                ]) * 5.0 +
                self.noise.get([
                    wx * 0.1, wz * 0.1
                ]) * 2.0;

                let color = if height > 5.0 {
                    soldatik3d::Color::rgb(0.6, 0.5, 0.4) // Rock
                } else if height > 2.0 {
                    soldatik3d::Color::rgb(0.3, 0.6, 0.3) // Grass
                } else if height > 0.0 {
                    soldatik3d::Color::rgb(0.2, 0.5, 0.8) // Water
                } else {
                    soldatik3d::Color::rgb(0.1, 0.2, 0.6) // Deep water
                };

                vertices.push(soldatik3d::Vertex {
                    position: soldatik3d::Vec3::new(wx, height, wz),
                    normal: soldatik3d::Vec3::Y,
                    tangent: soldatik3d::Vec3::X,
                    bitangent_sign: 1.0,
                    uv: soldatik3d::Vec2::new(gx as f32 / size as f32, gz as f32 / size as f32),
                    color: color,
                });
            }
        }

        // Generate indices (quads -> 2 triangles)
        let mut indices = Vec::new();
        for gz in 0..size {
            for gx in 0..size {
                let a = gz * (size + 1) + gx;
                let b = gz * (size + 1) + gx + 1;
                let c = (gz + 1) * (size + 1) + gx;
                let d = (gz + 1) * (size + 1) + gx + 1;
                
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }

        let mesh = soldatik3d::Mesh { vertices, indices: Some(indices), bounds: soldatik3d::Aabb::default() };
        let entity = world.spawn_mesh(mesh, soldatik3d::Transform::identity(),
            soldatik3d::Material::default().with_albedo(soldatik3d::Color::WHITE));

        TerrainChunk {
            entity,
            position: soldatik3d::Vec3::new(cx as f32 * size as f32 * 2.0, 0.0, cz as f32 * size as f32 * 2.0),
            size,
        }
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // Chunk streaming based on camera position would go here
        
        if ctx.input.is_key_just_pressed(soldatik3d::KeyCode::Escape) {
            ctx.request_exit();
        }
    }
}
```

---

## Custom Shader Integration

```rust
// Custom PBR shader with additional features
const CUSTOM_VERTEX: &str = r#"
#version 330 core
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec3 a_tangent;
layout(location = 3) in float a_bitangent_sign;
layout(location = 4) in vec2 a_uv;
layout(location = 5) in vec4 a_color;

layout(std140, binding = 0) uniform PerFrame {
    mat4 u_view_projection;
    mat4 u_view;
    vec3 u_camera_pos;
    float u_time;
};

layout(location = 0) out vec3 v_world_pos;
layout(location = 1) out vec3 v_normal;
layout(location = 2) out vec3 v_tangent;
layout(location = 3) out float v_bitangent_sign;
layout(location = 4) out vec2 v_uv;
layout(location = 5) out vec4 v_color;

void main() {
    vec4 world_pos = vec4(a_position, 1.0);
    v_world_pos = world_pos.xyz;
    v_normal = normalize(mat3(u_view) * a_normal);
    v_tangent = normalize(mat3(u_view) * a_tangent);
    v_bitangent_sign = a_bitangent_sign;
    v_uv = a_uv;
    v_color = a_color;
    gl_Position = u_view_projection * world_pos;
}
"#;

const CUSTOM_FRAGMENT: &str = r#"
#version 330 core
layout(location = 0) in vec3 v_world_pos;
layout(location = 1) in vec3 v_normal;
layout(location = 2) in vec3 v_tangent;
layout(location = 3) in float v_bitangent_sign;
layout(location = 4) in vec2 v_uv;
layout(location = 5) in vec4 v_color;

layout(std140, binding = 1) uniform Lights {
    vec4 u_light_pos[8];
    vec4 u_light_color[8];
    int u_light_count;
    vec3 u_ambient;
};

layout(binding = 0) uniform sampler2D u_albedo_map;
layout(binding = 1) uniform sampler2D u_normal_map;
layout(binding = 2) uniform sampler2D u_roughness_map;
layout(binding = 3) uniform sampler2D u_metallic_map;
layout(binding = 4) uniform sampler2D u_ao_map;

layout(binding = 5) uniform samplerCube u_env_map;

out vec4 frag_color;

vec3 unpack_normal(vec3 map_normal) {
    vec3 n = map_normal * 2.0 - 1.0;
    return normalize(n);
}

vec3 calculate_lighting(vec3 N, vec3 V, vec3 albedo, float metallic, float roughness, float ao) {
    vec3 Lo = vec3(0.0);
    
    for (int i = 0; i < u_light_count; i++) {
        vec3 L = normalize(u_light_pos[i].xyz - v_world_pos);
        float distance = length(u_light_pos[i].xyz - v_world_pos);
        float attenuation = 1.0 / (distance * distance + 1.0);
        vec3 radiance = u_light_color[i].rgb * u_light_color[i].a * attenuation;
        
        // PBR BRDF (simplified)
        float NdotL = max(dot(N, L), 0.0);
        Lo += radiance * albedo * NdotL;
    }
    
    // Ambient (IBL approximation)
    vec3 ambient = u_ambient * albedo * ao;
    
    return Lo + ambient;
}

void main() {
    vec3 N = normalize(v_normal);
    vec3 V = normalize(-v_world_pos);
    
    // Sample textures
    vec4 albedo_tex = texture(u_albedo_map, v_uv);
    vec3 albedo = albedo_tex.rgb * v_color.rgb;
    float alpha = albedo_tex.a * v_color.a;
    
    float metallic = texture(u_metallic_map, v_uv).r;
    float roughness = texture(u_roughness_map, v_uv).r;
    float ao = texture(u_ao_map, v_uv).r;
    
    // Normal mapping
    vec3 map_normal = texture(u_normal_map, v_uv).xyz;
    vec3 N = N; // Would need TBN matrix for proper normal mapping
    
    vec3 color = calculate_lighting(N, V, albedo, metallic, roughness, ao);
    
    // Tonemapping (ACES)
    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0/2.2));
    
    frag_color = vec4(color, alpha);
}
"#;

struct ShaderDemo {
    custom_shader: Option<soldatik3d::ShaderHandle>,
}

impl Default for ShaderDemo {
    fn default() -> Self { Self { custom_shader: None } }
}

impl Game for ShaderDemo {
    fn create_world(&mut self) -> World {
        let mut world = World::default();
        
        // Load custom shader
        self.custom_shader = Some(ctx.renderer.load_shader("assets/shaders/custom.frag")?);
        
        world
    }
    
    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // Use custom shader for specific entities
        // ctx.renderer.set_shader_for_entity(entity, self.custom_shader.unwrap());
    }
}
```

---

## Compute Shader (Vulkan Only)

```rust
// Not directly exposed in engine API
// Would require extending Renderer trait
// Example structure:

struct ComputeExample {
    compute_pipeline: Option<vk::Pipeline>,
    storage_buffer: Option<vk::Buffer>,
}

impl ComputeExample {
    fn init_compute(&mut self, renderer: &mut VulkanRenderer) {
        // Create compute shader
        let compute_src = r#"
#version 450
layout(local_size_x = 256) in;

layout(set = 0, binding = 0, std430) readonly buffer Input {
    float data[];
} input_buf;

layout(set = 0, binding = 1, std430) buffer Output {
    float data[];
} output_buf;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    if (idx < input_buf.data.length()) {
        output_buf.data[idx] = input_buf.data[idx] * 2.0;
    }
}
"#;
        
        // Compile with shaderc
        // Create pipeline, descriptor sets, dispatch
    }
}
```

---

## Multiplayer Architecture (Conceptual)

```rust
use soldatik3d::*;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

struct MultiplayerGame {
    local_player: EntityId,
    remote_players: Arc<Mutex<HashMap<u64, RemotePlayer>>>,
    network_tx: mpsc::Sender<NetworkMessage>,
    network_rx: mpsc::Receiver<NetworkMessage>,
}

struct RemotePlayer {
    entity: EntityId,
    position: Vec3,
    rotation: Quat,
    last_update: f32,
}

enum NetworkMessage {
    PlayerJoin(u64),
    PlayerLeave(u64),
    PlayerUpdate { id: u64, pos: Vec3, rot: Quat },
    ChatMessage(String),
}

impl Game for MultiplayerGame {
    fn create_world(&mut self) -> World {
        let mut world = World::default();
        
        // Spawn local player
        self.local_player = world.spawn_mesh(
            Mesh::capsule(1.0, 2.0),
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            Material::default().with_albedo(Color::rgb(0.0, 0.5, 1.0))
        );
        
        // Start network thread
        let (tx, rx) = mpsc::channel(100);
        self.network_tx = tx.clone();
        self.network_rx = rx;
        
        std::thread::spawn(move || {
            // Network loop
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Connect to server, handle messages
            });
        });
        
        World::default()
    }
    
    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // Process network messages
        while let Ok(msg) = self.network_rx.try_recv() {
            match msg {
                NetworkMessage::PlayerJoin(id) => {
                    let entity = ctx.world.spawn_mesh(
                        Mesh::capsule(1.0, 2.0),
                        Transform::identity(),
                        Material::default().with_albedo(Color::rgb(1.0, 0.0, 0.5))
                    );
                    self.remote_players.lock().unwrap().insert(id, RemotePlayer {
                        entity, position: Vec3::ZERO, rotation: Quat::IDENTITY, last_update: ctx.time
                    });
                }
                NetworkMessage::PlayerLeave(id) => {
                    if let Some(player) = self.remote_players.lock().unwrap().remove(&id) {
                        ctx.world.remove_entity(player.entity);
                    }
                }
                NetworkMessage::PlayerUpdate { id, pos, rot } => {
                    if let Some(player) = self.remote_players.lock().unwrap().get_mut(&id) {
                        if let Some(entity) = ctx.world.entity_mut(player.entity) {
                            entity.transform.translation = pos;
                            entity.transform.rotation = rot;
                            player.position = pos;
                            player.rotation = rot;
                            player.last_update = ctx.time;
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Send local player position
        if let Some(local) = ctx.world.entity(self.local_player) {
            let msg = NetworkMessage::PlayerUpdate {
                id: 0, // Local player ID
                pos: local.transform.translation,
                rot: local.transform.rotation,
            };
            let _ = self.network_tx.try_send(msg);
        }
    }
}
```

---

## Entity Component System (Custom)

```rust
// Lightweight ECS on top of World
use std::any::TypeId;
use std::collections::HashMap;

pub trait Component: Send + Sync + 'static {}

#[derive(Default)]
pub struct ComponentStorage {
    components: HashMap<TypeId, Box<dyn Any>>,
}

impl ComponentStorage {
    pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) {
        let map = self.components
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<EntityId, T>::new()))
            .downcast_mut::<HashMap<EntityId, T>>()
            .unwrap();
        map.insert(entity, component);
    }
    
    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        self.components.get(&TypeId::of::<T>())?
            .downcast_ref::<HashMap<EntityId, T>>()?
            .get(&entity)
    }
    
    pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.components.get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<EntityId, T>>()?
            .get_mut(&entity)
    }
    
    pub fn query<T: Component>(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.components.get(&TypeId::of::<T>())
            .and_then(|m| m.downcast_ref::<HashMap<EntityId, T>>())
            .into_iter()
            .flat_map(|m| m.iter())
            .map(|(k, v)| (*k, v))
    }
}

// Custom components
#[derive(Component)]
struct Velocity { pub value: Vec3 }

#[derive(Component)]
struct Health { pub current: i32, pub max: i32 }

#[derive(Component)]
struct AI { pub state: AIState, pub target: Option<EntityId> }

enum AIState { Idle, Chase, Attack, Flee }

// Usage in game
struct ECSGame {
    components: ComponentStorage,
}

impl Game for ECSGame {
    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // Physics system
        for (entity, vel) in self.components.query::<Velocity>() {
            if let Some(transform) = ctx.world.entity_mut(entity).map(|e| &mut e.transform) {
                transform.translation += vel.value * ctx.dt;
            }
        }
        
        // Health system
        for (entity, health) in self.components.query::<Health>() {
            if health.current <= 0 {
                ctx.world.remove_entity(entity);
            }
        }
        
        // AI system
        for (entity, ai) in self.components.query_mut::<AI>() {
            match ai.state {
                AIState::Chase => { /* ... */ }
                _ => {}
            }
        }
    }
}
```

---

## Asset Pipeline Integration

```rust
use soldatik3d::*;
use std::path::Path;

struct AssetPipelineGame {
    asset_registry: AssetRegistry,
}

struct AssetRegistry {
    textures: HashMap<String, TextureHandle>,
    models: HashMap<String, ModelHandle>,
    shaders: HashMap<String, ShaderHandle>,
    load_queue: Vec<AssetLoadRequest>,
}

enum AssetLoadRequest {
    Texture { path: String, handle: TextureHandle },
    Model { path: String, handle: ModelHandle },
}

impl AssetRegistry {
    fn load_texture(&mut self, renderer: &mut dyn Renderer, path: &str) -> TextureHandle {
        if let Some(handle) = self.textures.get(path) {
            return *handle;
        }
        let handle = renderer.load_texture(path).unwrap();
        self.textures.insert(path.to_string(), handle);
        handle
    }
    
    fn load_model(&mut self, renderer: &mut dyn Renderer, path: &str) -> ModelHandle {
        if let Some(handle) = self.models.get(path) {
            return *handle;
        }
        let handle = renderer.load_model(path).unwrap();
        self.models.insert(path.to_string(), handle);
        handle
    }
    
    fn hot_reload(&mut self, renderer: &mut dyn Renderer, changed_files: &[String]) {
        for file in changed_files {
            if file.ends_with(".png") || file.ends_with(".jpg") {
                if let Some(handle) = self.textures.get(file) {
                    // Reload texture
                    let data = std::fs::read(file).unwrap();
                    renderer.update_texture(*handle, &data).ok();
                }
            } else if file.ends_with(".glsl") || file.ends_with(".frag") || file.ends_with(".vert") {
                // Reload shader
                // renderer.reload_shader(...)
            }
        }
    }
}

fn watch_assets(registry: Arc<Mutex<AssetRegistry>>) {
    use notify::{Watcher, RecursiveMode, Event};
    
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx).unwrap();
    watcher.watch(Path::new("assets"), RecursiveMode::Recursive).unwrap();
    
    std::thread::spawn(move || {
        for event in rx {
            if let Ok(Event { paths, .. }) = event {
                let changed: Vec<String> = paths.into_iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                if let Ok(mut reg) = registry.lock() {
                    reg.hot_reload(&mut renderer, &changed);
                }
            }
        }
    }
}
```

---

*See [API Reference](api/) for complete API documentation, or [Getting Started](getting-started/first-game.md) for foundational concepts.*