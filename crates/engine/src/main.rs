use anyhow::Result;
use winit::event_loop::EventLoop;

mod app;
mod camera;
mod ui;
mod vulkan;
mod world;

fn main() -> Result<()> {
    println!("Soldatik3D v{}", env!("CARGO_PKG_VERSION"));
    let event_loop = EventLoop::new()?;
    let mut app = app::App::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}