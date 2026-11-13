//! Demo game implementation of the engine `Game` trait.

use runtime::glam::Vec3;
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
];

pub struct DemoGame {
    player: Player,
    colliders: Vec<Aabb>,
    menu: DebugMenu,
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
                Some(Ok(format!("debug view: {:?}", view)))
            }
            "debug.cycle" => {
                self.menu.view = match self.menu.view {
                    DebugView::Lit => DebugView::Normals,
                    DebugView::Normals => DebugView::Depth,
                    DebugView::Depth => DebugView::Lit,
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
            _ => None,
        }
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        self.menu.handle_shortcuts(ctx.input);

        let player_active = self
            .player_camera
            .is_some_and(|id| ctx.world.active_camera_id() == Some(id));
        if player_active && ctx.input.cursor_captured() && !ctx.console_open {
            self.player.update(ctx.dt, ctx.input, &self.colliders);
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
    }
}
