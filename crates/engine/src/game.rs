use engine_core::{Camera, DebugView, RenderStats, Scene, UiDrawList};
use engine_platform::Input;
use engine_render::BackendKind;

/// Window and runtime options requested by a game.
#[derive(Clone, Debug)]
pub struct GameConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Try to capture and hide the cursor when the window opens.
    pub cursor_locked: bool,
    /// Graphics backend used to present frames. Vulkan is the default and the
    /// only backend shipped today; future backends are selected here.
    pub backend: BackendKind,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: "Soldatik3D".to_owned(),
            width: 1280,
            height: 720,
            cursor_locked: true,
            backend: BackendKind::Vulkan,
        }
    }
}

impl GameConfig {
    pub fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backend = backend;
        self
    }
}

/// Per-frame state handed to [`Game::update`].
pub struct UpdateContext<'a> {
    /// Time since the previous frame, in seconds. Clamped to avoid huge steps
    /// after the window was suspended.
    pub dt: f32,
    /// Keyboard, mouse and cursor state for this frame.
    pub input: &'a Input,
    /// Immediate-mode UI draw list, in physical pixels. The engine clears it
    /// before every update and uploads it after the update returns.
    pub ui: &'a mut UiDrawList,
    /// Statistics of the previous rendered frame.
    pub stats: RenderStats,
    /// Current framebuffer size in physical pixels.
    pub viewport: (u32, u32),
}

/// A game built on top of the engine.
///
/// The engine owns the window, renderer and input. The game owns gameplay
/// state, the camera used for rendering and the scene description.
pub trait Game {
    /// Options used when the window is created.
    fn config(&self) -> GameConfig;

    /// Called once when the renderer is created. The returned scene is
    /// uploaded to the GPU and remains immutable for the lifetime of the
    /// renderer.
    fn create_scene(&mut self) -> Scene;

    /// Camera used to render the current frame.
    fn camera(&self) -> Camera;

    /// Debug visualization of the forward pass.
    fn debug_view(&self) -> DebugView {
        DebugView::Lit
    }

    /// Advances gameplay by one frame and optionally fills the UI draw list.
    fn update(&mut self, ctx: &mut UpdateContext<'_>);
}
