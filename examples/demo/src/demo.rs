//! Demo game implementation of the engine `Game` trait.

use runtime::glam::{Vec2, Vec3};
use runtime::ui::{
    ButtonStyle, DropdownStyle, JoystickState, JoystickStyle, LabelStyle, Layout, Rect,
    TouchCameraState, TouchCameraStyle, Ui,
};
use runtime::{
    Aabb, CameraRig, CommandContext, DebugView, Game, GameConfig, InputMap, UpdateContext, World,
};

use crate::debug_ui::DebugMenu;
use crate::player::{Player, default_input_map};
use crate::world;

const GAME_COMMANDS: &[(&str, &str)] = &[
    ("debug.lit", "switch the forward pass to the lit view"),
    ("debug.normals", "show world-space normals"),
    ("debug.depth", "show the depth buffer"),
    ("debug.cycle", "cycle lit -> normals -> depth"),
    ("player.camera", "select the first-person camera"),
    (
        "player.teleport x y z [yaw] [pitch]",
        "move the player camera",
    ),
    ("player.fly [on|off|toggle]", "toggle free flight"),
    ("player.reset", "return to the spawn point"),
    ("ui.settings", "open settings menu"),
];

pub struct DemoGame {
    player: Player,
    colliders: Vec<Aabb>,
    menu: DebugMenu,
    settings_open: bool,
    vsync_enabled: bool,
    debug_view_items: Vec<&'static str>,
    debug_view_selected: usize,
    joystick: JoystickState,
    touch_camera: TouchCameraState,
    fps: f32,
    frame_ms: f32,
    fps_accumulator: f32,
    fps_frames: u32,
    player_camera: Option<runtime::CameraId>,
}

impl DemoGame {
    pub fn new() -> Self {
        Self {
            player: Player::new(),
            colliders: Vec::new(),
            menu: DebugMenu::new(),
            settings_open: false,
            vsync_enabled: true,
            debug_view_items: vec!["LIT", "NORMALS", "DEPTH"],
            debug_view_selected: 0,
            joystick: JoystickState::default(),
            touch_camera: TouchCameraState::default(),
            fps: 0.0,
            frame_ms: 0.0,
            fps_accumulator: 0.0,
            fps_frames: 0,
            player_camera: None,
        }
    }

    /// Writes the player's owned camera into the world when it is active.
    fn sync_player_camera(&mut self, world: &mut World) {
        let Some(player_id) = self.player_camera else {
            return;
        };
        if world.active_camera_id() != Some(player_id) {
            return;
        }
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
            .with_title("Soldatik3D — modular engine demo")
            .with_size(1280, 720)
    }

    fn create_world(&mut self) -> World {
        let (scene, colliders) = world::build_scene();
        self.colliders = colliders;

        let mut cameras = CameraRig::empty();
        self.player_camera = Some(cameras.spawn("player", self.player.camera()));
        for (name, camera) in world::camera_presets() {
            cameras.spawn(name, camera);
        }
        World::with_cameras(scene, cameras)
    }

    fn debug_view(&self) -> DebugView {
        self.menu.view
    }

    fn input_map(&self) -> InputMap {
        default_input_map()
    }

