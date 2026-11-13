//! Window, input and event integration for the Soldatik3D engine.
//!
//! This crate owns the direct `winit` dependency. It turns a live window into
//! the renderer-neutral [`render::WindowSurface`] value and exposes an
//! aggregated [`Input`] snapshot to games. The engine runtime drives the winit
//! event loop through the re-exported module, but games never depend on winit
//! directly.

pub mod input;
pub mod platform;
pub mod window;

pub use input::{Input, InputMap};
pub use platform::{
    PlatformCapabilities, TargetOS, TargetArch, WindowHints,
};
pub use window::PlatformWindow;

/// Re-exported so games can use key codes, mouse buttons and window types
/// through the `runtime` facade without depending on winit themselves.
pub use winit;
