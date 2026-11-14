# Scene API Reference

Complete API documentation for scene management, entities, and world operations.

## World

The `World` is the central scene container holding cameras, entities, and lights.

```rust
pub struct World {
    cameras: HashMap<String, Camera>,
    active_camera: Option<String>,
    entities: SlotMap<EntityId, Entity>,
    lights: Vec<Light>,
}
```

### Creation

```rust
use soldatik3d::World;

let mut world = World::default();
// or
let mut world = World::new();
```

### Cameras

```rust
impl World {
    /// Add a camera, returns the name used
    pub fn add_camera(&mut self, name: String, camera: Camera) -> String;
    
    /// Get active camera name
    pub fn active_camera_name(&self) -> &str;
    
    /// Get active camera ID
    pub fn active_camera_id(&self) -> Option<CameraId>;
    
    /// Get active camera (immutable)
    pub fn active_camera(&self) -> Option<&Camera>;
    
    /// Get active camera (mutable)
    pub fn active_camera_mut(&mut self) -> Option<&mut Camera>;
    
    /// Set active camera by name
    pub fn select_camera(&mut self, name: &str) -> bool;
    
    /// Select next camera in list
    pub fn select_next_camera(&mut self) -> Option<CameraId>;
    
    /// Select previous camera
    pub fn select_previous_camera(&mut self) -> Option<CameraId>;
    
    /// Remove camera by name
    pub fn remove_camera(&mut self, name: &str) -> Option<Camera>;
    
    /// Iterate all cameras
    pub fn cameras(&self) -> impl Iterator<Item = (&String, &Camera)>;
}
```

### Entities

```rust
impl World {
    /// Spawn a new entity with mesh, transform, and material
    pub fn spawn_mesh(
        &mut self,
        mesh: Mesh,
        transform: Transform,
        material: Material,
    ) -> EntityId;
    
    /// Get entity by ID (immutable)
    pub fn entity(&self, id: EntityId) -> Option<&Entity>;
    
    /// Get entity by ID (mutable)
    pub fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity>;
    
    /// Remove entity
    pub fn remove_entity(&mut self, id: EntityId) -> Option<Entity>;
    
    /// Iterate all entities (immutable)
    pub fn entities(&self) -> impl Iterator<Item = (EntityId, &Entity)>;
    
    /// Iterate all entities (mutable)
    pub fn entities_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Entity)>;
    
    /// Find entity by tag
    pub fn find_by_tag(&self, tag: &str) -> Option<EntityId>;
    
    /// Find all entities with tag
    pub fn find_all_by_tag(&self, tag: &str) -> Vec<EntityId>;
}
```

### Lights

```rust
impl World {
    /// Add a light, returns LightId
    pub fn add_light(&mut self, light: Light) -> LightId;
    
    /// Get light (immutable)
    pub fn light(&self, id: LightId) -> Option<&Light>;
    
    /// Get light (mutable)
    pub fn light_mut(&mut self, id: LightId) -> Option<&mut Light>;
    
    /// Remove light
    pub fn remove_light(&mut self, id: LightId) -> Option<Light>;
    
    /// All lights (immutable)
    pub fn lights(&self) -> &[Light];
    
    /// All lights (mutable)
    pub fn lights_mut(&mut self) -> &mut [Light];
}
```

### Component Queries

```rust
impl World {
    /// Iterate entities with a specific component
    pub fn entities_with<T: Component>(&self) 
        -> impl Iterator<Item = (EntityId, &T)>;
    
    /// Iterate entities with multiple components
    pub fn entities_with2<T1: Component, T2: Component>(&self)
        -> impl Iterator<Item = (EntityId, &T1, &T2)>;
    
    /// Mutable query
    pub fn entities_with_mut<T: Component>(&mut self)
        -> impl Iterator<Item = (EntityId, &mut T)>;
}
```

---

## Camera

```rust
pub struct Camera {
    /// World position
    pub position: Vec3,
    
    /// Look-at target
    pub target: Vec3,
    
    /// Up vector (usually Vec3::Y)
    pub up: Vec3,
    
    /// Vertical field of view in degrees
    pub fov_y: f32,
    
    /// Aspect ratio (width/height)
    pub aspect: f32,
    
    /// Near clip plane
    pub near: f32,
    
    /// Far clip plane
    pub far: f32,
}
```

### Construction

```rust
use soldatik3d::Camera;

let camera = Camera::new()
    .with_position(Vec3::new(0.0, 2.0, 5.0))
    .with_target(Vec3::ZERO)
    .with_up(Vec3::Y)
    .with_fov_y(75.0)
    .with_aspect(16.0 / 9.0)
    .with_near_far(0.1, 1000.0);

// Or manually
let camera = Camera {
    position: Vec3::new(0.0, 2.0, 5.0),
    target: Vec3::ZERO,
    up: Vec3::Y,
    fov_y: 75.0,
    aspect: 16.0 / 9.0,
    near: 0.1,
    far: 1000.0,
};
```

### Matrices

