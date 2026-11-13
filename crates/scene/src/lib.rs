//! Platform-independent world model of the Soldatik3D engine.
//!
//! This crate owns cameras, the camera rig, meshes, lights, materials and the
//! [`World`] container. It has no GPU or windowing dependency: backends decide
//! *how* to draw, the scene describes *what* exists.
//!
//! The central camera API lives in [`CameraRig`]:
//!
//! ```
//! use scene::{Camera, World};
//! use scene::glam::Vec3;
//!
//! let mut world = World::default();
//! let id = world.spawn_camera("pool", Camera::at(Vec3::new(70.0, -3.0, 60.0)));
//! assert!(world.select_camera(id));
//! world.active_camera_mut().unwrap().camera.yaw = 0.0;
//! ```

pub use glam;

pub mod camera;
pub mod camera_rig;
pub mod geometry;
pub mod light;
pub mod material;
pub mod scene;
pub mod world;

pub use assets::TextureData;
pub use camera::Camera;
pub use camera_rig::{CameraId, CameraRig, NamedCamera};
pub use geometry::{Aabb, Mesh, Vertex};
pub use light::{Light, LightUniformBuffer, MAX_LIGHTS, PrimaryLight, ShadowSettings};
pub use material::Material;
pub use scene::Scene;
pub use world::World;

/// The most commonly used world types.
pub mod prelude {
    pub use crate::TextureData;
    pub use crate::camera::Camera;
    pub use crate::camera_rig::{CameraId, CameraRig, NamedCamera};
    pub use crate::geometry::{Aabb, Mesh, Vertex};
    pub use crate::light::{Light, LightUniformBuffer, MAX_LIGHTS, PrimaryLight, ShadowSettings};
    pub use crate::material::Material;
    pub use crate::scene::Scene;
    pub use crate::world::World;
    pub use glam::{Mat4, Vec2, Vec3, Vec4};
}
