# Scene API

The `scene` crate is platform- and GPU-independent. It describes geometry,
lights, a material and named cameras. `runtime` re-exports every public scene
type.

```rust
use runtime::{
    Aabb, Camera, CameraId, CameraRig, Light, Material, Mesh, NamedCamera,
    PrimaryLight, Scene, ShadowSettings, Vertex, World,
};
use runtime::glam::{Mat4, Vec2, Vec3, Vec4};
```

## `Scene`

The immutable GPU-side description of a level.

```rust
pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub lights: Vec<Light>,
    pub primary_light: PrimaryLight,
    pub material: Material,
    pub clear_color: [f32; 4],
}
```

| Method | Description |
|--------|-------------|
| `Scene::new(material)` | Empty scene with one material |
| `with_meshes(Vec<Mesh>)` | Replace the mesh list |
| `with_lights(Vec<Light>)` | Replace local lights |
| `with_primary_light(PrimaryLight)` | Set the shadow-casting light |
| `with_clear_color([f32; 4])` | Background color |
| `add_mesh(Mesh)` | Append one mesh |
| `add_light(Light)` | Append one light |

`Scene::default()` has no meshes, one default primary light, a default material
and a black clear color.

The renderer uploads the scene once. Changing `Scene` after the renderer is
created does not re-upload geometry or lights; see
[Static scene, dynamic camera](../guides/architecture.md#static-scene-dynamic-camera).

```rust
let scene = Scene::new(Material {
        albedo: TextureData::white(),
        metallic: 0.0,
        roughness: 0.8,
    })
    .with_meshes(level_meshes())
    .with_lights(vec![
        Light::new(Vec3::new(4.0, 3.0, 2.0), Vec3::new(1.0, 0.7, 0.4), 12.0),
    ])
    .with_primary_light(PrimaryLight::default())
    .with_clear_color([0.01, 0.02, 0.04, 1.0]);
```

## `Mesh`, `Vertex`, `Aabb`

Geometry is a flat vertex list. There are no index buffers and no primitive
types; each `Mesh` is one draw.

```rust
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub bounds: Aabb,
}
```

```rust
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub emission: f32,
}
```

| Method | Description |
|--------|-------------|
| `Vertex::new(position, color, normal, uv)` | Vertex with `emission = 0.0` |
| `with_emission(f32)` | Emissive contribution |
| `Mesh::new(vertices, bounds)` | Build a mesh |
| `Aabb::new(min, max)` | Axis-aligned bounds |
| `Aabb::from_center_half_extents(center, half)` | Convenience constructor |
| `Aabb::ZERO` | Degenerate bounds |
| `center()`, `half_extents()`, `overlaps(&other)` | Queries |

`bounds` is used for camera and shadow frustum culling. Wrong bounds cause
meshes to disappear at certain angles. A mesh whose bounds are always visible
can use a large `Aabb`.

Vertices are in world space: the engine has no per-mesh transform.

```rust
fn triangle() -> Mesh {
    let vertices = vec![
        Vertex::new([0.0, 0.5, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.5, 1.0]),
        Vertex::new([-0.5, -0.5, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
        Vertex::new([0.5, -0.5, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
    ];
    Mesh::new(
        vertices,
        Aabb::new(Vec3::new(-0.5, -0.5, -0.01), Vec3::new(0.5, 0.5, 0.01)),
    )
}
```

Triangle winding must match the backend's front-face convention (counter-
clockwise for the standard pipeline). Back-face culled geometry with reversed
winding will be invisible.

## `Material` and `TextureData`

```rust
pub struct Material {
    pub albedo: TextureData,
    pub metallic: f32,
    pub roughness: f32,
}
```

One material is shared by every mesh in the scene. `Material::default()` uses a
white albedo, `metallic = 0.0`, `roughness = 0.9`.

`TextureData` holds RGBA8 pixels:

```rust
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
```

| Constructor | Description |
|-------------|-------------|
| `TextureData::new(w, h, pixels)` | Asserts `pixels.len() == w*h*4` |
| `TextureData::white()` | 1×1 opaque white |
| `TextureData::from_rgb8(w, h, &rgb)` | Expands RGB8 to RGBA8 |

See [Assets](assets.md) for the font atlas and environment cubemap.