```rust
impl Camera {
    /// Projection matrix (OpenGL convention: Y-up, depth -1..1)
    pub fn projection_matrix(&self, aspect: f32) -> Mat4;
    
    /// View matrix (world → view)
    pub fn view_matrix(&self) -> Mat4;
    
    /// View-projection matrix (clip ← world)
    pub fn view_projection(&self, aspect: f32) -> Mat4;
    
    /// Inverse view-projection (world ← clip)
    pub fn inv_view_projection(&self, aspect: f32) -> Mat4;
    
    /// Frustum planes for culling
    pub fn frustum_planes(&self, aspect: f32) -> [Vec4; 6];
}
```

### Camera Controller (Example)

```rust
struct CameraController {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
}

impl CameraController {
    fn update(&mut self, input: &Input, dt: f32) {
        // Mouse rotation
        let delta = input.mouse_delta();
        self.yaw -= delta.x * 0.002;
        self.pitch = (self.pitch - delta.y * 0.002).clamp(-1.5, 1.5);
        
        // Scroll zoom
        self.distance = (self.distance - input.scroll_delta().y * 0.5).clamp(1.0, 50.0);
        
        // WASD movement
        let mut move_dir = Vec3::ZERO;
        if input.is_key_pressed(KeyCode::W) { move_dir.z -= 1.0; }
        if input.is_key_pressed(KeyCode::S) { move_dir.z += 1.0; }
        if input.is_key_pressed(KeyCode::A) { move_dir.x -= 1.0; }
        if input.is_key_pressed(KeyCode::D) { move_dir.x += 1.0; }
        
        let forward = Quat::from_rotation_y(self.yaw) * Vec3::Z;
        let right = Quat::from_rotation_y(self.yaw) * Vec3::X;
        self.target += (forward * -move_dir.z + right * move_dir.x) * dt * 10.0;
    }
    
    fn apply(&self, camera: &mut Camera) {
        let offset = Quat::from_rotation_y(self.yaw) * 
                     Quat::from_rotation_x(self.pitch) * 
                     Vec3::Z * self.distance;
        
        camera.position = self.target + offset;
        camera.target = self.target;
    }
}
```

---

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

### Manipulation

```rust
impl Entity {
    /// Get world transform matrix
    pub fn world_matrix(&self) -> Mat4;
    
    /// Get world position
    pub fn world_position(&self) -> Vec3;
    
    /// Get world rotation
    pub fn world_rotation(&self) -> Quat;
    
    /// Get world scale
    pub fn world_scale(&self) -> Vec3;
    
    /// Set world position
    pub fn set_world_position(&mut self, pos: Vec3);
    
    /// Look at target
    pub fn look_at(&mut self, target: Vec3, up: Vec3);
    
    /// Move relative to local axes
    pub fn translate_local(&mut self, v: Vec3);
    
    /// Rotate around local axis
    pub fn rotate_local(&mut self, axis: Vec3, angle: f32);
    
    /// Scale locally
    pub fn scale_local(&mut self, v: Vec3);
}
```

### Tags

```rust
impl Entity {
    pub fn add_tag(&mut self, tag: &str);
    pub fn remove_tag(&mut self, tag: &str);
    pub fn has_tag(&self, tag: &str) -> bool;
    pub fn tags(&self) -> &HashSet<String>;
}
```

---

## Transform

```rust
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn identity() -> Self;
    pub fn from_translation(v: Vec3) -> Self;
    pub fn from_rotation(q: Quat) -> Self;
    pub fn from_scale(v: Vec3) -> Self;
    pub fn from_translation_rotation_scale(t: Vec3, r: Quat, s: Vec3) -> Self;
    
    /// World matrix (translation * rotation * scale)
    pub fn matrix(&self) -> Mat4;
    
    /// Inverse transform
    pub fn inverse(&self) -> Self;
    
    /// Compose transforms (parent * child)
    pub fn mul(&self, other: &Transform) -> Self;
    
    /// Interpolate between transforms
    pub fn lerp(&self, other: &Transform, t: f32) -> Self;
}
```

### Helpers

```rust
impl Transform {
    /// Forward vector (local -Z)
    pub fn forward(&self) -> Vec3;
    
    /// Right vector (local +X)
    pub fn right(&self) -> Vec3;
    
    /// Up vector (local +Y)
    pub fn up(&self) -> Vec3;
    
    /// Look at target
    pub fn look_at(&mut self, target: Vec3, up: Vec3);
    
    /// Point towards direction
    pub fn face_towards(&mut self, direction: Vec3, up: Vec3);
}
```

---

## Light

```rust
pub struct Light {
    pub light_type: LightType,
    pub color: Color,
    pub intensity: f32,
    pub position: Vec3,
    pub direction: Vec3,
    pub range: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub cast_shadows: bool,
    pub shadow_bias: f32,
    pub shadow_normal_bias: f32,
}

pub enum LightType {
    Directional,
    Point,
    Spot,
    Ambient,
}
```

### Light Helpers

