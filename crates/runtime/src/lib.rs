//! Public facade and runtime of the Soldatik3D engine.
//!
//! The engine owns the platform window, the event loop, input state and a
//! renderer selected through [`GameConfig::backend`]. A game only implements
//! [`Game`] and describes its world with the platform-independent types
//! re-exported from `scene`.
//!
//! ```no_run
//! use runtime::{run, Camera, Game, GameConfig, UpdateContext, World};
//!
//! struct MyGame;
//!
//! impl Game for MyGame {
//!     fn config(&self) -> GameConfig {
//!         GameConfig::default()
//!     }
//!
//!     fn create_world(&mut self) -> World {
//!         World::default()
//!     }
//!
//!     fn update(&mut self, _ctx: &mut UpdateContext<'_>) {}
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     run(MyGame)
//! }
//! ```

mod app;
mod backend;
mod commands;
mod console;
pub mod game;

pub use assets::{TextureData, font_atlas_rgba8, generate_environment_cubemap};
pub use game::{CommandContext, Game, GameConfig, UpdateContext};
pub use platform::{
    Input, InputMap, PlatformCapabilities, TargetOS, TargetArch, WindowHints, winit,
};
pub use render::{
    BackendKind, CameraFrame, DebugView, FrameData, RenderStats, Renderer, VsyncMode, WindowSurface,
};
pub use scene::glam;
pub use scene::prelude::*;
pub use scene::{Camera, CameraId, CameraRig, NamedCamera, Scene, World};
pub use ui::{UiDrawList, UiVertex};

use anyhow::Result;

/// Creates the platform event loop and runs `game` until the window is closed.
///
/// This function must be called from the main thread.
pub fn run<G>(game: G) -> Result<()>
where
    G: Game + 'static,
{
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::new(game);
    event_loop.run_app(&mut app)?;
    Ok(())
}
