//! OpenGL display, context and window-surface bootstrap.
//!
//! The engine runtime already owns the winit window. This module attaches a
//! glutin (EGL/GLX/WGL/CGL) OpenGL 3.3 core context to that existing native
//! handle and exposes a loaded `glow` function table.

use std::num::NonZeroU32;

use anyhow::{Context, Result};
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{
    ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext, PossiblyCurrentContext,
    Version,
};
use glutin::display::{Display, DisplayApiPreference, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use render::{VsyncMode, WindowSurface as EngineSurface};

/// A current OpenGL context plus the loaded function table.
pub struct GlContext {
    display: Display,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    glow: glow::Context,
}

impl GlContext {
    pub fn new(surface: EngineSurface) -> Result<Self> {
        if surface.is_empty() {
            anyhow::bail!("cannot create an OpenGL context for a zero-sized window");
        }

        let display = unsafe { Display::new(surface.display, display_preference()) }
            .context("failed to open an OpenGL display (EGL/GLX)")?;

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_depth_size(24)
            .with_stencil_size(0)
            .with_transparency(false)
            .build();
        let config = unsafe { display.find_configs(template) }
            .context("failed to enumerate OpenGL framebuffer configurations")?
            .next()
            .context("no OpenGL framebuffer configuration supports a window surface")?;

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .with_profile(GlProfile::Core)
            .build(Some(surface.window));
        let not_current = unsafe { display.create_context(&config, &context_attributes) }
            .context("failed to create an OpenGL 3.3 core context")?;

        let width = NonZeroU32::new(surface.width.max(1)).expect("non-zero width");
        let height = NonZeroU32::new(surface.height.max(1)).expect("non-zero height");
        let attributes =
            SurfaceAttributesBuilder::<WindowSurface>::new().build(surface.window, width, height);
        let window_surface = unsafe { display.create_window_surface(&config, &attributes) }
            .context("failed to create the OpenGL window surface")?;
        let context = not_current
            .make_current(&window_surface)
            .context("failed to make the OpenGL context current")?;

        if let Some(interval) = NonZeroU32::new(1) {
            let _ = window_surface.set_swap_interval(&context, SwapInterval::Wait(interval));
        }

        let glow = unsafe {
            glow::Context::from_loader_function_cstr(|name| display.get_proc_address(name))
        };

        Ok(Self {
            display,
            surface: window_surface,
            context,
            glow,
        })
    }

    pub fn glow(&self) -> &glow::Context {
        &self.glow
    }

    pub fn version(&self) -> String {
        self.display.version_string()
    }

    pub fn resize(&self, width: u32, height: u32) -> Result<()> {
        if let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(&self.context, width, height);
        }
        Ok(())
    }

    pub fn swap_buffers(&self) -> Result<()> {
        self.surface
            .swap_buffers(&self.context)
            .context("failed to present the OpenGL framebuffer")
    }

    pub fn set_vsync(&self, vsync: VsyncMode) -> Result<()> {
        use glutin::surface::SwapInterval;
        use std::num::NonZeroU32;

        let interval = match vsync {
            VsyncMode::Off => SwapInterval::DontWait,
            VsyncMode::On => SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
            VsyncMode::Adaptive => SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
            VsyncMode::Mailbox => SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
            _ => SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
        };

        let _ = self.surface.set_swap_interval(&self.context, interval);
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn display_preference() -> DisplayApiPreference {
    // EGL is the standard path on both Wayland and modern X11. GLX remains a
    // fallback for older X11 setups.
    DisplayApiPreference::EglThenGlx(Box::new(|_hook| {}))
}

#[cfg(target_os = "windows")]
fn display_preference() -> DisplayApiPreference {
    DisplayApiPreference::EglThenWgl(None)
}

#[cfg(target_os = "macos")]
fn display_preference() -> DisplayApiPreference {
    DisplayApiPreference::Cgl
}