```rust
impl Light {
    /// Directional light (sun)
    pub fn directional() -> Self;
    
    /// Point light
    pub fn point() -> Self;
    
    /// Spot light
    pub fn spot() -> Self;
    
    /// Ambient light
    pub fn ambient() -> Self;
}

impl Light {
    pub fn with_direction(mut self, dir: Vec3) -> Self;
    pub fn with_color(mut self, color: Color) -> Self;
    pub fn with_intensity(mut self, intensity: f32) -> Self;
    pub fn with_position(mut self, pos: Vec3) -> Self;
    pub fn with_range(mut self, range: f32) -> Self;
    pub fn with_angles(mut self, inner: f32, outer: f32) -> Self;
    pub fn with_cast_shadows(mut self, cast: bool) -> Self;
    pub fn with_shadow_bias(mut self, bias: f32) -> Self;
    pub fn with_shadow_normal_bias(mut self, bias: f32) -> Self;
}
```

### Light Types Detail

```rust
// Directional: Infinite distance, parallel rays
Light::directional()
    .with_direction(Vec3::new(-0.5, -1.0, -0.3).normalize())
    .with_color(Color::rgb(1.0, 0.95, 0.85))
    .with_intensity(3.0)
    .with_cast_shadows(true);

// Point: Omnidirectional, quadratic attenuation
Light::point()
    .with_position(Vec3::new(0.0, 2.0, 0.0))
    .with_color(Color::rgb(1.0, 0.5, 0.2))
    .with_intensity(10.0)
    .with_range(20.0);

// Spot: Cone-shaped, quadratic attenuation
Light::spot()
    .with_position(Vec3::new(0.0, 5.0, 0.0))
    .with_direction(Vec3::NEG_Y)
    .with_color(Color::WHITE)
    .with_intensity(15.0)
    .with_range(30.0)
    .with_angles(15.0_f32.to_radians(), 30.0_f32.to_radians());

// Ambient: Constant everywhere
Light::ambient()
    .with_color(Color::rgb(0.2, 0.2, 0.3))
    .with_intensity(0.5);
```

---

## Mesh

```rust
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Option<Vec<u32>>,
    pub bounds: Aabb,
}

impl Mesh {
    pub fn cube(size: f32) -> Self;
    pub fn sphere(radius: f32, segments: u32) -> Self;
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self;
    pub fn plane(size: f32, subdivisions: u32) -> Self;
    pub fn quad(size: f32) -> Self;
    pub fn from_gltf(data: &[u8], mesh_index: usize) -> Result<Self>;
    pub fn generate_tangents(&mut self) -> Result<()>;
}
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
    pub emissive: (Color, f32),
    pub metallic: f32,
    pub roughness: f32,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
}

pub enum AlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
}
```

---

## Primitive Factories

```rust
impl Mesh {
    /// Unit cube [-0.5, 0.5]^3
    pub fn cube(size: f32) -> Self;
    
    /// UV sphere
    pub fn sphere(radius: f32, segments: u32) -> Self;
    
    /// Cylinder (capless by default)
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self;
    
    /// Plane on XZ
    pub fn plane(size: f32, subdivisions: u32) -> Self;
    
    /// Quad on XY
    pub fn quad(size: f32) -> Self;
}
```

---

## Spawning Entities

```rust
// In Game::create_world
fn create_world(&mut self) -> World {
    let mut world = World::default();
    
    // Camera
    world.add_camera("main".into(), Camera::new()
        .with_position(Vec3::new(0.0, 2.0, 5.0))
        .with_fov_y(70.0));
    
    // Lights
    world.add_light(Light::directional()
        .with_direction(Vec3::NEG_Y)
        .with_intensity(3.0));
    
    // Static geometry
    let floor = Mesh::plane(50.0, 1);
    world.spawn_mesh(
        floor,
        Transform::identity(),
        Material::default().with_albedo(Color::rgb(0.3, 0.3, 0.4))
    );
    
    // Dynamic objects
    for i in 0..10 {
        let cube = Mesh::cube(1.0);
        world.spawn_mesh(
            cube,
            Transform::from_translation(Vec3::new(
                fastrand::f32() * 20.0 - 10.0,
                2.0,
                fastrand::f32() * 20.0 - 10.0,
            )),
            Material::default().with_albedo(Color::rgb(0.8, 0.2, 0.2))
        );
    }
    
    world
}
```

---

## Querying Entities

```rust
// In Game::update
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    // Iterate all
    for (id, entity) in ctx.world.entities_mut() {
        entity.transform.translation.y += ctx.dt;
    }
    
    // Query with component (if using ECS components)
    for (id, transform) in ctx.world.entities_with::<Transform>() {
        // ...
    }
    
    // Find by tag
    if let Some(player_id) = ctx.world.find_by_tag("player") {
        if let Some(player) = ctx.world.entity_mut(player_id) {
            // ...
        }
    }
    
    // All with tag
    for enemy_id in ctx.world.find_all_by_tag("enemy") {
        if let Some(enemy) = ctx.world.entity_mut(enemy_id) {
            // AI logic
        }
    }
}
```

---

*See [Runtime API](runtime.md) for `World` methods in `UpdateContext`, or [Game Loop](guides/game-loop.md) for integration with frame loop.*