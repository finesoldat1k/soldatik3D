use anyhow::Result;
use ash::{khr, vk, Entry, Instance};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

pub fn create_surface(
    entry: &Entry,
    instance: &Instance,
    window: &Window,
) -> Result<(khr::surface::Instance, vk::SurfaceKHR)> {
    let surface = unsafe {
        ash_window::create_surface(
            entry,
            instance,
            window.display_handle()?.as_raw(),
            window.window_handle()?.as_raw(),
            None,
        )?
    };
    let surface_loader = khr::surface::Instance::new(entry, instance);
    Ok((surface_loader, surface))
}
