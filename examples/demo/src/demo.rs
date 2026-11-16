//! Engine test polygon demo: a showcase for PBR, IBL, bloom, lighting, shadows,
//! physics, flight, and performance — no gameplay goals.

use runtime::glam::{Vec2, Vec3};
use runtime::ui::{
    AlignItems, ButtonStyle, JoystickState, JoystickStyle, LabelStyle, Layout, Rect, SliderStyle,
    TextAlign, Theme, ToggleStyle, TouchCameraState, TouchCameraStyle, Ui,
};
use runtime::{
    CameraId, CameraRig, CommandContext, DebugView, Game, GameConfig, InputMap, RenderStats,
    UpdateContext, World,
};

use crate::debug_ui::DebugMenu;
use crate::level::TestPolygon;
// use crate::physics; // unused after removing minigame logic
use crate::player::{Player, default_input_map};

const GAME_COMMANDS: &[(&str, &str)] = &[
    ("game.status", "print engine stats and player state"),
    ("game.reset", "reset player to spawn"),
    ("player.teleport x y z [yaw] [pitch]", "move the player"),
    ("player.fly [on|off|toggle]", "toggle free flight"),
    ("player.reset", "return to spawn"),
    ("debug.toggle", "toggle debug overlay"),
    ("debug.view lit|normals|depth", "set debug view mode"),
    ("ui.settings", "open the pause / settings overlay"),
];

const MOUSE_SENSITIVITY: f32 = 0.0022;

pub struct DemoGame {
    player: Player,
    level: Option<TestPolygon>,
    camera: Option<CameraId>,
    initialized: bool,

    paused: bool,
    paused_by_cursor: bool,
    sensitivity: f32,
    invert_y: bool,

    joystick: JoystickState,
    touch_camera: TouchCameraState,
    ui_scale: f32,

    fps: f32,
    frame_ms: f32,
    fps_accumulator: f32,
    fps_frames: u32,

    debug_menu: DebugMenu,
}

impl DemoGame {
    pub fn new() -> Self {
        Self {
            player: Player::new(crate::level::SPAWN, crate::level::SPAWN_YAW),
            level: None,
            camera: None,
            initialized: false,
            paused: false,
            paused_by_cursor: false,
            sensitivity: 1.0,
            invert_y: false,
            joystick: JoystickState::default(),
            touch_camera: TouchCameraState::default(),
            ui_scale: 0.0,
            fps: 0.0,
            frame_ms: 0.0,
            fps_accumulator: 0.0,
            fps_frames: 0,
            debug_menu: DebugMenu::new(),
        }
    }

    fn sync_camera(&self, world: &mut World) {
        let camera = self.player.camera();
        if let Some(named) = world.active_camera_mut() {
            named.camera = camera;
        }
    }
}

