//! Backend factory of the engine runtime.
//!
//! This is the only place in the facade that knows concrete backend crates.
//! Adding OpenGL later means adding one arm here and one enum variant in
//! `render::BackendKind`; games keep using [`render::Renderer`].

use anyhow::Result;
use render::{BackendKind, Renderer, VsyncMode, WindowSurface};
use scene::Scene;

pub(crate) fn create_renderer(
    kind: BackendKind,
    surface: WindowSurface,
    scene: &Scene,
    vsync: VsyncMode,
) -> Result<Box<dyn Renderer>> {
    match kind {
        #[cfg(all(feature = "vulkan", not(target_os = "android")))]
        BackendKind::Vulkan => Ok(Box::new(vulkan::VulkanRenderer::new(
            surface, scene, vsync,
        )?)),
        #[cfg(feature = "opengl")]
        BackendKind::OpenGl => Ok(Box::new(opengl::OpenGlRenderer::new(
            surface, scene, vsync,
        )?)),
        // Reached when the selected backend is not compiled into this build.
        // When every variant is enabled the arm is unreachable, hence the allow.
        #[allow(unreachable_patterns)]
        backend => {
            let _ = (surface, scene, vsync);
            anyhow::bail!(
                "the `{}` backend is not available in this build; enable its engine feature",
                backend.name()
            )
        }
    }
}
