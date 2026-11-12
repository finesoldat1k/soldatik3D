//! Platform-independent types shared by the engine and the renderer backend.
//!
//! This crate must not depend on Vulkan, winit or any other platform API.
//! Everything here describes *what* should be rendered, never *how*.

pub use glam;

pub mod camera;
pub mod geometry;
pub mod light;
pub mod material;
pub mod render;
pub mod scene;
pub mod ui;

pub use camera::Camera;
pub use geometry::{Aabb, Mesh, Vertex};
pub use light::{Light, MAX_LIGHTS, PrimaryLight, ShadowSettings};
pub use material::{Material, TextureData};
pub use render::{CameraFrame, DebugView, FrameData, RenderStats};
pub use scene::Scene;
pub use ui::{UiDrawList, UiVertex, font_atlas_rgba8};

/// The most commonly used engine types.
pub mod prelude {
    pub use crate::camera::Camera;
    pub use crate::geometry::{Aabb, Mesh, Vertex};
    pub use crate::light::{Light, MAX_LIGHTS, PrimaryLight, ShadowSettings};
    pub use crate::material::{Material, TextureData};
    pub use crate::render::{CameraFrame, DebugView, FrameData, RenderStats};
    pub use crate::scene::Scene;
    pub use crate::ui::{UiDrawList, UiVertex};
    pub use glam::{Mat4, Vec2, Vec3, Vec4};
}
