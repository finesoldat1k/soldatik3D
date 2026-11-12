//! Vulkan 1.2 renderer backend for the Soldatik3D engine.
//!
//! This crate owns every Vulkan object and translates the platform-independent
//! [`engine_core::Scene`] / [`engine_core::FrameData`] into GPU work. It does
//! not know about winit: the engine runtime passes a
//! [`engine_render::WindowSurface`] containing raw native handles.

mod buffer;
mod commands;
mod context;
mod descriptor;
mod image;
mod memory;
mod pipeline;
mod render_pass;
mod renderer;
mod scene;
mod shader;
mod skybox;
mod swapchain;
mod sync;

pub use engine_render::{BackendKind, Renderer, WindowSurface};
pub use renderer::VulkanRenderer;
