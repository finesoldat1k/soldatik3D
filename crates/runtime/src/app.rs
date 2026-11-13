use std::time::Instant;

use platform::winit;
use platform::winit::application::ApplicationHandler;
use platform::winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use platform::winit::event_loop::{ActiveEventLoop, ControlFlow};
use platform::winit::keyboard::{KeyCode, PhysicalKey};
use platform::winit::window::WindowId;
use platform::{Input, PlatformWindow};
use render::{CameraFrame, FrameData, Renderer};
use scene::glam::Vec3;
use scene::{CameraId, World};
use ui::UiDrawList;

use crate::backend;
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
    ui: Option<UiDrawList>,
    last_frame: Instant,
    consecutive_render_errors: u32,
    frame_budget: Option<u64>,
    frames_rendered: u64,
    exit_requested: bool,
    console_restore_capture: bool,
    startup_commands: Vec<String>,
    open_console_on_start: bool,
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
            ui: None,
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
        match backend::create_renderer(self.config.backend, surface, world.scene()) {
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

        let ui = self
            .ui
            .get_or_insert_with(|| UiDrawList::new(width, height));
        ui.resize(width, height);
        ui.clear();

        let stats = self
            .renderer
            .as_ref()
            .map(|renderer| renderer.stats())
            .unwrap_or_default();
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let mut ctx = UpdateContext {
            dt,
            input: &self.input,
            ui,
            stats,
            viewport: (width, height),
            world,
            console_open: self.console.is_open(),
        };
        self.game.update(&mut ctx);

        self.console.draw(ui);

        let camera = world.active_camera();
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
        }

        self.input.begin_frame();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Executes an engine command, then delegates unknown names to the game.
    fn execute_command(&mut self, line: &str) {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap_or_default();
        let args: Vec<&str> = parts.collect();

        match name {
            "" => {}
            "help" => self.print_help(),
            "clear" => self.console.clear(),
            "quit" | "exit" => {
                self.console.print("shutting down");
                self.exit_requested = true;
            }
            "backend" => {
                let name = self
                    .renderer
                    .as_ref()
                    .map(|renderer| renderer.backend().name())
                    .unwrap_or("none");
                self.console.print(format!(
                    "backend: {name} (SOLDATIK3D_BACKEND=vulkan|opengl)"
                ));
            }
            "camera.list" => self.command_camera_list(),
            "camera.select" => self.command_camera_select(&args),
            "camera.next" => self.command_camera_step(true),
            "camera.prev" | "camera.previous" => self.command_camera_step(false),
            "camera.spawn" => self.command_camera_spawn(&args),
            "camera.remove" => self.command_camera_remove(&args),
            "camera.rename" => self.command_camera_rename(&args),
            "camera.info" => self.command_camera_info(),
            "camera.set" => self.command_camera_set(&args),
            _ => self.command_game(name, &args),
        }
    }

    fn print_help(&mut self) {
        self.console.print("Soldatik3D console commands:");
        self.console.print("  help | clear | quit | backend");
        self.console
            .print("  camera.list | camera.next | camera.prev");
        self.console.print("  camera.select <name|id>");
        self.console
            .print("  camera.spawn <name> [x y z] [yaw] [pitch]");
        self.console
            .print("  camera.remove <name|id> | camera.rename <name>");
        self.console
            .print("  camera.info | camera.set x y z [yaw] [pitch]");
        let extra = self.game.command_help();
        if !extra.is_empty() {
            self.console.print("game commands:");
            for (name, description) in extra {
                self.console.print(format!("  {name:<18} {description}"));
            }
        }
    }

    fn command_camera_list(&mut self) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let active = world.active_camera_id();
        let lines: Vec<String> = world
            .cameras()
            .iter()
            .map(|camera| {
                let marker = if Some(camera.id) == active { "*" } else { " " };
                format!(
                    "{marker} {:<16} {}  pos ({:.1}, {:.1}, {:.1})",
                    camera.name,
                    camera.id,
                    camera.camera.position.x,
                    camera.camera.position.y,
                    camera.camera.position.z
                )
            })
            .collect();
        self.console.print(format!("{} camera(s):", lines.len()));
        for line in lines {
            self.console.print(format!("  {line}"));
        }
    }

    fn command_camera_select(&mut self, args: &[&str]) {
        let Some(token) = args.first() else {
            self.console.error("usage: camera.select <name|id>");
            return;
        };
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let selected = if world.select_camera_name(token) {
            true
        } else {
            resolve_camera(world, token).is_some_and(|id| world.select_camera(id))
        };
        if selected {
            let name = world.active_camera_name().to_owned();
            self.console.print(format!("active camera: {name}"));
        } else {
            self.console.error(format!("unknown camera: {token}"));
        }
    }

    fn command_camera_step(&mut self, forward: bool) {
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let id = if forward {
            world.select_next_camera()
        } else {
            world.select_previous_camera()
        };
        match id {
            Some(id) => self.console.print(format!(
                "active camera: {} ({id})",
                world.active_camera_name()
            )),
            None => self.console.error("no cameras"),
        }
    }

    fn command_camera_spawn(&mut self, args: &[&str]) {
        let Some(name) = args.first() else {
            self.console
                .error("usage: camera.spawn <name> [x y z] [yaw] [pitch]");
            return;
        };
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let mut camera = world.active_camera();
        if args.len() > 1 {
            if args.len() < 4 {
                self.console
                    .error("camera.spawn: provide all of x y z, or none");
                return;
            }
            let Some(position) = parse_vec3(&args[1..4]) else {
                self.console.error("camera.spawn: invalid position");
                return;
            };
            camera.position = position;
        }
        if let Some(yaw) = args.get(4) {
            match yaw.parse::<f32>() {
                Ok(value) => camera.yaw = value.to_radians(),
                Err(_) => {
                    self.console.error("camera.spawn: invalid yaw");
                    return;
                }
            }
        }
        if let Some(pitch) = args.get(5) {
            match pitch.parse::<f32>() {
                Ok(value) => camera.pitch = value.to_radians(),
                Err(_) => {
                    self.console.error("camera.spawn: invalid pitch");
                    return;
                }
            }
        }
        let id = world.spawn_camera(*name, camera);
        let resolved = world
            .cameras()
            .get(id)
            .map(|camera| camera.name.clone())
            .unwrap_or_else(|| (*name).to_owned());
        self.console
            .print(format!("spawned camera {resolved} ({id})"));
    }

    fn command_camera_remove(&mut self, args: &[&str]) {
        let Some(token) = args.first() else {
            self.console.error("usage: camera.remove <name|id>");
            return;
        };
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let Some(id) = resolve_camera(world, token) else {
            self.console.error(format!("unknown camera: {token}"));
            return;
        };
        match world.remove_camera(id) {
            Some(removed) => self
                .console
                .print(format!("removed camera {} ({id})", removed.name)),
            None => self
                .console
                .error("cannot remove the last remaining camera"),
        }
    }

    fn command_camera_rename(&mut self, args: &[&str]) {
        let Some(name) = args.first() else {
            self.console.error("usage: camera.rename <name>");
            return;
        };
        let Some(world) = self.world.as_mut() else {
            return;
        };
        match world.cameras_mut().rename_active(*name) {
            Some(resolved) => self.console.print(format!("active camera: {resolved}")),
            None => self.console.error("no active camera"),
        }
    }

    fn command_camera_info(&mut self) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let Some(camera) = world.active_camera_ref() else {
            self.console.error("no active camera");
            return;
        };
        let camera = camera.camera;
        self.console.print(format!(
            "{}: pos ({:.2}, {:.2}, {:.2}) yaw {:.1}° pitch {:.1}° fov {:.1}° range {:.2}..{:.1}",
            world.active_camera_name(),
            camera.position.x,
            camera.position.y,
            camera.position.z,
            camera.yaw.to_degrees(),
            camera.pitch.to_degrees(),
            camera.fov_y_degrees,
            camera.z_near,
            camera.z_far
        ));
    }

    fn command_camera_set(&mut self, args: &[&str]) {
        if args.len() < 3 {
            self.console.error("usage: camera.set x y z [yaw] [pitch]");
            return;
        }
        let Some(position) = parse_vec3(&args[0..3]) else {
            self.console.error("camera.set: invalid position");
            return;
        };
        let yaw = args.get(3).and_then(|value| value.parse::<f32>().ok());
        let pitch = args.get(4).and_then(|value| value.parse::<f32>().ok());
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let Some(camera) = world.active_camera_mut() else {
            self.console.error("no active camera");
            return;
        };
        camera.camera.position = position;
        if let Some(yaw) = yaw {
            camera.camera.yaw = yaw.to_radians();
        }
        if let Some(pitch) = pitch {
            camera.camera.pitch = pitch.to_radians();
        }
        let name = camera.name.clone();
        self.console.print(format!("moved camera {name}"));
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
                        KeyCode::Escape => self.set_console_open(false),
                        KeyCode::Enter => self.console.submit(),
                        KeyCode::Backspace => self.console.backspace(),
                        KeyCode::ArrowUp => self.console.history_previous(),
                        KeyCode::ArrowDown => self.console.history_next(),
                        _ => {}
                    }
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

fn resolve_camera(world: &World, token: &str) -> Option<CameraId> {
    if let Some(camera) = world.cameras().get_by_name(token) {
        return Some(camera.id);
    }
    let raw = token.strip_prefix("cam-").unwrap_or(token);
    let value = raw.parse::<u32>().ok()?;
    let id = CameraId::from_raw(value);
    world.cameras().get(id).map(|_| id)
}

fn parse_vec3(values: &[&str]) -> Option<Vec3> {
    Some(Vec3::new(
        values.first()?.parse().ok()?,
        values.get(1)?.parse().ok()?,
        values.get(2)?.parse().ok()?,
    ))
}
