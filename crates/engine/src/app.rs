use std::time::Instant;

use engine_core::glam::Vec2;
use engine_core::{CameraFrame, FrameData, UiDrawList};
use engine_platform::winit;
use engine_platform::winit::application::ApplicationHandler;
use engine_platform::winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use engine_platform::winit::event_loop::{ActiveEventLoop, ControlFlow};
use engine_platform::winit::keyboard::{KeyCode, PhysicalKey};
use engine_platform::winit::window::WindowId;
use engine_platform::{Input, PlatformWindow};
use engine_render::Renderer;

use crate::backend;
use crate::game::{Game, GameConfig, UpdateContext};

/// Engine-owned runtime. Games never see winit or Vulkan details directly.
pub(crate) struct App<G: Game> {
    game: G,
    config: GameConfig,

    // The renderer must be dropped before the window that owns its surface.
    renderer: Option<Box<dyn Renderer>>,
    window: Option<PlatformWindow>,

    input: Input,
    ui: Option<UiDrawList>,
    last_frame: Instant,
    consecutive_render_errors: u32,
    frame_budget: Option<u64>,
    frames_rendered: u64,
}

impl<G: Game> App<G> {
    pub(crate) fn new(game: G) -> Self {
        let config = game.config();
        Self {
            game,
            config,
            renderer: None,
            window: None,
            input: Input::default(),
            ui: None,
            last_frame: Instant::now(),
            consecutive_render_errors: 0,
            frame_budget: std::env::var("SOLDATIK3D_MAX_FRAMES")
                .ok()
                .and_then(|value| value.parse().ok()),
            frames_rendered: 0,
        }
    }

    fn set_cursor_captured(&mut self, captured: bool) {
        let Some(window) = &self.window else {
            return;
        };
        let actual = window.set_cursor_captured(captured);
        self.input.set_cursor_captured(actual);
    }

    fn toggle_cursor_capture(&mut self) {
        let captured = !self.input.cursor_captured();
        self.set_cursor_captured(captured);
    }

    fn shutdown(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            let _ = renderer.wait_idle();
        }
        self.window = None;
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match PlatformWindow::new(
            event_loop,
            &self.config.title,
            self.config.width,
            self.config.height,
        ) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("[engine] failed to create window: {error:?}");
                event_loop.exit();
                return;
            }
        };
        let surface = match window.surface() {
            Ok(surface) => surface,
            Err(error) => {
                eprintln!("[engine] failed to query native window handles: {error:?}");
                event_loop.exit();
                return;
            }
        };

        let scene = self.game.create_scene();
        match backend::create_renderer(self.config.backend, surface, &scene) {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                self.window = Some(window);
                self.last_frame = Instant::now();
                event_loop.set_control_flow(ControlFlow::Poll);
                self.set_cursor_captured(self.config.cursor_locked);
                if !self.input.cursor_captured() {
                    eprintln!(
                        "[engine] cursor capture is unavailable; menu mode is active (Tab retries)"
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Err(error) => {
                eprintln!(
                    "[engine] failed to initialize the {} backend: {error:?}",
                    self.config.backend.name()
                );
                event_loop.exit();
            }
        }
    }

    fn draw(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().clamp(0.0, 0.1);
        self.last_frame = now;

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let (width, height) = renderer.extent();
        if width == 0 || height == 0 {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }

        let ui = self
            .ui
            .get_or_insert_with(|| UiDrawList::new(width, height));
        ui.resize(width, height);
        ui.clear();

        let stats = renderer.stats();
        let mut ctx = UpdateContext {
            dt,
            input: &self.input,
            ui,
            stats,
            viewport: (width, height),
        };
        self.game.update(&mut ctx);

        let camera = self.game.camera();
        let camera_frame = CameraFrame {
            view: camera.view_matrix(),
            projection: camera.projection_matrix(width as f32 / height as f32),
            position: camera.position,
        };
        let frame = FrameData {
            camera: camera_frame,
            ui_vertices: ui.vertices(),
            debug_view: self.game.debug_view(),
        };
        if let Err(error) = renderer.render(&frame) {
            eprintln!("[engine] render error: {error:?}");
            self.consecutive_render_errors += 1;
            if self.consecutive_render_errors >= 10 {
                eprintln!("[engine] renderer failed repeatedly, shutting down");
                self.shutdown();
                return;
            }
        } else {
            self.consecutive_render_errors = 0;
        }

        self.input.begin_frame();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl<G: Game> ApplicationHandler for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.initialize(event_loop);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.shutdown();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input
                .add_mouse_delta(Vec2::new(delta.0 as f32, delta.1 as f32));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.shutdown();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer
                    && let Err(error) = renderer.resize(size.width, size.height)
                {
                    eprintln!("[engine] resize failed: {error:?}");
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input
                    .set_cursor_position(Vec2::new(position.x as f32, position.y as f32));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left && state == ElementState::Pressed {
                    self.input.set_primary_clicked(true);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(position) => {
                        position.y as f32 / 40.0
                    }
                };
                self.input.add_scroll_delta(amount);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let pressed = event.state == ElementState::Pressed;
                self.input.set_key(code, pressed);
                if !pressed {
                    return;
                }
                match code {
                    KeyCode::Tab => self.toggle_cursor_capture(),
                    KeyCode::Escape => {
                        if self.input.cursor_captured() {
                            self.toggle_cursor_capture();
                        } else {
                            self.shutdown();
                            event_loop.exit();
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.draw();
                self.frames_rendered += 1;
                if self
                    .frame_budget
                    .is_some_and(|limit| self.frames_rendered >= limit)
                {
                    eprintln!(
                        "[engine] frame budget reached ({} frames), shutting down",
                        self.frames_rendered
                    );
                    self.shutdown();
                    event_loop.exit();
                }
            }
            WindowEvent::Destroyed => {
                self.shutdown();
                event_loop.exit();
            }
            _ => {}
        }
    }
}