    fn command_help(&self) -> &'static [(&'static str, &'static str)] {
        GAME_COMMANDS
    }

    fn command(&mut self, ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
        match ctx.name {
            "debug.lit" | "debug.normals" | "debug.depth" => {
                let view = match ctx.name {
                    "debug.normals" => DebugView::Normals,
                    "debug.depth" => DebugView::Depth,
                    _ => DebugView::Lit,
                };
                self.menu.view = view;
                self.debug_view_selected = match view {
                    DebugView::Lit => 0,
                    DebugView::Normals => 1,
                    DebugView::Depth => 2,
                };
                Some(Ok(format!("debug view: {:?}", view)))
            }
            "debug.cycle" => {
                self.menu.view = match self.menu.view {
                    DebugView::Lit => DebugView::Normals,
                    DebugView::Normals => DebugView::Depth,
                    DebugView::Depth => DebugView::Lit,
                };
                self.debug_view_selected = match self.menu.view {
                    DebugView::Lit => 0,
                    DebugView::Normals => 1,
                    DebugView::Depth => 2,
                };
                Some(Ok(format!("debug view: {:?}", self.menu.view)))
            }
            "player.camera" => {
                let id = self.player_camera?;
                ctx.world.select_camera(id);
                Some(Ok("active camera: player".to_owned()))
            }
            "player.teleport" => {
                if ctx.args.len() < 3 {
                    return Some(Err("usage: player.teleport x y z [yaw] [pitch]".to_owned()));
                }
                let mut values = [0.0f32; 5];
                for (index, slot) in values.iter_mut().enumerate() {
                    if let Some(value) = ctx.args.get(index) {
                        match value.parse() {
                            Ok(parsed) => *slot = parsed,
                            Err(_) => {
                                return Some(Err(format!("invalid number: {value}")));
                            }
                        }
                    }
                }
                self.player.teleport(
                    Vec3::new(values[0], values[1], values[2]),
                    ctx.args.get(3).map(|_| values[3]),
                    ctx.args.get(4).map(|_| values[4]),
                );
                self.sync_player_camera(ctx.world);
                Some(Ok(format!(
                    "player at ({:.1}, {:.1}, {:.1})",
                    values[0], values[1], values[2]
                )))
            }
            "player.fly" => {
                let flying = match ctx.args.first().copied() {
                    None | Some("toggle") => !self.player.is_flying(),
                    Some("on" | "true" | "1") => true,
                    Some("off" | "false" | "0") => false,
                    Some(value) => return Some(Err(format!("unknown mode: {value}"))),
                };
                self.player.set_flying(flying);
                Some(Ok(format!(
                    "flight mode: {}",
                    if flying { "on" } else { "off" }
                )))
            }
            "player.reset" => {
                self.player.reset();
                self.sync_player_camera(ctx.world);
                Some(Ok("player returned to spawn".to_owned()))
            }
            "ui.settings" => {
                self.settings_open = !self.settings_open;
                Some(Ok(format!(
                    "settings: {}",
                    if self.settings_open { "open" } else { "closed" }
                )))
            }
            _ => None,
        }
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let screen_w = ctx.viewport.0 as f32;
        let screen_h = ctx.viewport.1 as f32;
        let is_mobile = ctx.input.touch_mode() || cfg!(target_os = "android");
        self.menu.open = !ctx.input.cursor_captured() && !ctx.console_open;

        let player_active = self
            .player_camera
            .is_some_and(|id| ctx.world.active_camera_id() == Some(id));

        let mut move_input = Vec3::ZERO;
        let mut camera_delta = Vec2::ZERO;
        let mut jump_pressed = false;
        let mut fly_toggle = false;

        // HUD layer: full-screen transparent container with absolutely placed
        // touch controls and the settings button.
        let touch_controls = is_mobile && player_active && !ctx.console_open && !self.settings_open;
        ctx.ui.begin_container(Layout::default().fill());
        if touch_controls {
            let joystick_style = JoystickStyle::from_theme(ctx.ui.theme());
            let axes = ctx.ui.virtual_joystick(&mut self.joystick, &joystick_style);
            move_input = Vec3::new(axes.x, 0.0, -axes.y);

            let button_style = ButtonStyle::from_theme(ctx.ui.theme())
                .height(84.0)
                .font_size(18.0);
            let jump = ctx.ui.button_with(
                Layout::default().absolute(Rect::new(
                    screen_w - 108.0,
                    screen_h - 108.0,
                    84.0,
                    84.0,
                )),
                "JUMP",
                button_style.clone(),
            );
            jump_pressed = jump.clicked;

            let fly = ctx.ui.button_with(
                Layout::default().absolute(Rect::new(
                    screen_w - 108.0,
                    screen_h - 204.0,
                    84.0,
                    84.0,
                )),
                "FLY",
                button_style,
            );
            fly_toggle = fly.clicked;
        }

        if is_mobile || self.menu.open {
            let style = ButtonStyle::from_theme(ctx.ui.theme())
                .height(44.0)
                .font_size(20.0);
            let settings = ctx.ui.button_with(
                Layout::default().absolute(Rect::new(screen_w - 60.0, 16.0, 44.0, 44.0)),
                "SET",
                style,
            );
            if settings.clicked {
                self.settings_open = !self.settings_open;
            }
        }

        // The camera is a background gesture for the right half. Call it after
        // the HUD widgets so buttons get the first chance to claim a tap.
        if touch_controls {
            let camera_style = TouchCameraStyle::from_theme(ctx.ui.theme());
            camera_delta = ctx.ui.touch_camera(&mut self.touch_camera, &camera_style);
        }
        ctx.ui.end_container();

        if camera_delta.length_squared() > 0.0 {
            self.player.add_yaw(camera_delta.x);
            self.player.add_pitch(-camera_delta.y);
        }

        if player_active && (ctx.input.cursor_captured() || is_mobile) && !ctx.console_open {
            self.player.update_with_virtual_mobile(
                ctx.dt,
                ctx.input,
                &self.colliders,
                move_input,
                jump_pressed,
                fly_toggle,
            );
            self.sync_player_camera(ctx.world);
        } else {
            self.player.stop();
        }

        self.frame_ms = ctx.dt * 1000.0;
        self.fps_accumulator += ctx.dt;
        self.fps_frames += 1;
        if self.fps_accumulator >= 0.5 {
            self.fps = self.fps_frames as f32 / self.fps_accumulator;
            self.fps_accumulator = 0.0;
            self.fps_frames = 0;
        }

        self.menu.draw(
            ctx.ui,
            ctx.input,
            ctx.stats,
            ctx.world.active_camera().position,
            ctx.world.active_camera_name(),
            self.fps,
            self.frame_ms,
            ctx.console_open,
        );

        if self.settings_open {
            self.draw_settings(ctx.ui, screen_w);
        }
    }
}