## `Light`

Up to eight local point lights are uploaded per frame, chosen by distance to
the camera. `MAX_LIGHTS` is 8.

```rust
pub struct Light {
    pub position: Vec3,
    pub color: Vec3,
    pub radius: f32,
    pub intensity: f32,
}
```

```rust
let light = Light::new(Vec3::new(2.0, 3.0, 1.0), Vec3::new(1.0, 0.9, 0.7), 8.0)
    .with_intensity(2.5);
```

`radius` controls attenuation. Lights beyond the eight nearest are ignored.

## `PrimaryLight` and `ShadowSettings`

There is exactly one shadow-casting light. It also acts as the primary direct
light of the PBR shader.

```rust
pub struct PrimaryLight {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub shadow: ShadowSettings,
}

pub struct ShadowSettings {
    pub ortho_half_extent: f32,
    pub near: f32,
    pub far: f32,
}
```

Defaults: position `(0, 10, 0)`, target `ZERO`, up `Z`, white, intensity 1.0,
orthographic half extent 50.0, near 0.1, far 100.0.

The shadow map is 2048×2048 with PCF filtering. The light frustum is
orthographic, so cover the playable area with `ortho_half_extent`; too small a
value clips shadows, too large a value reduces resolution.

## `Camera`

```rust
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,           // radians; 0 looks down -Z
    pub pitch: f32,         // radians
    pub fov_y_degrees: f32,
    pub z_near: f32,
    pub z_far: f32,
}
```

Default: position `ZERO`, yaw −90°, pitch 0, FOV 60°, range 0.1–100.

| Method | Description |
|--------|-------------|
| `Camera::at(position)` | Camera at a position looking down −Z |
| `with_yaw_degrees(f32)` | Set yaw |
| `with_pitch_degrees(f32)` | Set pitch |
| `with_fov_degrees(f32)` | Vertical field of view |
| `with_range(near, far)` | Clip planes |
| `forward()` | Unit direction including pitch |
| `forward_flat()` | Horizontal direction |
| `right_flat()` | Horizontal right vector |
| `view_matrix()` | View matrix |
| `projection_matrix(aspect)` | Perspective projection |

The camera contains no movement code. Games integrate position and look input
themselves.

```rust
let camera = Camera::at(Vec3::new(0.0, 1.7, 6.0))
    .with_yaw_degrees(0.0)
    .with_pitch_degrees(-5.0)
    .with_fov_degrees(70.0)
    .with_range(0.05, 250.0);
```

## `CameraRig`, `CameraId`, `NamedCamera`

```rust
pub struct CameraRig { /* private */ }

pub struct NamedCamera {
    pub id: CameraId,
    pub name: String,
    pub camera: Camera,
}

pub struct CameraId(u32);
```

`CameraId` implements `Display`; `get()` and `from_raw()` convert to and from
`u32`.

| Method | Description |
|--------|-------------|
| `CameraRig::new()` | Rig with a default `main` camera |
| `CameraRig::empty()` | Rig with no cameras |
| `spawn(name, camera)` | Add and return a new ID |
| `add(camera)` | Add with an automatic name |
| `remove(id)` | Remove and return the camera |
| `select(id)` / `select_name(name)` | Select an active camera |
| `select_next()` / `select_previous()` | Cycle in insertion order |
| `active()`, `active_mut()`, `active_id()`, `active_name()` | Active camera access |
| `rename_active(name)` | Rename the active camera |
| `get(id)`, `get_mut(id)`, `get_by_name(name)` | Lookup |
| `iter()`, `len()`, `is_empty()` | Iteration and counts |

`World` exposes the same camera operations directly; use it from gameplay code.
Use `CameraRig::empty()` when you want to create exactly your own cameras.

```rust
let mut world = World::default();
let player = world.spawn_camera("player", Camera::at(Vec3::new(0.0, 1.7, 5.0)));
world.spawn_camera("overview", Camera::at(Vec3::new(0.0, 20.0, 20.0)));
assert!(world.select_camera(player));
assert_eq!(world.active_camera_name(), "player");
```

## Related

- [Runtime API](runtime.md)
- [Assets API](assets.md)
- [Game loop](../guides/game-loop.md)
