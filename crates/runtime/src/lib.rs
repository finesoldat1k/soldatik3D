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

pub use assets::{
    ENVIRONMENT_SIZE, IRRADIANCE_SIZE, PREFILTERED_MIP_LEVELS, PREFILTERED_SIZE, TextureData,
    font_atlas_rgba8, generate_environment_cubemap, generate_environment_cubemap_with_sun,
    generate_irradiance_cubemap, generate_prefiltered_cubemap,
};
pub use game::{CommandContext, Game, GameConfig, UpdateContext};
pub use platform::{
    Input, InputMap, PlatformCapabilities, TargetArch, TargetOS, WindowHints, winit,
};
pub use render::{
    BackendKind, CameraFrame, DebugView, FrameData, RenderStats, Renderer, VsyncMode, WindowSurface,
};
pub use scene::glam;
pub use scene::prelude::*;
pub use scene::{Camera, CameraId, CameraRig, NamedCamera, Scene, World};
pub use ui;
pub use ui::{Ui, UiDrawList, UiVertex};

use anyhow::Result;

/// Creates the platform event loop and runs `game` until the window is closed.
///
/// This function must be called from the main thread.
pub fn run<G>(game: G) -> Result<()>
where
    G: Game + 'static,
{
    let args: Vec<String> = std::env::args().collect();
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::new(game, &args);
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// Runs `game` from an Android `android_main` entry point.
///
/// Android has no global winit application object: the [`AndroidApp`] supplied
/// by the `android-activity` glue must be attached to the event loop builder.
/// A game compiled as a native-activity `cdylib` exports `android_main` and
/// forwards the app handle here.
///
/// [`AndroidApp`]: winit::platform::android::activity::AndroidApp
#[cfg(target_os = "android")]
pub fn run_android<G>(
    game: G,
    android_app: winit::platform::android::activity::AndroidApp,
) -> Result<()>
where
    G: Game + 'static,
{
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let args: Vec<String> = Vec::new(); // Android doesn't use command-line args
    let event_loop = winit::event_loop::EventLoop::builder()
        .with_android_app(android_app)
        .build()?;
    let mut app = app::App::new(game, &args);
    event_loop.run_app(&mut app)?;
    Ok(())
}
