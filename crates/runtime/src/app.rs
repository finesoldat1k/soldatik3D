use std::time::Instant;

use platform::winit;
use platform::winit::application::ApplicationHandler;
use platform::winit::event::{
    DeviceEvent, ElementState, MouseButton, Touch, TouchPhase, WindowEvent,
};
use platform::winit::event_loop::{ActiveEventLoop, ControlFlow};
use platform::winit::keyboard::{KeyCode, PhysicalKey};
use platform::winit::window::WindowId;
use platform::{Input, PlatformWindow};
use render::{CameraFrame, FrameData, Renderer};
use scene::glam::Vec3;
use scene::{CameraId, World};
use ui::Ui;

use crate::backend;
use crate::commands::{BuiltinCommandContext, CommandRegistry};
use crate::console::Console;
use crate::game::{CommandContext, Game, GameConfig, UpdateContext};

/// Engine-owned runtime. Games never see winit or graphics-API details directly.
pub(crate) struct App<G: Game> {
    game: G,
    config: GameConfig,

    // The renderer must be dropped before the window that owns its surface.
    renderer: Option<Box<dyn Renderer>>,
    window: Option<PlatformWindow>,
    world: Option<World>,

    input: Input,
    console: Console,
    ui: Ui,
    last_frame: Instant,
    consecutive_render_errors: u32,
    frame_budget: Option<u64>,
    frames_rendered: u64,
    exit_requested: bool,
    console_restore_capture: bool,
    startup_commands: Vec<String>,
    open_console_on_start: bool,
    command_registry: CommandRegistry,
}

