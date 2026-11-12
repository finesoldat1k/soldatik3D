use anyhow::Result;
use winit::window::Window;
use crate::types::{Vertex, Collider}; // Общие типы

pub trait Renderer {
    /// Создать рендерер для окна. `initial_geometry` — вершины мира.
    fn new(window: &Window, initial_geometry: &[Vertex]) -> Result<Self>
    where
        Self: Sized;

    /// Отрисовать кадр.
    fn render_frame(
        &mut self,
        camera_view: &glam::Mat4,
        colliders: &[Collider],
        debug_menu: &mut DebugMenuData, // Данные для UI
    ) -> Result<()>;

    /// Обработать изменение размера окна.
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;

    /// Дождаться завершения работы GPU.
    fn wait_idle(&self) -> Result<()>;
}

/// Данные, которые игра передаёт в UI рендерера.
pub struct DebugMenuData {
    pub fps: f32,
    pub frame_time: f32,
    pub mode: u32,
    // ... другие поля
}