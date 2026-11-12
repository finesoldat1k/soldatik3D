//! Public facade and runtime of the Soldatik3D engine.
//!
//! The engine owns the platform window, the event loop, input state and a
//! renderer selected through [`GameConfig::backend`]. A game only implements
//! [`Game`] and describes its world with the platform-independent types
//! re-exported from `engine-core`.
//!
//! ```no_run
//! use engine::{run, Camera, Game, GameConfig, Scene, UpdateContext};
//!
//! struct MyGame;
//!
//! impl Game for MyGame {
//!     fn config(&self) -> GameConfig {
//!         GameConfig::default()
//!     }
//!
//!     fn create_scene(&mut self) -> Scene {
//!         Scene::new(Default::default())
//!     }
//!
//!     fn camera(&self) -> Camera {
//!         Camera::default()
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
pub mod game;

pub use engine_core::glam;
pub use engine_core::prelude::*;
pub use engine_core::{Camera, Scene, UiDrawList, UiVertex, font_atlas_rgba8};
pub use engine_platform::{Input, winit};
pub use engine_render::{BackendKind, Renderer, WindowSurface};
pub use game::{Game, GameConfig, UpdateContext};

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
