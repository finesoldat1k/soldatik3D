//! Soldatik3D demo game.
//!
//! This crate depends only on the public `runtime` facade. It owns all gameplay
//! code: the procedural level, first-person controller, camera presets and
//! debug interface. Camera spawning and selection are engine features exposed
//! through [`runtime::World`] and the console.

pub mod debug_ui;
pub mod demo;
mod player;
pub mod world;

pub use demo::DemoGame;

/// Runs the demo through the engine runtime.
pub fn run() -> anyhow::Result<()> {
    runtime::run(DemoGame::new())
}
