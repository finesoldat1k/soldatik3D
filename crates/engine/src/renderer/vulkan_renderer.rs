// engine/src/renderer/vulkan_renderer.rs
use super::{Renderer, DebugMenuData};
use crate::vulkan::VulkanContext; // Твой текущий контекст
use crate::types::{Vertex, Collider};
use anyhow::Result;
use winit::window::Window;

pub struct VulkanRenderer {
    context: VulkanContext,
    // Возможно, кэш для UI-буферов и т.д.
}

impl Renderer for VulkanRenderer {
    fn new(window: &Window, initial_geometry: &[Vertex]) -> Result<Self> {
        let context = VulkanContext::new(window, initial_geometry)?;
        Ok(Self { context })
    }

    fn render_frame(
        &mut self,
        camera_view: &glam::Mat4,
        colliders: &[Collider],
        debug_data: &mut DebugMenuData,
    ) -> Result<()> {
        // Здесь вызываешь методы своего VulkanContext, передавая view-матрицу и данные.
        // Например: self.context.render(camera_view, colliders, debug_data);
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.context.recreate_swapchain(width, height) // если такой метод есть
    }

    fn wait_idle(&self) -> Result<()> {
        unsafe { self.context.device().device_wait_idle()? };
        Ok(())
    }
}
