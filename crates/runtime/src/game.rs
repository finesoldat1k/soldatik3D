use platform::{Input, InputMap};
use render::{BackendKind, DebugView, RenderStats, VsyncMode};
use scene::World;
use ui::UiDrawList;

/// Window and runtime options requested by a game.
#[derive(Clone, Debug)]
pub struct GameConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Try to capture and hide the cursor when the window opens.
    pub cursor_locked: bool,
    /// Graphics backend used to present frames. `SOLDATIK3D_BACKEND` overrides
    /// the default at startup (`vulkan` or `opengl`).
    pub backend: BackendKind,
    /// Vertical sync mode. `SOLDATIK3D_VSYNC` overrides the default at startup.
    pub vsync: VsyncMode,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: "Soldatik3D".to_owned(),
            width: 1280,
            height: 720,
            cursor_locked: true,
            backend: BackendKind::from_env().unwrap_or_default(),
            vsync: VsyncMode::from_env().unwrap_or_default(),
        }
    }
}

impl GameConfig {
    pub fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backend = backend;
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_cursor_locked(mut self, locked: bool) -> Self {
        self.cursor_locked = locked;
        self
    }

    pub fn with_vsync(mut self, vsync: VsyncMode) -> Self {
        self.vsync = vsync;
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
    /// before every update and uploads it after the game and console have run.
    pub ui: &'a mut UiDrawList,
    /// Statistics of the previous rendered frame.
    pub stats: RenderStats,
    /// Current framebuffer size in physical pixels.
    pub viewport: (u32, u32),
    /// Engine-owned world. Camera spawning, selection and movement happen
    /// here; the runtime renders through `world.active_camera()`.
    pub world: &'a mut World,
    /// Whether the engine console currently owns text input.
    pub console_open: bool,
}

/// Context for a game-defined console command.
pub struct CommandContext<'a> {
    /// Command name without arguments, for example `player.teleport`.
    pub name: &'a str,
    /// Whitespace-separated arguments after the command name.
    pub args: &'a [&'a str],
    /// Engine world, so commands can spawn/select cameras or inspect the scene.
    pub world: &'a mut World,
}

/// A game built on top of the engine.
///
/// The engine owns the window, renderer, input and console. The game builds a
/// [`World`] once (or rebuilds it explicitly) and mutates it every frame. The
/// runtime renders through the world's active camera, so camera management is
/// centralized instead of reimplemented by each game.
pub trait Game {
    /// Options used when the window is created.
    fn config(&self) -> GameConfig;

    /// Called once when the renderer is created. The returned world's scene is
    /// uploaded to the GPU and remains immutable for the renderer's lifetime.
    fn create_world(&mut self) -> World;

    /// Debug visualization of the forward pass.
    fn debug_view(&self) -> DebugView {
        DebugView::Lit
    }

    /// Advances gameplay by one frame.
    fn update(&mut self, ctx: &mut UpdateContext<'_>);

    /// Returns the action map for input bindings. Default returns an empty map.
    ///
    /// Games should override this to provide their action bindings. The map is
    /// applied to the input state once at startup.
    fn input_map(&self) -> InputMap {
        InputMap::new()
    }

    /// Descriptions of game-specific console commands, shown by `help`.
    ///
    /// Each entry is `(command, description)`. The engine advertises these
    /// alongside its built-in camera and diagnostics commands.
    fn command_help(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// Handles a game-specific console command.
    ///
    /// Built-in commands (`camera.*`, `help`, `clear`, `quit`, `backend`)
    /// are handled by the engine before this method is called. Return
    /// `Some(Ok(message))` to print a response, `Some(Err(message))` to print
    /// an error, or `None` to let the console report an unknown command.
    fn command(&mut self, _ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
        None
    }
}
