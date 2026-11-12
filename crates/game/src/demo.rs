//! Demo game implementation of the engine `Game` trait.

use engine::{Camera, DebugView, Game, GameConfig, Scene, UpdateContext};

use crate::debug_ui::DebugMenu;
use crate::player::Player;
use crate::world;

pub struct DemoGame {
    player: Player,
    colliders: Vec<engine::Aabb>,
    menu: DebugMenu,
    fps: f32,
    frame_ms: f32,
    fps_accumulator: f32,
    fps_frames: u32,
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
    }

    fn create_scene(&mut self) -> Scene {
        let (scene, colliders) = world::build_scene();
        self.colliders = colliders;
        scene
    }

    fn camera(&self) -> Camera {
        self.player.camera()
    }

    fn debug_view(&self) -> DebugView {
        self.menu.view
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        self.menu.handle_shortcuts(ctx.input);

        if ctx.input.cursor_captured() {
            self.player.update(ctx.dt, ctx.input, &self.colliders);
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

        self.menu.open = !ctx.input.cursor_captured();
        self.menu.draw(
            ctx.ui,
            ctx.input,
            ctx.stats,
            self.player.camera().position,
            self.fps,
            self.frame_ms,
        );
    }
}
