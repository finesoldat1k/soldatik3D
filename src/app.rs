use anyhow::Result;
use std::sync::Arc;
use std::collections::HashSet;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent, DeviceEvent, MouseButton},
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId, CursorGrabMode},
};

use crate::{camera::Camera, vulkan::VulkanContext, world, ui::DebugMenu};

pub struct App {
    context: Option<VulkanContext>,
    window: Option<Arc<Window>>,
    camera: Camera,
    keys: HashSet<KeyCode>,
    last_frame: std::time::Instant,
    debug_menu: DebugMenu,
    mouse_captured: bool,
    fps_timer: std::time::Instant,
    frames: u32,
    mouse_x: f64,
    mouse_y: f64,
    colliders: Vec<world::Collider>,
    ui_update_counter: u32,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            context: None,
            window: None,
            camera: Camera::new(),
            keys: HashSet::new(),
            last_frame: std::time::Instant::now(),
            debug_menu: DebugMenu::new(),
            mouse_captured: true,
            fps_timer: std::time::Instant::now(),
            frames: 0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            colliders: Vec::new(),
            ui_update_counter: 0,
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes().with_title("Soldatik3D").with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
            ).expect("Failed to create window"),
        );
        let _ = window.set_cursor_grab(CursorGrabMode::Locked);
        window.set_cursor_visible(false);

        // ИСПРАВЛЕНИЕ: Получаем чанки и коллайдеры отдельно
        let (room_chunks, all_colliders) = world::create_room_vertices();
        self.colliders = all_colliders;

        match VulkanContext::new(&window, &room_chunks) {
            Ok(ctx) => {
                self.context = Some(ctx);
                self.window = Some(window);
                event_loop.set_control_flow(ControlFlow::Poll);
                self.last_frame = std::time::Instant::now();
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            Err(e) => { eprintln!("Failed to initialize Vulkan: {e:?}"); event_loop.exit(); }
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: winit::event::DeviceId, event: DeviceEvent) {
        if self.mouse_captured {
            if let DeviceEvent::MouseMotion { delta } = event {
                self.camera.process_mouse(delta.0 as f32, delta.1 as f32);
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => { self.context = None; self.window = None; event_loop.exit(); }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x;
                self.mouse_y = position.y;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed && button == MouseButton::Left {
                    if !self.mouse_captured && self.debug_menu.is_open {
                        self.debug_menu.check_click(self.mouse_x as f32, self.mouse_y as f32);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        match code {
                            KeyCode::Escape => { self.context = None; self.window = None; event_loop.exit(); }
                            KeyCode::F1 => { self.debug_menu.is_open = !self.debug_menu.is_open; }
                            KeyCode::Tab => {
                                self.mouse_captured = !self.mouse_captured;
                                if let Some(w) = &self.window {
                                    if self.mouse_captured {
                                        let _ = w.set_cursor_grab(CursorGrabMode::Locked);
                                        w.set_cursor_visible(false);
                                    } else {
                                        let _ = w.set_cursor_grab(CursorGrabMode::None);
                                        w.set_cursor_visible(true);
                                    }
                                }
                            }
                            KeyCode::F2 => self.debug_menu.mode = 0,
                            KeyCode::F3 => self.debug_menu.mode = 1,
                            KeyCode::F4 => self.debug_menu.mode = 2,
                            KeyCode::KeyF => {
                                self.camera.is_flying = !self.camera.is_flying;
                                self.camera.velocity = glam::Vec3::ZERO;
                            }
                            _ => { self.keys.insert(code); }
                        }
                    } else if event.state == ElementState::Released {
                        self.keys.remove(&code);
                    }
                }
            }
            WindowEvent::Resized(_) => {
                if let (Some(window), Some(ctx)) = (&self.window, &mut self.context) {
                    if let Err(e) = ctx.recreate_swapchain(window) { eprintln!("Swapchain recreate failed: {e:?}"); }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                self.frames += 1;
                self.debug_menu.frame_time = dt * 1000.0;
                
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    self.debug_menu.fps = self.frames as f32;
                    self.frames = 0;
                    self.fps_timer = std::time::Instant::now();
                }

                if self.ui_update_counter % 5 == 0 {
                    self.debug_menu.camera_pos = [self.camera.position.x, self.camera.position.y, self.camera.position.z];
                    self.debug_menu.vertices = if let Some(ctx) = &self.context { ctx.get_vertex_count() } else { 0 };
                    self.debug_menu.triangles = self.debug_menu.vertices / 3;
                }
                self.ui_update_counter = self.ui_update_counter.wrapping_add(1);

                self.camera.update(dt, &self.keys, &self.colliders);

                if let (Some(window), Some(ctx)) = (&self.window, &mut self.context) {
                    if self.ui_update_counter % 5 == 0 {
                        ctx.update_ui(&self.debug_menu);
                    }
                    if let Err(e) = ctx.draw_frame(window, &self.camera, self.debug_menu.mode, self.camera.current_fov) {
                        eprintln!("Draw error: {e:?}");
                    }
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}
