//! Demo game: a compact reactor facility with a first-person controller.
//!
//! The player restarts three coolant pumps in the west wing, then engages the
//! reactor in the north chamber. The run is timed, checkpoints provide respawn
//! points, and the HUD adapts to touch screens with a joystick, look area and
//! action buttons that can all be used at the same time.

use runtime::glam::{Vec2, Vec3};
use runtime::ui::{
    AlignItems, ButtonStyle, JoystickState, JoystickStyle, LabelStyle, Layout, Rect, SliderStyle,
    TextAlign, Theme, ToggleStyle, TouchCameraState, TouchCameraStyle, Ui,
};
use runtime::{
    Aabb, CameraId, CameraRig, CommandContext, Game, GameConfig, InputMap, UpdateContext, World,
};

use crate::level::{self, Checkpoint, Device, Facility, Interactable};
use crate::physics::{PLAYER_HEIGHT, PLAYER_RADIUS, PhysicsWorld};
use crate::player::{Player, default_input_map};

const GAME_COMMANDS: &[(&str, &str)] = &[
    ("game.status", "print objective, timer and best time"),
    ("game.restart", "restart the facility run"),
    ("game.pumps", "activate every coolant pump (debug)"),
    ("game.win", "complete the run (debug)"),
    ("player.teleport x y z [yaw] [pitch]", "move the player"),
    ("player.fly [on|off|toggle]", "toggle free flight"),
    ("player.reset", "return to the active checkpoint"),
    ("ui.settings", "open the pause / settings overlay"),
];

const MOUSE_SENSITIVITY: f32 = 0.0022;
const MESSAGE_TIME: f32 = 3.2;
const FLASH_TIME: f32 = 0.65;

struct LevelState {
    physics: PhysicsWorld,
    checkpoints: Vec<Checkpoint>,
    interactables: Vec<Interactable>,
}

pub struct DemoGame {
    player: Player,
    level: Option<LevelState>,
    camera: Option<CameraId>,
    initialized: bool,

    pumps: [bool; 3],
    reactor_online: bool,
    timer_started: bool,
    elapsed: f32,
    best_time: Option<f32>,
    deaths: u32,
    respawn_position: Vec3,
    respawn_yaw: f32,
    active_checkpoint: Option<usize>,
    show_win: bool,

    message: String,
    message_timer: f32,
    flash: f32,

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
}

impl DemoGame {
    pub fn new() -> Self {
        Self {
            player: Player::new(level::SPAWN, level::SPAWN_YAW),
            level: None,
            camera: None,
            initialized: false,
            pumps: [false; 3],
            reactor_online: false,
            timer_started: false,
            elapsed: 0.0,
            best_time: None,
            deaths: 0,
            respawn_position: level::SPAWN,
            respawn_yaw: level::SPAWN_YAW,
            active_checkpoint: None,
            show_win: false,
            message: String::new(),
            message_timer: 0.0,
            flash: 0.0,
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
        }
    }

    fn begin_run(&mut self) {
        self.pumps = [false; 3];
        self.reactor_online = false;
        self.timer_started = false;
        self.elapsed = 0.0;
        self.deaths = 0;
        self.respawn_position = level::SPAWN;
        self.respawn_yaw = level::SPAWN_YAW;
        self.active_checkpoint = None;
        self.show_win = false;
        self.message.clear();
        self.message_timer = 0.0;
        self.flash = 0.0;
        self.player.teleport(level::SPAWN, level::SPAWN_YAW);
        self.player.set_flying(false);
        self.joystick.reset();
        self.touch_camera.reset();
    }

    fn sync_camera(&self, world: &mut World) {
        let camera = self.player.camera();
        if let Some(named) = world.active_camera_mut() {
            named.camera = camera;
        }
    }

    fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.message_timer = MESSAGE_TIME;
    }

    fn active_pump_count(&self) -> usize {
        self.pumps.iter().filter(|&&active| active).count()
    }

    fn all_pumps_online(&self) -> bool {
        self.pumps.iter().all(|&active| active)
    }

    fn device_done(&self, device: Device) -> bool {
        match device {
            Device::CoolantPump(index) => self.pumps.get(index).copied().unwrap_or(true),
            Device::Reactor => self.reactor_online,
        }
    }

    /// Closest interactable in front of the player, if any.
    fn nearby_interaction(&self) -> Option<usize> {
        let level = self.level.as_ref()?;
        let eye = self.player.eye_position();
        let look = self.player.look_direction();
        let mut best = None;
        let mut best_score = f32::NEG_INFINITY;
        for (index, interactable) in level.interactables.iter().enumerate() {
            if self.device_done(interactable.device) {
                continue;
            }
            let offset = interactable.position - eye;
            let distance = offset.length();
            if distance > interactable.radius || distance < 1e-4 {
                continue;
            }
            let facing = look.dot(offset / distance);
            if facing < 0.45 {
                continue;
            }
            let score = facing - distance * 0.015;
            if score > best_score {
                best_score = score;
                best = Some(index);
            }
        }
        best
    }

    fn interaction_prompt(&self, index: usize) -> (String, bool) {
        let Some(level) = self.level.as_ref() else {
            return (String::new(), false);
        };
        let Some(interactable) = level.interactables.get(index) else {
            return (String::new(), false);
        };
        match interactable.device {
            Device::CoolantPump(_) => (interactable.prompt.to_owned(), true),
            Device::Reactor if self.all_pumps_online() => (interactable.prompt.to_owned(), true),
            Device::Reactor => (
                "Reactor interlock: restart all three coolant pumps".to_owned(),
                false,
            ),
        }
    }

    fn player_bounds(&self) -> Aabb {
        let position = self.player.body.position;
        Aabb::new(
            position - Vec3::new(PLAYER_RADIUS, 0.0, PLAYER_RADIUS),
            position + Vec3::new(PLAYER_RADIUS, PLAYER_HEIGHT, PLAYER_RADIUS),
        )
    }

    fn check_checkpoint(&mut self) {
        let bounds = self.player_bounds();
        let mut entered = None;
        if let Some(level) = self.level.as_ref() {
            for (index, checkpoint) in level.checkpoints.iter().enumerate() {
                if checkpoint.trigger.overlaps(&bounds) {
                    entered = Some(index);
                    break;
                }
            }
        }
        let Some(index) = entered else {
            return;
        };
        if self.active_checkpoint == Some(index) {
            return;
        }
        let Some(checkpoint) = self
            .level
            .as_ref()
            .and_then(|level| level.checkpoints.get(index))
            .cloned()
        else {
            return;
        };
        self.active_checkpoint = Some(index);
        self.respawn_position = checkpoint.position;
        self.respawn_yaw = checkpoint.yaw;
        self.set_message(format!("Checkpoint — {}", checkpoint.name));
    }

    fn respawn(&mut self) {
        self.deaths += 1;
        self.player
            .teleport(self.respawn_position, self.respawn_yaw);
        self.player.set_flying(false);
        self.flash = 1.0;
        self.joystick.reset();
        self.touch_camera.reset();
        self.set_message("Recovered at the last checkpoint");
    }

    fn activate(&mut self, index: usize) {
        let Some(device) = self
            .level
            .as_ref()
            .and_then(|level| level.interactables.get(index))
            .map(|interactable| interactable.device)
        else {
            return;
        };
        match device {
            Device::CoolantPump(pump) => {
                if let Some(slot) = self.pumps.get_mut(pump)
                    && !*slot
                {
                    *slot = true;
                    let label = ['A', 'B', 'C'].get(pump).copied().unwrap_or('?');
                    if self.all_pumps_online() {
                        self.set_message("All coolant pumps online — reactor interlocks released");
                    } else {
                        self.set_message(format!(
                            "Coolant pump {label} online ({}/3)",
                            self.active_pump_count()
                        ));
                    }
                }
            }
            Device::Reactor => {
                if !self.all_pumps_online() {
                    self.set_message("Reactor interlock: restart all three coolant pumps");
                    return;
                }
                if self.reactor_online {
                    return;
                }
                self.reactor_online = true;
                self.show_win = true;
                self.best_time = Some(match self.best_time {
                    Some(best) => best.min(self.elapsed),
                    None => self.elapsed,
                });
                self.set_message("Reactor stable — facility online");
                self.joystick.reset();
                self.touch_camera.reset();
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
}

impl Default for DemoGame {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for DemoGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("SOL-3 // Soldatik3D demo")
            .with_size(1280, 720)
            .with_cursor_locked(!cfg!(target_os = "android"))
    }

    fn create_world(&mut self) -> World {
        let Facility {
            scene,
            physics,
            checkpoints,
            interactables,
        } = level::build_facility();
        self.level = Some(LevelState {
            physics,
            checkpoints,
            interactables,
        });

        let mut cameras = CameraRig::empty();
        self.camera = Some(cameras.spawn("player", self.player.camera()));
        if !self.initialized {
            self.initialized = true;
            self.begin_run();
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
            "game.status" => Some(Ok(format!(
                "pumps {}/3 · reactor {} · time {} · best {}",
                self.active_pump_count(),
                if self.reactor_online {
                    "online"
                } else {
                    "offline"
                },
                format_time(self.elapsed),
                self.best_time
                    .map(format_time)
                    .unwrap_or_else(|| "--:--.--".to_owned())
            ))),
            "game.restart" => {
                self.begin_run();
                self.paused = false;
                self.sync_camera(ctx.world);
                Some(Ok("facility run restarted".to_owned()))
            }
            "game.pumps" => {
                self.pumps = [true; 3];
                self.set_message("Debug: all coolant pumps online");
                Some(Ok("all coolant pumps activated".to_owned()))
            }
            "game.win" => {
                self.pumps = [true; 3];
                self.reactor_online = true;
                self.timer_started = true;
                self.show_win = true;
                self.paused = false;
                self.paused_by_cursor = false;
                Some(Ok("debug: reactor completed".to_owned()))
            }
            "player.teleport" => {
                let x = match Self::parse_number(ctx.args, 0) {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return Some(Err("usage: player.teleport x y z [yaw] [pitch]".to_owned()));
                    }
                    Err(error) => return Some(Err(error)),
                };
                let y = match Self::parse_number(ctx.args, 1) {
                    Ok(Some(value)) => value,
                    _ => return Some(Err("missing y".to_owned())),
                };
                let z = match Self::parse_number(ctx.args, 2) {
                    Ok(Some(value)) => value,
                    _ => return Some(Err("missing z".to_owned())),
                };
                let yaw = match Self::parse_number(ctx.args, 3) {
                    Ok(value) => value.unwrap_or(self.player.yaw),
                    Err(error) => return Some(Err(error)),
                };
                let pitch = match Self::parse_number(ctx.args, 4) {
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
                    .teleport(self.respawn_position, self.respawn_yaw);
                self.player.set_flying(false);
                self.sync_camera(ctx.world);
                Some(Ok("player returned to the active checkpoint".to_owned()))
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

        // Releasing the desktop cursor (Escape / Tab) opens the pause menu;
        // locking it again with Tab resumes automatically.
        if !touch_mode && !ctx.input.cursor_captured() && !ctx.console_open {
            if !self.paused {
                self.paused = true;
                self.paused_by_cursor = true;
                self.joystick.reset();
                self.touch_camera.reset();
            }
        } else if !touch_mode
            && ctx.input.cursor_captured()
            && self.paused_by_cursor
            && !self.show_win
        {
            self.paused = false;
            self.paused_by_cursor = false;
        }

        // P is an explicit pause toggle for keyboard play. Resuming is only
        // useful while the cursor is captured (or on touch), because a released
        // cursor immediately re-opens the pause menu on the next frame.
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
            self.begin_run();
            self.paused = false;
            self.paused_by_cursor = false;
            self.sync_camera(ctx.world);
        }

        self.message_timer = (self.message_timer - dt).max(0.0);
        self.flash = (self.flash - dt / FLASH_TIME).max(0.0);

        if self.level.is_none() {
            return;
        }

        let mut controls_active = !self.paused && !self.show_win && !ctx.console_open;
        let mut axes = Vec2::ZERO;
        let mut jump_pressed = false;
        let mut interact_pressed = false;
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
                let use_button = ctx.ui.button_with(
                    Layout::default().absolute(Rect::new(
                        width - margin - size * 2.0 - gap,
                        height - margin - size,
                        size,
                        size,
                    )),
                    "USE",
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
                if jump.pressed {
                    jump_pressed = true;
                }
                if use_button.pressed {
                    interact_pressed = true;
                }

                let mut joystick_style = JoystickStyle::from_theme(ctx.ui.theme()).scaled(scale);
                joystick_style.area = Some(Rect::new(0.0, 0.0, width * 0.45, height));
                axes = ctx.ui.virtual_joystick(&mut self.joystick, &joystick_style);

                // The look area is claimed last, so buttons and the joystick
                // always win their own fingers.
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

            if !touch_mode {
                // Desktop keyboard camera controls remain available even when
                // the cursor is locked.
                self.player.look_keyboard(ctx.input, dt);
            } else {
                self.player.look_keyboard(ctx.input, dt);
            }

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
            if ctx.input.action_pressed("interact") {
                interact_pressed = true;
            }
            if ctx.input.action_pressed("fly_toggle") {
                fly_toggle = true;
            }
        }

        if fly_toggle {
            self.player.set_flying(!self.player.is_flying());
        }

        controls_active = controls_active && !self.paused && !self.show_win;
        if controls_active {
            let crouch = ctx.input.action_down("crouch");
            let sprint = ctx.input.action_down("sprint")
                || (touch_mode && self.joystick.active && self.joystick.value.length() > 0.92);

            if let Some(level) = self.level.as_ref() {
                self.player
                    .update(dt, &level.physics, axes, jump_pressed, crouch, sprint);
            }

            if self.player.body.position.y < level::FALL_LIMIT {
                self.respawn();
            } else {
                self.check_checkpoint();
            }

            if !self.timer_started && (axes.length_squared() > 0.01 || jump_pressed) {
                self.timer_started = true;
            }
            if self.timer_started && !self.reactor_online {
                self.elapsed += dt;
            }

            if interact_pressed && let Some(index) = self.nearby_interaction() {
                self.activate(index);
            }
            self.sync_camera(ctx.world);
        }

        let prompt = if controls_active {
            self.nearby_interaction()
        } else {
            None
        };
        self.draw_hud(ctx.ui, width, scale, touch_mode, prompt);
        if controls_active {
            self.draw_reticle(ctx.ui, width, height, scale, prompt.is_some());
            self.draw_interaction_prompt(ctx.ui, width, height, scale, touch_mode, prompt);
            self.draw_toast(ctx.ui, width, scale);
        }
        if self.flash > 0.0 {
            let alpha = self.flash * self.flash * 0.32;
            ctx.ui
                .rect(0.0, 0.0, width, height, [0.75, 0.08, 0.04, alpha]);
        }
        if self.show_win {
            self.draw_win(ctx.ui, width, height, scale, touch_mode);
        } else if self.paused {
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

// ---------------------------------------------------------------------------
// HUD and overlays
// ---------------------------------------------------------------------------

impl DemoGame {
    fn checkpoint_name(&self) -> &'static str {
        self.active_checkpoint
            .and_then(|index| self.level.as_ref()?.checkpoints.get(index))
            .map(|checkpoint| checkpoint.name)
            .unwrap_or("Atrium")
    }

    fn draw_hud(
        &self,
        ui: &mut Ui,
        width: f32,
        scale: f32,
        touch_mode: bool,
        _prompt: Option<usize>,
    ) {
        let text = ui.theme().text;
        let muted = ui.theme().text_muted;
        let accent = ui.theme().accent;
        let success = ui.theme().success;
        let panel = ui.theme().panel;
        let border = ui.theme().panel_border;

        let margin = 22.0 * scale;
        let panel_fill = [panel[0], panel[1], panel[2], 0.72];
        let panel_edge = [border[0], border[1], border[2], 0.55];

        // Objective card, top left.
        let card_w = (270.0 * scale)
            .min(width * 0.28)
            .max(140.0)
            .min(width - margin * 2.0);
        let card_h = 116.0 * scale;
        let card = Rect::new(margin, margin, card_w, card_h);
        draw_hud_card(ui, card, scale, panel_fill, panel_edge);

        let pad = 13.0 * scale;
        let inner_w = (card_w - pad * 2.0).max(1.0);
        let title_size = (9.0 * scale).min(card_w * 0.07).max(7.0);
        let line_size = (11.0 * scale).min(card_w * 0.09).max(8.0);
        let small_size = (9.5 * scale).min(card_w * 0.085).max(7.0);
        let tiny_size = (8.0 * scale).min(card_w * 0.055).max(6.0);

        ui.text(
            "SOL-3 // FACILITY",
            card.x + pad,
            card.y + 8.0 * scale,
            title_size,
            [accent[0], accent[1], accent[2], 0.95],
        );
        ui.text(
            "RESTART COOLANT PUMPS",
            card.x + pad,
            card.y + 27.0 * scale,
            line_size,
            text,
        );

        // Pump chips: filled when the pump is running.
        let chip_gap = 6.0 * scale;
        let chip_w = ((inner_w - chip_gap * 2.0) / 3.0).clamp(1.0, 26.0 * scale);
        let chip_h = 18.0 * scale;
        let chip_y = card.y + 52.0 * scale;
        for (index, label) in ["A", "B", "C"].iter().enumerate() {
            let active = self.pumps.get(index).copied().unwrap_or(false);
            let chip = Rect::new(
                card.x + pad + index as f32 * (chip_w + chip_gap),
                chip_y,
                chip_w,
                chip_h,
            );
            if active {
                ui.rect_rounded(
                    chip.x,
                    chip.y,
                    chip.w,
                    chip.h,
                    4.0 * scale,
                    [success[0], success[1], success[2], 0.9],
                );
            } else {
                ui.rect_rounded(
                    chip.x,
                    chip.y,
                    chip.w,
                    chip.h,
                    4.0 * scale,
                    [0.10, 0.12, 0.16, 0.9],
                );
                ui.rect_outline(chip, 1.0, [border[0], border[1], border[2], 0.8]);
            }
            let letter_size = (chip_h * 0.55).max(6.0);
            let measured = ui.measure_text(label, letter_size);
            ui.text(
                label,
                chip.center().x - measured.x * 0.5,
                chip.center().y - measured.y * 0.5,
                letter_size,
                if active {
                    [0.02, 0.06, 0.03, 1.0]
                } else {
                    muted
                },
            );
        }
        let status_size = (chip_h * 0.45).max(6.0);
        let count = format!("{}/3", self.active_pump_count());
        let count_w = ui.text_width(&count, status_size);
        let chips_end = card.x + pad + chip_w * 3.0 + chip_gap * 2.0;
        if chips_end + count_w + 6.0 * scale < card.right() - pad {
            ui.text(
                &count,
                card.right() - pad - count_w,
                chip_y + (chip_h - status_size * 1.25) * 0.5,
                status_size,
                muted,
            );
        }

        let (reactor_text, reactor_color) = if self.reactor_online {
            ("REACTOR ONLINE", success)
        } else if self.all_pumps_online() {
            ("REACTOR READY", accent)
        } else {
            ("REACTOR LOCKED", [1.0, 0.72, 0.34, 1.0])
        };
        ui.text(
            reactor_text,
            card.x + pad,
            card.y + 78.0 * scale,
            small_size,
            reactor_color,
        );
        ui.text(
            &format!("CP · {}", self.checkpoint_name()),
            card.x + pad,
            card.y + 98.0 * scale,
            tiny_size,
            [muted[0], muted[1], muted[2], 0.75],
        );

        // Timer and best time, top centre.
        let timer_size = (22.0 * scale).min(width * 0.045).max(12.0);
        let timer = format_time(self.elapsed);
        let timer_w = ui.text_width(&timer, timer_size);
        let best = self
            .best_time
            .map(|time| format!("BEST {}", format_time(time)))
            .unwrap_or_else(|| "BEST --:--.--".to_owned());
        let best_size = (8.5 * scale).max(7.0);
        let best_w = ui.text_width(&best, best_size);
        let timer_card_w = timer_w.max(best_w) + 28.0 * scale;
        let timer_card_h = 54.0 * scale;
        let timer_card = Rect::new(
            (width - timer_card_w) * 0.5,
            margin,
            timer_card_w,
            timer_card_h,
        );
        draw_hud_card(ui, timer_card, scale, panel_fill, panel_edge);
        ui.text(
            &timer,
            timer_card.x + (timer_card_w - timer_w) * 0.5,
            timer_card.y + 7.0 * scale,
            timer_size,
            if self.reactor_online { success } else { text },
        );
        ui.text(
            &best,
            timer_card.x + (timer_card_w - best_w) * 0.5,
            timer_card.y + 36.0 * scale,
            best_size,
            if self.best_time.is_some() {
                success
            } else {
                muted
            },
        );

        if !touch_mode {
            // Compact control legend and frame rate, desktop only. The touch
            // layout has visible buttons and no room for reference text.
            let hint_size = (9.0 * scale).max(7.0);
            let hint = "WASD move · SPACE jump · SHIFT sprint · CTRL crouch · E use · R restart · TAB menu";
            let hint_w = ui.text_width(hint, hint_size);
            if hint_w < width * 0.62 {
                ui.text(
                    hint,
                    margin,
                    ui.height() - margin - hint_size * 1.35,
                    hint_size,
                    [muted[0], muted[1], muted[2], 0.68],
                );
            }
            let fps = if self.fps > 0.0 {
                format!("{:.0} FPS", self.fps)
            } else {
                "-- FPS".to_owned()
            };
            let fps_w = ui.text_width(&fps, hint_size);
            ui.text(
                &fps,
                width - margin - fps_w,
                ui.height() - margin - hint_size * 1.35,
                hint_size,
                [muted[0], muted[1], muted[2], 0.55],
            );
        }
    }

    fn draw_reticle(&self, ui: &mut Ui, width: f32, height: f32, scale: f32, active: bool) {
        let accent = ui.theme().accent;
        let color = if active {
            [accent[0], accent[1], accent[2], 0.95]
        } else {
            [0.92, 0.94, 0.97, 0.55]
        };
        let cx = width * 0.5;
        let cy = height * 0.5;
        let gap = 4.0 * scale;
        let arm = 7.0 * scale;
        let thickness = (1.5 * scale).max(1.0);
        ui.rect(cx - gap - arm, cy - thickness * 0.5, arm, thickness, color);
        ui.rect(cx + gap, cy - thickness * 0.5, arm, thickness, color);
        ui.rect(cx - thickness * 0.5, cy - gap - arm, thickness, arm, color);
        ui.rect(cx - thickness * 0.5, cy + gap, thickness, arm, color);
        if active {
            let dot = (2.5 * scale).max(1.5);
            ui.rect(cx - dot * 0.5, cy - dot * 0.5, dot, dot, color);
        }
    }

    fn draw_interaction_prompt(
        &self,
        ui: &mut Ui,
        width: f32,
        height: f32,
        scale: f32,
        touch_mode: bool,
        prompt: Option<usize>,
    ) {
        let Some((message, enabled)) = prompt.map(|index| self.interaction_prompt(index)) else {
            return;
        };
        let accent = ui.theme().accent;
        let text = ui.theme().text;
        let muted = ui.theme().text_muted;
        let key = if touch_mode { "USE" } else { "[E]" };
        let label = format!("{key}  {message}");

        let max_width = (width - 56.0 * scale).max(120.0);
        let mut font = 13.0 * scale;
        let mut label_w = ui.text_width(&label, font);
        while label_w > max_width - 30.0 * scale && font > 9.0 * scale {
            font -= 1.0 * scale;
            label_w = ui.text_width(&label, font);
        }
        let card_w = (label_w + 30.0 * scale).min(max_width);
        let card_h = 34.0 * scale;
        let y = if touch_mode {
            height - 168.0 * scale
        } else {
            height - 96.0 * scale
        };
        let card = Rect::new((width - card_w) * 0.5, y, card_w, card_h);
        draw_hud_card(
            ui,
            card,
            scale,
            [0.035, 0.045, 0.065, 0.86],
            if enabled {
                [accent[0], accent[1], accent[2], 0.8]
            } else {
                [1.0, 0.72, 0.34, 0.75]
            },
        );
        ui.rect(
            card.x,
            card.y + 4.0 * scale,
            3.0 * scale,
            card.h - 8.0 * scale,
            if enabled {
                accent
            } else {
                [1.0, 0.72, 0.34, 1.0]
            },
        );
        let measured = ui.measure_text(&label, font);
        ui.text(
            &label,
            card.x + 15.0 * scale,
            card.y + (card.h - measured.y) * 0.5,
            font,
            if enabled {
                text
            } else {
                [1.0, 0.78, 0.46, 1.0]
            },
        );
        if !enabled {
            // Keep the lock reason readable without competing with the prompt.
            let hint = "restart the pumps first";
            let hint_size = (8.0 * scale).max(6.0);
            let hint_w = ui.text_width(hint, hint_size);
            ui.text(
                hint,
                card.right() - 12.0 * scale - hint_w,
                card.y - hint_size * 1.5,
                hint_size,
                muted,
            );
        }
    }

    fn draw_toast(&self, ui: &mut Ui, width: f32, scale: f32) {
        if self.message_timer <= 0.0 || self.message.is_empty() {
            return;
        }
        let accent = ui.theme().accent;
        let text = ui.theme().text;
        let alpha = (self.message_timer / 0.35).min(1.0);
        let font = 12.0 * scale;
        let message_w = ui.text_width(&self.message, font);
        let card_w = (message_w + 34.0 * scale).min(width - 44.0 * scale);
        let card_h = 32.0 * scale;
        let card = Rect::new(
            (width - card_w) * 0.5,
            22.0 * scale + 120.0 * scale,
            card_w,
            card_h,
        );
        draw_hud_card(
            ui,
            card,
            scale,
            [0.035, 0.045, 0.065, 0.78 * alpha],
            [accent[0], accent[1], accent[2], 0.5 * alpha],
        );
        ui.rect(
            card.x,
            card.y + 4.0 * scale,
            3.0 * scale,
            card.h - 8.0 * scale,
            [accent[0], accent[1], accent[2], alpha],
        );
        let measured = ui.measure_text(&self.message, font);
        ui.text(
            &self.message,
            card.x + 15.0 * scale,
            card.y + (card.h - measured.y) * 0.5,
            font,
            [text[0], text[1], text[2], alpha],
        );
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
        let stats = format!(
            "{}  ·  pumps {}/3  ·  falls {}",
            format_time(self.elapsed),
            self.active_pump_count(),
            self.deaths
        );
        ui.label_styled(stats, LabelStyle::new(12.0 * scale).color(theme.text_muted));
        ui.label_styled(
            format!("Checkpoint · {}", self.checkpoint_name()),
            LabelStyle::new(11.0 * scale).color(theme.text_muted),
        );

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
        let restart = ui.button_with(
            Layout::default().width(button_w),
            "Restart run",
            if resume.is_some() { regular } else { primary },
        );
        ui.end_container();
        if resume.as_ref().is_some_and(|response| response.clicked) {
            self.paused = false;
            self.paused_by_cursor = false;
            self.joystick.reset();
            self.touch_camera.reset();
        }
        if restart.clicked {
            self.begin_run();
            self.paused = false;
            self.paused_by_cursor = false;
        }
        if !touch_mode {
            let hint = if cursor_captured {
                "P resumes · TAB releases the cursor · R restarts"
            } else {
                "TAB resumes · R restarts"
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

    fn draw_win(&mut self, ui: &mut Ui, width: f32, height: f32, scale: f32, touch_mode: bool) {
        let theme = ui.theme().clone();
        ui.rect(0.0, 0.0, width, height, [0.008, 0.014, 0.02, 0.66]);

        let panel_w = (440.0 * scale).min(width - 44.0 * scale).max(240.0);
        let pad = 24.0 * scale;
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
        ui.heading("Reactor online");
        ui.label_styled(
            "All coolant loops stable. The facility is back under control.",
            LabelStyle::new(12.0 * scale)
                .color(theme.text_muted)
                .align(TextAlign::Center),
        );
        ui.label_styled(
            format!("Completed in {}", format_time(self.elapsed)),
            LabelStyle::new(25.0 * scale).color(theme.success),
        );
        ui.label_styled(
            match self.best_time {
                Some(best) => format!("Best run · {}", format_time(best)),
                None => "First completed run".to_owned(),
            },
            LabelStyle::new(12.0 * scale).color(theme.text),
        );
        ui.label_styled(
            format!(
                "Pumps 3/3 · checkpoint {} · falls {}",
                self.checkpoint_name(),
                self.deaths
            ),
            LabelStyle::new(10.5 * scale).color(theme.text_muted),
        );

        let primary = ButtonStyle::primary(&theme);
        let regular = ButtonStyle::from_theme(&theme);
        ui.begin_container(Layout::row().gap(gap));
        let again = ui.button_with(Layout::default().width(button_w), "Run again", primary);
        let explore = ui.button_with(Layout::default().width(button_w), "Keep exploring", regular);
        ui.end_container();
        if again.clicked {
            self.begin_run();
            self.paused = false;
            self.paused_by_cursor = false;
        }
        if explore.clicked {
            self.show_win = false;
            self.paused = false;
            self.paused_by_cursor = false;
        }
        ui.label_styled(
            if touch_mode {
                "Tap Run again to beat your time"
            } else {
                "R restarts · TAB releases the cursor"
            },
            LabelStyle::new(9.5 * scale)
                .color(theme.text_muted)
                .align(TextAlign::Center),
        );
        ui.end_container();
        ui.end_container();
    }
}

fn draw_hud_card(ui: &mut Ui, rect: Rect, scale: f32, fill: [f32; 4], border: [f32; 4]) {
    ui.rect_rounded(rect.x, rect.y, rect.w, rect.h, 7.0 * scale, fill);
    ui.rect_outline(rect, (1.0 * scale).max(1.0), border);
}

fn format_time(seconds: f32) -> String {
    // Truncate instead of rounding so the hundredths place never carries into
    // the seconds display (59:59.999 must read 59:59.99, not 59:60.00).
    let centiseconds = (seconds.max(0.0) * 100.0).floor() as u64;
    let minutes = centiseconds / 6000;
    let remainder = (centiseconds % 6000) as f32 / 100.0;
    format!("{minutes:02}:{remainder:05.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_format_is_stable() {
        assert_eq!(format_time(0.0), "00:00.00");
        assert_eq!(format_time(61.5), "01:01.50");
        assert_eq!(format_time(3599.999), "59:59.99");
    }
}
