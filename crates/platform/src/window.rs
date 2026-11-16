use anyhow::{Context, Result};
use render::WindowSurface;
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{CursorGrabMode, Window, WindowId};

/// Owns the platform window and provides renderer-neutral surface handles.
///
/// The renderer is always destroyed before this value is dropped by the engine
/// runtime, so the native surface outlives every Vulkan/OpenGL object.
pub struct PlatformWindow {
    window: Window,
}

impl PlatformWindow {
    pub fn new(
        event_loop: &ActiveEventLoop,
        title: &str,
        width: u32,
        height: u32,
        fullscreen: bool,
    ) -> Result<Self> {
        let attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(LogicalSize::new(width, height));
        let attributes = if fullscreen {
            attributes.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        } else {
            attributes
        };
        let window = event_loop
            .create_window(attributes)
            .context("failed to create the platform window")?;
        Ok(Self { window })
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn inner_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    /// Physical pixels per logical pixel for the current display.
    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    /// Returns the raw handles and current logical size for a renderer backend.
    pub fn surface(&self) -> Result<WindowSurface> {
        let size = self.window.inner_size();
        Ok(WindowSurface::new(
            self.window.window_handle()?.as_raw(),
            self.window.display_handle()?.as_raw(),
            size.width,
            size.height,
        ))
    }

    /// Tries to lock the cursor and returns whether it actually succeeded.
    ///
    /// Wayland compositors differ in which grab mode they support, so the
    /// engine falls back to a confined cursor before giving up on capture.
    pub fn set_cursor_captured(&self, captured: bool) -> bool {
        let actual = if captured {
            match self.window.set_cursor_grab(CursorGrabMode::Locked) {
                Ok(()) => true,
                Err(_) => self
                    .window
                    .set_cursor_grab(CursorGrabMode::Confined)
                    .is_ok(),
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            false
        };
        self.window.set_cursor_visible(!actual);
        actual
    }
}