impl<G: Game> App<G> {
    pub(crate) fn new(game: G) -> Self {
        let config = game.config();
        Self {
            game,
            config,
            renderer: None,
            window: None,
            world: None,
            input: Input::default(),
            console: Console::new(),
            ui: Ui::new(),
            last_frame: Instant::now(),
            consecutive_render_errors: 0,
            frame_budget: std::env::var("SOLDATIK3D_MAX_FRAMES")
                .ok()
                .and_then(|value| value.parse().ok()),
            frames_rendered: 0,
            exit_requested: false,
            console_restore_capture: false,
            startup_commands: std::env::var("SOLDATIK3D_COMMANDS")
                .ok()
                .map(|value| {
                    value
                        .split(';')
                        .map(str::trim)
                        .filter(|command| !command.is_empty())
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            open_console_on_start: std::env::var("SOLDATIK3D_CONSOLE")
                .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on")),
            command_registry: crate::commands::create_default_registry(),
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

    /// Opens or closes the console and releases / recaptures the cursor.
    fn set_console_open(&mut self, open: bool) {
        if open == self.console.is_open() {
            return;
        }
        if open {
            self.console_restore_capture = self.input.cursor_captured();
            self.set_cursor_captured(false);
            self.console.set_open(true);
        } else {
            self.console.set_open(false);
            if self.console_restore_capture && self.config.cursor_locked {
                self.set_cursor_captured(true);
            }
            self.console_restore_capture = false;
        }
    }

    fn toggle_console(&mut self) {
        let open = !self.console.is_open();
        self.set_console_open(open);
    }

    fn shutdown(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            let _ = renderer.wait_idle();
        }
        self.world = None;
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

        let world = self.game.create_world();
        match backend::create_renderer(
            self.config.backend,
            surface,
            world.scene(),
            self.config.vsync,
        ) {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                self.world = Some(world);
                self.window = Some(window);
                self.last_frame = Instant::now();
                event_loop.set_control_flow(ControlFlow::Poll);
                self.set_cursor_captured(self.config.cursor_locked);
                if !self.input.cursor_captured() {
                    eprintln!(
                        "[engine] cursor capture is unavailable; menu mode is active (Tab retries)"
                    );
                }
                if !self.startup_commands.is_empty() {
                    let commands = std::mem::take(&mut self.startup_commands);
                    for command in commands {
                        self.execute_command(&command);
                        if self.exit_requested {
                            event_loop.exit();
                            return;
                        }
                    }
                }
                if self.open_console_on_start {
                    self.set_console_open(true);
                }
                // Apply vsync setting
                if let Some(renderer) = &mut self.renderer {
                    let _ = renderer.set_vsync(self.config.vsync);
                }
                // Apply game-defined input action map
                self.input.set_action_map(self.game.input_map());
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

        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let (width, height) = renderer.extent();
        if width == 0 || height == 0 {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }

        // Text events arrive between frames. Hand them to the console now;
        // `Input` clears its buffer after the frame is drawn.
        let typed = self.input.text().to_owned();
        self.console.push_text(&typed);
        if let Some(line) = self.console.take_pending() {
            let line = line.to_owned();
            self.execute_command(&line);
        }
        if self.exit_requested {
            return;
        }

        self.ui.begin_frame(
            &self.input,
            scene::glam::Vec2::new(width as f32, height as f32),
        );

        let stats = self
            .renderer
            .as_ref()
            .map(|renderer| renderer.stats())
            .unwrap_or_default();
        let Some(world) = self.world.as_mut() else {
            return;
        };
        {
            let mut ctx = UpdateContext {
                dt,
                input: &self.input,
                ui: &mut self.ui,
                stats,
                viewport: (width, height),
                world,
                console_open: self.console.is_open(),
            };
            self.game.update(&mut ctx);
        }
        self.ui.end_frame();
        // The console is raw immediate drawing; appending it after the tree
        // keeps it on top of game HUDs.
        self.console.draw(&mut self.ui);

        let camera = world.active_camera();
        let camera_frame = CameraFrame {
            view: camera.view_matrix(),
            projection: camera.projection_matrix(width as f32 / height as f32),
            position: camera.position,
        };

        let atlas = self.ui.draw().atlas_update();
        let dirty: Vec<render::UiAtlasRect> = atlas
            .as_ref()
            .map(|update| {
                update
                    .dirty
                    .iter()
                    .map(|rect| render::UiAtlasRect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let ui_atlas = atlas.as_ref().map(|update| render::UiAtlasFrame {
            width: update.width,
            height: update.height,
            pixels: self.ui.draw().atlas_pixels(),
            full: update.full,
            dirty: &dirty,
        });
        let frame = FrameData {
            camera: camera_frame,
            ui_vertices: self.ui.draw().vertices(),
            ui_atlas,
            debug_view: self.game.debug_view(),
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
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
            self.ui.draw_mut().mark_atlas_uploaded();
        }

        self.input.begin_frame();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Handles touch input events from winit.
    fn handle_touch(&mut self, touch: Touch) {
        let position = scene::glam::Vec2::new(touch.location.x as f32, touch.location.y as f32);
        let id = touch.id;

        match touch.phase {
            TouchPhase::Started => {
                self.input.add_touch(id, position);
            }
            TouchPhase::Moved => {
                self.input.update_touch(id, position);
            }
            TouchPhase::Ended => {
                self.input.end_touch(id);
            }
            TouchPhase::Cancelled => {
                self.input.end_touch(id);
            }
        }
    }

    /// Executes an engine command, then delegates unknown names to the game.
    fn execute_command(&mut self, line: &str) {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap_or_default();
        let args: Vec<&str> = parts.collect();

        if name.is_empty() {
            return;
        }

        if name == "help" {
            self.print_help();
            return;
        }

        // Try built-in commands first
        if let Some(command) = self.command_registry.get(name) {
            let mut ctx = BuiltinCommandContext::new(
                self.world.as_mut().expect("world exists during command"),
                &mut self.console,
                self.renderer.as_mut(),
                &args,
                &mut self.exit_requested,
            );
            let result = command.execute(&mut ctx);
            match result {
                crate::commands::CommandResult::Ok(Some(msg)) => self.console.print(msg),
                crate::commands::CommandResult::Ok(None) => {}
                crate::commands::CommandResult::Err(msg) => self.console.error(msg),
            }
            return;
        }

        // Delegate to game
        self.command_game(name, &args);
    }

    fn print_help(&mut self) {
        self.console.print("Soldatik3D console commands:");
        // Print built-in commands from registry
        for command in self.command_registry.all() {
            self.console.print(format!(
                "  {:<24} {}",
                command.name(),
                command.description()
            ));
        }
        // Print game commands
        let extra = self.game.command_help();
        if !extra.is_empty() {
            self.console.print("game commands:");
            for (name, description) in extra {
                self.console.print(format!("  {name:<24} {description}"));
            }
        }
    }

    fn command_game(&mut self, name: &str, args: &[&str]) {
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let mut ctx = CommandContext { name, args, world };
        match self.game.command(&mut ctx) {
            Some(Ok(message)) => self.console.print(message),
            Some(Err(message)) => self.console.error(message),
            None => self
                .console
                .error(format!("unknown command: {name} (type `help`)")),
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
                .add_mouse_delta(scene::glam::Vec2::new(delta.0 as f32, delta.1 as f32));
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
                self.input.set_cursor_position(scene::glam::Vec2::new(
                    position.x as f32,
                    position.y as f32,
                ));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.input.set_primary_down(state == ElementState::Pressed);
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
            WindowEvent::Touch(touch) => {
                self.handle_touch(touch);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let pressed = event.state == ElementState::Pressed;

                if pressed && self.console.is_open() {
                    let text: String = event
                        .text
                        .as_ref()
                        .map(|text| text.chars().filter(|ch| !ch.is_control()).collect())
                        .unwrap_or_default();
                    self.input.push_text(&text);
                }
                self.input.set_key(code, pressed);
                if !pressed {
                    return;
                }

                if matches!(code, KeyCode::F1 | KeyCode::Backquote) {
                    self.toggle_console();
                    return;
                }

                if self.console.is_open() {
                    match code {
                        KeyCode::Escape => {
                            self.set_console_open(false);
                            self.console.reset_completion();
                        }
                        KeyCode::Enter => {
                            self.console.submit();
                            self.console.reset_completion();
                        }
                        KeyCode::Backspace => {
                            self.console.backspace();
                            self.console.reset_completion();
                        }
                        KeyCode::ArrowUp => {
                            self.console.history_previous();
                            self.console.reset_completion();
                        }
                        KeyCode::ArrowDown => {
                            self.console.history_next();
                            self.console.reset_completion();
                        }
                        KeyCode::Tab => {
                            self.console.complete(&self.command_registry);
                        }
                        _ => {
                            self.console.reset_completion();
                        }
                    }
                    return;
                }

                match code {
                    KeyCode::Tab => self.toggle_cursor_capture(),
                    KeyCode::F12 => {
                        if let Some(renderer) = &mut self.renderer {
                            let _ = renderer.screenshot();
                        }
                    }
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
                let budget_reached = self
                    .frame_budget
                    .is_some_and(|limit| self.frames_rendered >= limit);
                if budget_reached {
                    eprintln!(
                        "[engine] frame budget reached ({} frames), shutting down",
                        self.frames_rendered
                    );
                }
                if self.exit_requested || budget_reached {
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

pub(crate) fn resolve_camera(world: &World, token: &str) -> Option<CameraId> {
    if let Some(camera) = world.cameras().get_by_name(token) {
        return Some(camera.id);
    }
    let raw = token.strip_prefix("cam-").unwrap_or(token);
    let value = raw.parse::<u32>().ok()?;
    let id = CameraId::from_raw(value);
    world.cameras().get(id).map(|_| id)
}

pub(crate) fn parse_vec3(values: &[&str]) -> Option<Vec3> {
    Some(Vec3::new(
        values.first()?.parse().ok()?,
        values.get(1)?.parse().ok()?,
        values.get(2)?.parse().ok()?,
    ))
}