impl DemoGame {
    fn draw_settings(&mut self, ui: &mut Ui, screen_w: f32) {
        let panel_w = 300.0;
        let panel_h = 250.0;
        let panel_rect = Rect::new(screen_w - panel_w - 12.0, 70.0, panel_w, panel_h);
        let settings_rect = Rect::new(screen_w - 60.0, 16.0, 44.0, 44.0);
        let outside_click = ui.input().pointers.iter().any(|pointer| {
            pointer.pressed
                && !panel_rect.contains(pointer.position)
                && !settings_rect.contains(pointer.position)
        });

        let background = ui.theme().window;
        let border = ui.theme().panel_border;
        let radius = ui.theme().radius;
        // Full-screen wrapper so the panel can be placed at an absolute
        // position instead of being centered as a root node.
        ui.begin_container(Layout::default().fill());
        ui.begin_styled_container(
            Layout::column().absolute(panel_rect).padding(14.0).gap(8.0),
            background,
            border,
            radius,
        );

        ui.heading("SETTINGS");
        ui.hint("DEBUG VIEW");
        let items: Vec<&str> = self.debug_view_items.clone();
        let style = DropdownStyle::from_theme(ui.theme());
        let response = ui.dropdown_with(
            Layout::default().fill_width(),
            "",
            &items,
            &mut self.debug_view_selected,
            style,
        );
        if response.changed {
            self.menu.view = match self.debug_view_selected {
                0 => DebugView::Lit,
                1 => DebugView::Normals,
                2 => DebugView::Depth,
                _ => DebugView::Lit,
            };
        }

        ui.toggle("VSYNC", &mut self.vsync_enabled);

        let stats_text = format!("FPS: {:.1}\nFrame: {:.2} ms", self.fps, self.frame_ms);
        let stats_style = LabelStyle::new(14.0)
            .color(ui.theme().text_muted)
            .wrap(panel_w - 28.0);
        ui.label_styled(stats_text, stats_style);

        ui.end_container();
        ui.end_container();

        if outside_click {
            self.settings_open = false;
        }
    }
}
