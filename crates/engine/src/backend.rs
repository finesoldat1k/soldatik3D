//! Backend factory of the engine runtime.
//!
//! This is the only place in the facade that knows concrete backend crates.
//! Adding OpenGL later means adding one arm here and one enum variant in
//! `engine_render::BackendKind`; games keep using [`engine_render::Renderer`].

use anyhow::Result;
use engine_core::Scene;
use engine_render::{BackendKind, Renderer, WindowSurface};

pub(crate) fn create_renderer(
    kind: BackendKind,
    surface: WindowSurface,
    scene: &Scene,
) -> Result<Box<dyn Renderer>> {
    match kind {
        #[cfg(feature = "vulkan")]
        BackendKind::Vulkan => Ok(Box::new(engine_vulkan::VulkanRenderer::new(
            surface, scene,
        )?)),
        // Reached when the selected backend is not compiled into this build.
        // When every variant is enabled the arm is unreachable, hence the allow.
        #[allow(unreachable_patterns)]
        backend => {
            let _ = (surface, scene);
            anyhow::bail!(
                "the `{}` backend is not available in this build; enable its engine feature",
                backend.name()
            )
        }
    }
}
