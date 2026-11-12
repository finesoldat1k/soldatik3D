//! Debug overlay and mode selector for the demo.

use engine::glam::Vec3;
use engine::winit::keyboard::KeyCode;
use engine::{DebugView, Input, RenderStats, UiDrawList};

struct Button {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    view: DebugView,
    label: &'static str,
}

pub struct DebugMenu {
    pub open: bool,
    pub view: DebugView,
    buttons: [Button; 3],
}

impl Default for DebugMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugMenu {
    pub fn new() -> Self {
        let view = match std::env::var("SOLDATIK3D_DEBUG_VIEW").as_deref() {
            Ok("normals") => DebugView::Normals,
            Ok("depth") => DebugView::Depth,
            _ => DebugView::Lit,
        };
        Self {
            open: false,
            view,
            buttons: [
                Button {
                    x: 20.0,
                    y: 195.0,
                    width: 90.0,
                    height: 26.0,
                    view: DebugView::Lit,
                    label: "LIT",
                },
                Button {
                    x: 120.0,
                    y: 195.0,
                    width: 90.0,
                    height: 26.0,
                    view: DebugView::Normals,
                    label: "NORMAL",
                },
                Button {
                    x: 220.0,
                    y: 195.0,
                    width: 90.0,
                    height: 26.0,
                    view: DebugView::Depth,
                    label: "DEPTH",
                },
            ],
        }
    }

    pub fn handle_shortcuts(&mut self, input: &Input) {
        if input.key_pressed(KeyCode::F2) {
            self.view = DebugView::Lit;
        }
        if input.key_pressed(KeyCode::F3) {
            self.view = DebugView::Normals;
        }
        if input.key_pressed(KeyCode::F4) {
            self.view = DebugView::Depth;
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut UiDrawList,
        input: &Input,
        stats: RenderStats,
        camera_position: Vec3,
        fps: f32,
        frame_ms: f32,
    ) {
        if !self.open {
            ui.crosshair();
            return;
        }

        if input.primary_clicked() {
            let cursor = input.cursor_position();
            for button in &self.buttons {
                if cursor.x >= button.x
                    && cursor.x <= button.x + button.width
                    && cursor.y >= button.y
                    && cursor.y <= button.y + button.height
                {
                    self.view = button.view;
                }
            }
        }

        let panel_width = 320.0;
        let panel_height = 300.0;
        ui.rect(
            10.0,
            10.0,
            panel_width,
            panel_height,
            [0.04, 0.04, 0.05, 0.92],
        );
        ui.rect(10.0, 10.0, panel_width, 2.0, [1.0, 0.55, 0.1, 1.0]);

        let white = [1.0, 1.0, 1.0, 1.0];
        let dim = [0.65, 0.65, 0.65, 1.0];
        let accent = [1.0, 0.75, 0.2, 1.0];

        ui.text("SOLDATIK3D DEBUG", 20.0, 20.0, 2.0, accent);
        ui.rect(20.0, 40.0, panel_width - 20.0, 1.0, [0.65, 0.45, 0.15, 1.0]);

        let lines = [
            format!("FPS: {fps:.0}"),
            format!("FRAME: {frame_ms:.2} MS"),
            format!("VERTS: {}", stats.vertices),
            format!("TRIS: {}", stats.triangles),
            format!("DRAWS: {}", stats.draw_calls),
            format!(
                "CAM: {:.1} {:.1} {:.1}",
                camera_position.x, camera_position.y, camera_position.z
            ),
        ];
        for (index, line) in lines.iter().enumerate() {
            ui.text(line, 20.0, 48.0 + index as f32 * 18.0, 1.5, white);
        }

        ui.rect(
            20.0,
            175.0,
            panel_width - 20.0,
            1.0,
            [0.35, 0.35, 0.35, 1.0],
        );
        for button in &self.buttons {
            let active = button.view == self.view;
            let color = if active {
                [0.15, 0.65, 0.25, 1.0]
            } else {
                [0.22, 0.22, 0.24, 1.0]
            };
            ui.rect(button.x, button.y, button.width, button.height, color);
            ui.text(button.label, button.x + 8.0, button.y + 7.0, 1.4, white);
        }

        ui.rect(
            20.0,
            232.0,
            panel_width - 20.0,
            1.0,
            [0.35, 0.35, 0.35, 1.0],
        );
        ui.text("TAB: FREE MOUSE", 20.0, 244.0, 1.4, dim);
        ui.text("WASD SPACE CTRL", 20.0, 259.0, 1.4, dim);
        ui.text("SHIFT: SPRINT   F: FLY", 20.0, 274.0, 1.4, dim);
        ui.text("F2 F3 F4: VIEW   ESC: QUIT", 20.0, 289.0, 1.4, dim);
    }
}
