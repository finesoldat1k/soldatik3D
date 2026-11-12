//! Backend-independent rendering interface of the Soldatik3D engine.
//!
//! Games and the engine runtime only talk to [`Renderer`]. A concrete graphics
//! API lives in its own crate (currently `engine-vulkan`) and implements this
//! trait. Adding an OpenGL backend later means adding `engine-opengl` and one
//! variant to [`BackendKind`]; no game code has to change.

use anyhow::Result;
use engine_core::{FrameData, RenderStats};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// Graphics API used to present the engine's frames.
///
/// The enum is non-exhaustive so new backends can be added without breaking
/// downstream code. Games select a backend through `GameConfig::with_backend`
/// rather than matching on this type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BackendKind {
    /// Vulkan 1.2 through the `engine-vulkan` backend.
    #[default]
    Vulkan,
    // Future backends are added here, for example:
    // OpenGl,
}

impl BackendKind {
    /// Human-readable backend name for logs and diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Vulkan => "vulkan",
        }
    }
}

/// Native handles and logical size needed to create a rendering surface.
///
/// The renderer abstraction itself does not depend on a windowing library.
/// `engine-platform` turns a live winit window into this small value, and any
/// backend can create its own surface/context from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowSurface {
    pub window: RawWindowHandle,
    pub display: RawDisplayHandle,
    pub width: u32,
    pub height: u32,
}

impl WindowSurface {
    pub fn new(
        window: RawWindowHandle,
        display: RawDisplayHandle,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            window,
            display,
            width,
            height,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Contract implemented by every graphics backend.
///
/// The renderer owns its swapchain and all GPU resources. The engine runtime
/// owns the window and forwards resize events. Rendering is intentionally
/// explicit: the runtime calls [`Renderer::render`] once per frame.
pub trait Renderer {
    /// Which backend this renderer implements.
    fn backend(&self) -> BackendKind;

    /// Updates the output size. Passing a zero width or height only records the
    /// new size; the swapchain is recreated on the next non-empty resize.
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;

    /// Records and submits one frame.
    fn render(&mut self, frame: &FrameData<'_>) -> Result<()>;

    /// Waits for the GPU to finish. Called before shutdown and destruction.
    fn wait_idle(&self) -> Result<()>;

    /// Statistics of the last submitted frame.
    fn stats(&self) -> RenderStats;

    /// Current framebuffer size in physical pixels.
    fn extent(&self) -> (u32, u32);
}
