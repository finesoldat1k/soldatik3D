//! Soldatik3D demo game.
//!
//! This crate depends only on the public `engine` facade. It owns all gameplay
//! code: the procedural level, first-person controller and debug interface.

pub mod debug_ui;
pub mod demo;
mod player;
pub mod world;

pub use demo::DemoGame;

/// Runs the demo through the engine runtime.
pub fn run() -> anyhow::Result<()> {
    engine::run(DemoGame::new())
}