impl Default for DemoGame {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for DemoGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("Soldatik3D // Engine Test Polygon")
            .with_size(1280, 720)
            .with_cursor_locked(!cfg!(target_os = "android"))
    }

    fn create_world(&mut self) -> World {
        let polygon = crate::level::build_polygon();
        let scene = polygon.scene.clone();

        self.level = Some(polygon);

        let mut cameras = CameraRig::empty();
        self.camera = Some(cameras.spawn("player", self.player.camera()));
        if !self.initialized {
            self.initialized = true;
        }
        World::with_cameras(scene, cameras)
    }

    fn input_map(&self) -> InputMap {
        default_input_map()
    }

    fn command_help(&self) -> &'static [(&'static str, &'static str)] {
        GAME_COMMANDS
    }

    fn command(&mut self, ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
        match ctx.name {
            "game.status" => {
                let pos = self.player.body.position;
                let flying = if self.player.is_flying() {
                    "FLY"
                } else {
                    "WALK"
                };
                Some(Ok(format!(
                    "pos ({:.1}, {:.1}, {:.1}) yaw {:.2} {} | FPS {:.0} frame {:.2}ms",
                    pos.x, pos.y, pos.z, self.player.yaw, flying, self.fps, self.frame_ms
                )))
            }
            "game.reset" => {
                self.player
                    .teleport(crate::level::SPAWN, crate::level::SPAWN_YAW);
                self.player.set_flying(false);
                self.sync_camera(ctx.world);
                Some(Ok("player reset to spawn".to_owned()))
            }
            "player.teleport" => {
                let x = match parse_number(ctx.args, 0) {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return Some(Err("usage: player.teleport x y z [yaw] [pitch]".to_owned()));
                    }
                    Err(error) => return Some(Err(error)),
                };
                let y = match parse_number(ctx.args, 1) {
                    Ok(Some(value)) => value,
                    _ => return Some(Err("missing y".to_owned())),
                };
                let z = match parse_number(ctx.args, 2) {
                    Ok(Some(value)) => value,
                    _ => return Some(Err("missing z".to_owned())),
                };
                let yaw = match parse_number(ctx.args, 3) {
                    Ok(value) => value.unwrap_or(self.player.yaw),
                    Err(error) => return Some(Err(error)),
                };
                let pitch = match parse_number(ctx.args, 4) {
                    Ok(value) => value.unwrap_or(0.0),
                    Err(error) => return Some(Err(error)),
                };
                self.player.teleport(Vec3::new(x, y, z), yaw);
                self.player.look(0.0, pitch);
                self.sync_camera(ctx.world);
                Some(Ok(format!("player at ({x:.1}, {y:.1}, {z:.1})")))
            }
            "player.fly" => {
                let flying = match ctx.args.first().copied() {
                    None | Some("toggle") => !self.player.is_flying(),
                    Some("on" | "true" | "1") => true,
                    Some("off" | "false" | "0") => false,
                    Some(value) => return Some(Err(format!("unknown mode: {value}"))),
                };
                self.player.set_flying(flying);
                self.sync_camera(ctx.world);
                Some(Ok(format!(
                    "flight mode: {}",
                    if flying { "on" } else { "off" }
                )))
            }
            "player.reset" => {
                self.player
                    .teleport(crate::level::SPAWN, crate::level::SPAWN_YAW);
                self.player.set_flying(false);
                self.sync_camera(ctx.world);
                Some(Ok("player returned to spawn".to_owned()))
            }
            "debug.toggle" => {
                self.debug_menu.open = !self.debug_menu.open;
                Some(Ok(format!(
                    "debug overlay: {}",
                    if self.debug_menu.open {
                        "open"
                    } else {
                        "closed"
                    }
                )))
            }
            "debug.view" => {
                let view = match ctx.args.first().copied() {
                    Some("lit") => DebugView::Lit,
                    Some("normals") => DebugView::Normals,
                    Some("depth") => DebugView::Depth,
                    _ => return Some(Err("usage: debug.view lit|normals|depth".to_owned())),
                };
                self.debug_menu.view = view;
                Some(Ok(format!("debug view: {:?}", view)))
            }
            "ui.settings" => {
                self.paused = !self.paused;
                self.paused_by_cursor = false;
                Some(Ok(format!(
                    "pause menu: {}",
                    if self.paused { "open" } else { "closed" }
                )))
            }
            _ => None,
        }
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let dt = ctx.dt.clamp(0.0, 0.05);
        let scale = ctx.input.scale_factor().clamp(0.75, 3.0);
        if (self.ui_scale - scale).abs() > 0.001 {
            self.ui_scale = scale;
            ctx.ui.set_theme(Theme::dark().scaled(scale));
        }

        self.frame_ms = dt * 1000.0;
        self.fps_accumulator += dt;
        self.fps_frames += 1;
        if self.fps_accumulator >= 0.5 {
            self.fps = self.fps_frames as f32 / self.fps_accumulator;
            self.fps_accumulator = 0.0;
            self.fps_frames = 0;
        }

        let touch_mode = ctx.input.touch_mode() || cfg!(target_os = "android");
        let width = ctx.viewport.0 as f32;
        let height = ctx.viewport.1 as f32;

        // Handle debug menu shortcuts
        self.debug_menu.handle_shortcuts(ctx.input);

        // Releasing the desktop cursor (Escape / Tab) opens the pause menu;
        // locking it again with Tab resumes automatically.
        if !touch_mode && !ctx.input.cursor_captured() && !ctx.console_open {
            if !self.paused {
                self.paused = true;
                self.paused_by_cursor = true;
                self.joystick.reset();
                self.touch_camera.reset();
            }
        } else if !touch_mode && ctx.input.cursor_captured() && self.paused_by_cursor {
            self.paused = false;
            self.paused_by_cursor = false;
        }

        // P is an explicit pause toggle for keyboard play.
        if !ctx.console_open && ctx.input.action_pressed("pause") {
            if self.paused {
                if touch_mode || ctx.input.cursor_captured() {
                    self.paused = false;
                    self.paused_by_cursor = false;
                    self.joystick.reset();
                    self.touch_camera.reset();
                }
            } else {
                self.paused = true;
                self.paused_by_cursor = false;
                self.joystick.reset();
                self.touch_camera.reset();
            }
        }

        if !ctx.console_open && ctx.input.action_pressed("restart") {
            self.player
                .teleport(crate::level::SPAWN, crate::level::SPAWN_YAW);
            self.player.set_flying(false);
            self.sync_camera(ctx.world);
            self.paused = false;
            self.paused_by_cursor = false;
        }

        if self.level.is_none() {
            return;
        }

        let mut controls_active = !self.paused && !ctx.console_open;
        let mut axes = Vec2::ZERO;
        let mut jump_pressed = false;
        let mut jump_held = false;
        let mut fly_toggle = false;

        if controls_active {
            if touch_mode {
                let margin = 24.0 * scale;
                let size = 84.0 * scale;
                let gap = 14.0 * scale;
                let pause_size = 54.0 * scale;
                let base_style = ButtonStyle::from_theme(ctx.ui.theme());
                let action_style = base_style
                    .clone()
                    .height(size)
                    .font_size(17.0 * scale)
                    .padding(8.0 * scale);
                let pause_style = base_style
                    .height(pause_size)
                    .font_size(16.0 * scale)
                    .padding(4.0 * scale);

                ctx.ui.begin_container(Layout::default().fill());
                let pause = ctx.ui.button_with(
                    Layout::default().absolute(Rect::new(
                        width - margin - pause_size,
                        margin,
                        pause_size,
                        pause_size,
                    )),
                    "II",
                    pause_style,
                );
                let fly_btn = ctx.ui.button_with(
                    Layout::default().absolute(Rect::new(
                        width - margin - size * 3.0 - gap * 2.0,
                        height - margin - size,
                        size,
                        size,
                    )),
                    "FLY",
                    action_style.clone(),
                );
                let up_btn = ctx.ui.button_with(
                    Layout::default().absolute(Rect::new(
                        width - margin - size * 2.0 - gap,
                        height - margin - size * 2.0 - gap,
                        size,
                        size,
                    )),
                    "UP",
                    action_style.clone(),
                );
                let down_btn = ctx.ui.button_with(
                    Layout::default().absolute(Rect::new(
                        width - margin - size * 2.0 - gap,
                        height - margin - size,
                        size,
                        size,
                    )),
                    "DOWN",
                    action_style.clone(),
                );
                let jump = ctx.ui.button_with(
                    Layout::default().absolute(Rect::new(
                        width - margin - size,
                        height - margin - size,
                        size,
                        size,
                    )),
                    "JUMP",
                    action_style,
                );
                if pause.clicked {
                    self.paused = true;
                    self.paused_by_cursor = false;
                    self.joystick.reset();
                    self.touch_camera.reset();
                    controls_active = false;
                }
                if fly_btn.clicked {
                    fly_toggle = true;
                }
                if up_btn.pressed {
                    jump_held = true;
                }
                if down_btn.pressed {
                    // crouch in flight = down
                }
                if jump.pressed {
                    jump_pressed = true;
                    jump_held = true;
                }

                let mut joystick_style = JoystickStyle::from_theme(ctx.ui.theme()).scaled(scale);
                joystick_style.area = Some(Rect::new(0.0, 0.0, width * 0.45, height));
                axes = ctx.ui.virtual_joystick(&mut self.joystick, &joystick_style);

                let mut camera_style = TouchCameraStyle::from_theme(ctx.ui.theme());
                camera_style.area = Some(Rect::new(width * 0.45, 0.0, width * 0.55, height));
                camera_style.sensitivity = 0.003 * self.sensitivity;
                camera_style.enable_mouse = false;
                let delta = ctx.ui.touch_camera(&mut self.touch_camera, &camera_style);
                if delta.length_squared() > 0.0 {
                    let pitch = if self.invert_y { delta.y } else { -delta.y };
                    self.player.look(delta.x, pitch);
                }
                ctx.ui.end_container();
            } else if ctx.input.cursor_captured() && !ctx.ui.captures_pointer() {
                self.player.look_mouse(
                    ctx.input.mouse_delta(),
                    MOUSE_SENSITIVITY * self.sensitivity,
                    self.invert_y,
                );
            }

            // Desktop keyboard camera controls remain available even when
            // the cursor is locked.
            self.player.look_keyboard(ctx.input, dt);

            if ctx.input.action_down("move_forward") {
                axes.y += 1.0;
            }
            if ctx.input.action_down("move_backward") {
                axes.y -= 1.0;
            }
            if ctx.input.action_down("move_right") {
                axes.x += 1.0;
            }
            if ctx.input.action_down("move_left") {
                axes.x -= 1.0;
            }
            if axes.length_squared() > 1.0 {
                axes = axes.normalize();
            }
            if ctx.input.action_pressed("jump") {
                jump_pressed = true;
            }
            if ctx.input.action_down("jump") {
                jump_held = true;
            }
            if ctx.input.action_pressed("fly_toggle") {
                fly_toggle = true;
            }
        }

        if fly_toggle {
            self.player.set_flying(!self.player.is_flying());
        }

        controls_active = controls_active && !self.paused;
        if controls_active {
            let crouch = ctx.input.action_down("crouch");
            let sprint = ctx.input.action_down("sprint")
                || (touch_mode && self.joystick.active && self.joystick.value.length() > 0.92);

            if let Some(level) = self.level.as_ref() {
                self.player.update(
                    dt,
                    &level.physics,
                    axes,
                    jump_pressed,
                    jump_held,
                    crouch,
                    sprint,
                );
            }

            if self.player.body.position.y < crate::level::FALL_LIMIT {
                self.player
                    .teleport(crate::level::SPAWN, crate::level::SPAWN_YAW);
                self.player.set_flying(false);
            }

            self.sync_camera(ctx.world);
        }

        // Draw minimal HUD (just crosshair when not paused)
        if !self.paused && !ctx.console_open {
            self.draw_hud(ctx.ui, width, height, scale, touch_mode);
        }

        // Draw debug menu overlay
        if let Some(level) = self.level.as_ref() {
            let stats = RenderStats {
                vertices: 0, // TODO: get from renderer
                triangles: 0,
                draw_calls: level.scene.meshes.len() as u32,
            };
            self.debug_menu.draw(
                ctx.ui,
                scale,
                (width, height),
                stats,
                ctx.world
                    .active_camera_ref()
                    .map(|c| c.camera.position)
                    .unwrap_or(Vec3::ZERO),
                "player",
                self.player.body.position,
                self.player.is_flying(),
                "polygon",
                self.fps,
                self.frame_ms,
                ctx.console_open,
            );
        }

        if self.paused {
            self.draw_pause(
                ctx.ui,
                width,
                height,
                scale,
                touch_mode,
                ctx.input.cursor_captured(),
            );
        }
    }
}

