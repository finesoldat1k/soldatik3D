//! OpenGL 3.3 core renderer backend for the Soldatik3D engine.
//!
//! The crate attaches a glutin context to the engine window and implements the
//! backend-independent [`render::Renderer`] contract with glow. It does not own
//! the window or the event loop.

mod context;
mod renderer;
mod shader;

pub use renderer::OpenGlRenderer;
