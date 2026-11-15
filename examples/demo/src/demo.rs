//! Demo game implementation of the engine `Game` trait.

use runtime::glam::{Vec2, Vec3};
use runtime::{
    Aabb, CameraRig, CommandContext, DebugView, Game, GameConfig, InputMap, UpdateContext,
    VsyncMode, World,
};

use crate::debug_ui::DebugMenu;
use crate::player::{Player, default_input_map};
use crate::world;
use ui::{Interaction, Rect, Theme, UiContext, WidgetId, widget_id};

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
    // New UI system state
    ui_context: Option<UiContext>,
    settings_open: bool,
    vsync_enabled: bool,
    debug_view_items: Vec<&'static str>,
    debug_view_selected: usize,
    joystick_id: WidgetId,
    camera_id: WidgetId,
    settings_button_id: WidgetId,
    dropdown_id: WidgetId,
    vsync_toggle_id: WidgetId,
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
            ui_context: None,
            settings_open: false,
            vsync_enabled: true,
            debug_view_items: vec!["LIT", "NORMALS", "DEPTH"],
            debug_view_selected: 0,
            joystick_id: widget_id("joystick"),
            camera_id: widget_id("camera"),
            settings_button_id: widget_id("settings_button"),
            dropdown_id: widget_id("debug_dropdown"),
            vsync_toggle_id: widget_id("vsync_toggle"),
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
        // Initialize UI context on first frame
        if self.ui_context.is_none() {
            let theme = Theme::dark();
            let ui_draw = ui::UiDrawList::new(ctx.viewport.0, ctx.viewport.1);
            self.ui_context = Some(UiContext::new(ui_draw, theme));
        }

        // Update UI context with input
        if let Some(ui_ctx) = &mut self.ui_context {
            ui_ctx.update_pointers(ctx.input);
            ui_ctx.draw.resize(ctx.viewport.0, ctx.viewport.1);
        }

        let screen_w = ctx.viewport.0 as f32;
        let screen_h = ctx.viewport.1 as f32;
        let is_mobile = cfg!(target_os = "android");

        let player_active = self
            .player_camera
            .is_some_and(|id| ctx.world.active_camera_id() == Some(id));

        // Handle mobile touch controls
        let mut move_input = Vec3::ZERO;
        let mut camera_delta = Vec2::ZERO;
        let mut jump_pressed = false;
        let mut fly_toggle = false;

        if let Some(ui_ctx) = &mut self.ui_context {
            if is_mobile && player_active && !ctx.console_open {
                // Virtual joystick on left side
                let joy_area = Rect::new(0.0, screen_h * 0.3, screen_w * 0.5, screen_h * 0.7);
                let (stick_output, _interaction) =
                    ui_ctx.virtual_joystick(self.joystick_id, joy_area, 0.15);
                move_input = Vec3::new(stick_output.x, 0.0, stick_output.y);

                // Touch camera on right side
                let cam_area = Rect::new(screen_w * 0.5, 0.0, screen_w * 0.5, screen_h);
                camera_delta = ui_ctx.touch_camera(self.camera_id, cam_area);

                // Jump button (bottom right area)
                let jump_rect = Rect::new(screen_w - 100.0, screen_h - 100.0, 80.0, 80.0);
                if ui_ctx.button(self.joystick_id, jump_rect, "JUMP", 1.2) == Interaction::Clicked {
                    jump_pressed = true;
                }

                // Fly toggle button (above jump)
                let fly_rect = Rect::new(screen_w - 100.0, screen_h - 200.0, 80.0, 80.0);
                if ui_ctx.button(self.vsync_toggle_id, fly_rect, "FLY", 1.2) == Interaction::Clicked
                {
                    fly_toggle = true;
                }

                // Settings button (top right)
                let settings_rect = Rect::new(screen_w - 60.0, 20.0, 40.0, 40.0);
                if ui_ctx.button(self.settings_button_id, settings_rect, "⚙", 1.5)
                    == Interaction::Clicked
                {
                    self.settings_open = !self.settings_open;
                }
            }

            // Apply camera delta from touch
            if camera_delta.length_squared() > 0.0 {
                self.player.add_yaw(camera_delta.x);
                self.player.add_pitch(-camera_delta.y);
            }
        }

        if player_active && (ctx.input.cursor_captured() || is_mobile) && !ctx.console_open {
            // For mobile, we need to inject the virtual joystick input
            // The player update will check for virtual controls via the ui_context
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

        self.menu.open = !ctx.input.cursor_captured() && !ctx.console_open;
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

        // Draw new UI system
        if let Some(mut ui_ctx) = self.ui_context.take() {
            Self::draw_ui_static(&mut ui_ctx, self, ctx, screen_w, screen_h, is_mobile);
            self.ui_context = Some(ui_ctx);
        }
    }
}