fn parse_number(args: &[&str], index: usize) -> Result<Option<f32>, String> {
    match args.get(index) {
        Some(value) => value
            .parse::<f32>()
            .map(Some)
            .map_err(|_| format!("invalid number: {value}")),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// HUD and overlays
// ---------------------------------------------------------------------------

impl DemoGame {
    fn draw_hud(&self, ui: &mut Ui, width: f32, height: f32, scale: f32, touch_mode: bool) {
        if touch_mode {
            // On touch, show a small crosshair
            ui.crosshair();
            return;
        }

        let accent = ui.theme().accent;
        let color = [accent[0], accent[1], accent[2], 0.95];
        let cx = width * 0.5;
        let cy = height * 0.5;
        let gap = 4.0 * scale;
        let arm = 7.0 * scale;
        let thickness = (1.5 * scale).max(1.0);
        ui.rect(cx - gap - arm, cy - thickness * 0.5, arm, thickness, color);
        ui.rect(cx + gap, cy - thickness * 0.5, arm, thickness, color);
        ui.rect(cx - thickness * 0.5, cy - gap - arm, thickness, arm, color);
        ui.rect(cx - thickness * 0.5, cy + gap, thickness, arm, color);
        let dot = (2.5 * scale).max(1.5);
        ui.rect(cx - dot * 0.5, cy - dot * 0.5, dot, dot, color);

        // Show FPS in corner
        let fps = if self.fps > 0.0 {
            format!("{:.0} FPS", self.fps)
        } else {
            "-- FPS".to_owned()
        };
        let fps_size = (9.0 * scale).max(7.0);
        let fps_w = ui.text_width(&fps, fps_size);
        let margin = 22.0 * scale;
        ui.text(
            &fps,
            width - margin - fps_w,
            margin,
            fps_size,
            [0.58, 0.63, 0.70, 0.7],
        );

        // Show flight mode indicator
        if self.player.is_flying() {
            let fly_text = "FLIGHT MODE";
            let fly_size = (11.0 * scale).max(8.0);
            let fly_w = ui.text_width(fly_text, fly_size);
            ui.text(
                fly_text,
                (width - fly_w) * 0.5,
                margin + 30.0 * scale,
                fly_size,
                [1.0, 0.62, 0.16, 1.0],
            );
        }
    }

    fn draw_pause(
        &mut self,
        ui: &mut Ui,
        width: f32,
        height: f32,
        scale: f32,
        touch_mode: bool,
        cursor_captured: bool,
    ) {
        let theme = ui.theme().clone();
        ui.rect(0.0, 0.0, width, height, [0.008, 0.012, 0.02, 0.62]);

        let panel_w = (430.0 * scale).min(width - 44.0 * scale).max(240.0);
        let pad = 22.0 * scale;
        let gap = 8.0 * scale;
        let content_w = (panel_w - pad * 2.0).max(80.0);
        let button_w = (content_w - gap) * 0.5;

        ui.begin_container(Layout::default().fill().center());
        ui.begin_styled_container(
            Layout::column()
                .width(panel_w)
                .padding(pad)
                .gap(gap)
                .align_items(AlignItems::Center),
            theme.window,
            theme.panel_border,
            theme.radius,
        );
        ui.heading("Paused");
        let pos = self.player.body.position;
        let stats = format!(
            "pos ({:.1}, {:.1}, {:.1}) · yaw {:.2} · {}",
            pos.x,
            pos.y,
            pos.z,
            self.player.yaw,
            if self.player.is_flying() {
                "FLY"
            } else {
                "WALK"
            }
        );
        ui.label_styled(stats, LabelStyle::new(12.0 * scale).color(theme.text_muted));

        let mut slider_style = SliderStyle::from_theme(&theme);
        slider_style.font_size = 13.0 * scale;
        slider_style.height = 40.0 * scale;
        ui.slider_styled(
            "Look sensitivity",
            &mut self.sensitivity,
            0.25,
            2.5,
            slider_style,
        );
        let mut toggle_style = ToggleStyle::from_theme(&theme);
        toggle_style.font_size = 13.0 * scale;
        toggle_style.height = 40.0 * scale;
        ui.toggle_styled("Invert look", &mut self.invert_y, toggle_style);

        let primary = ButtonStyle::primary(&theme);
        let regular = ButtonStyle::from_theme(&theme);
        ui.begin_container(Layout::row().gap(gap));
        let resume = if touch_mode {
            Some(ui.button_with(Layout::default().width(button_w), "Resume", primary.clone()))
        } else {
            None
        };
        let reset = ui.button_with(
            Layout::default().width(button_w),
            "Reset player",
            if resume.is_some() { regular } else { primary },
        );
        ui.end_container();
        if resume.as_ref().is_some_and(|response| response.clicked) {
            self.paused = false;
            self.paused_by_cursor = false;
            self.joystick.reset();
            self.touch_camera.reset();
        }
        if reset.clicked {
            self.player
                .teleport(crate::level::SPAWN, crate::level::SPAWN_YAW);
            self.player.set_flying(false);
            // Camera will sync on next frame
            self.paused = false;
            self.paused_by_cursor = false;
        }
        if !touch_mode {
            let hint = if cursor_captured {
                "P resumes · TAB releases cursor · R resets player"
            } else {
                "TAB resumes · R resets player"
            };
            ui.label_styled(
                hint,
                LabelStyle::new(10.0 * scale)
                    .color(theme.text_muted)
                    .align(TextAlign::Center),
            );
        }
        ui.end_container();
        ui.end_container();
    }
}

// Need to fix the ctx.world reference in draw_pause - use a closure or pass it
// Actually, draw_pause is called from update which has ctx, so we need to pass it.
// Let me fix this by making draw_pause take &mut UpdateContext instead.

// We'll fix this in the next edit.
