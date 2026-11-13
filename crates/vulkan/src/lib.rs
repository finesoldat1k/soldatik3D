//! Vulkan 1.2 renderer backend for the Soldatik3D engine.
//!
//! This crate owns every Vulkan object and translates the platform-independent
//! [`Scene`](::scene::Scene) / [`render::FrameData`] into GPU work. It does not
//! know about winit: the engine runtime passes a [`render::WindowSurface`]
//! containing raw native handles.

#![cfg(not(target_os = "android"))]

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
mod swapchain;
mod sync;

pub use render::{BackendKind, Renderer, WindowSurface};
pub use renderer::VulkanRenderer;