impl DemoGame {
    fn draw_ui_static(
        ui_ctx: &mut UiContext,
        game: &mut DemoGame,
        ctx: &mut UpdateContext<'_>,
        screen_w: f32,
        screen_h: f32,
        is_mobile: bool,
    ) {
        // Settings button (always visible on mobile, or when menu is open on desktop)
        if is_mobile || game.menu.open {
            let settings_rect = Rect::new(screen_w - 60.0, 20.0, 40.0, 40.0);
            if ui_ctx.button(game.settings_button_id, settings_rect, "⚙", 1.5)
                == Interaction::Clicked
            {
                game.settings_open = !game.settings_open;
            }
        }

        // Settings dropdown panel
        if game.settings_open {
            let panel_w = 280.0;
            let panel_h = 220.0;
            let panel_x = screen_w - panel_w - 10.0;
            let panel_y = 70.0;

            // Panel background with shadow
            ui_ctx.panel(Rect::new(panel_x, panel_y, panel_w, panel_h));

            // Title
            let title_rect = Rect::new(panel_x + 10.0, panel_y + 10.0, panel_w - 20.0, 30.0);
            ui_ctx.label(title_rect, "SETTINGS", 1.5, None);

            // Debug view dropdown
            let dropdown_rect = Rect::new(panel_x + 10.0, panel_y + 50.0, panel_w - 20.0, 35.0);
            let items: Vec<&str> = game.debug_view_items.iter().map(|s| *s).collect();
            if ui_ctx.dropdown(
                game.dropdown_id,
                dropdown_rect,
                "DEBUG VIEW",
                &items,
                &mut game.debug_view_selected,
                1.2,
            ) == Interaction::Changed
            {
                game.menu.view = match game.debug_view_selected {
                    0 => DebugView::Lit,
                    1 => DebugView::Normals,
                    2 => DebugView::Depth,
                    _ => DebugView::Lit,
                };
            }

            // VSync toggle
            let vsync_rect = Rect::new(panel_x + 10.0, panel_y + 100.0, panel_w - 20.0, 35.0);
            if ui_ctx.toggle(
                game.vsync_toggle_id,
                vsync_rect,
                "VSYNC",
                1.2,
                &mut game.vsync_enabled,
            ) == Interaction::Changed
            {
                // VSync change will be applied on next frame via GameConfig
                // For now, just store the preference
            }

            // Performance stats
            let stats_rect = Rect::new(panel_x + 10.0, panel_y + 150.0, panel_w - 20.0, 50.0);
            let stats_text = format!("FPS: {:.1}\nFrame: {:.2} ms", game.fps, game.frame_ms);
            ui_ctx.label(stats_rect, &stats_text, 1.1, Some(ui_ctx.theme.text));

            // Close on click outside
            if let Some(pointer) = ui_ctx.primary_pointer() {
                let panel_rect = Rect::new(panel_x, panel_y, panel_w, panel_h);
                let settings_rect = Rect::new(screen_w - 60.0, 20.0, 40.0, 40.0);
                if pointer.just_pressed
                    && !panel_rect.contains(pointer.position)
                    && !settings_rect.contains(pointer.position)
                {
                    game.settings_open = false;
                }
            }
        }

        // Mobile-only controls
        if is_mobile {
            // Virtual joystick area (left side)
            let joy_area = Rect::new(0.0, screen_h * 0.3, screen_w * 0.5, screen_h * 0.7);
            let (stick_output, _interaction) =
                ui_ctx.virtual_joystick(game.joystick_id, joy_area, 0.15);

            // Touch camera area (right side)
            let cam_area = Rect::new(screen_w * 0.5, 0.0, screen_w * 0.5, screen_h);
            let _camera_delta = ui_ctx.touch_camera(game.camera_id, cam_area);

            // Jump button
            let jump_rect = Rect::new(screen_w - 100.0, screen_h - 100.0, 80.0, 80.0);
            ui_ctx.button(game.joystick_id, jump_rect, "JUMP", 1.2);

            // Fly toggle button
            let fly_rect = Rect::new(screen_w - 100.0, screen_h - 200.0, 80.0, 80.0);
            ui_ctx.button(game.vsync_toggle_id, fly_rect, "FLY", 1.2);
        }
    }
}
