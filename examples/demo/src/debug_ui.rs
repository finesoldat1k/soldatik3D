//! Debug overlay and mode selector for the demo.

use runtime::glam::Vec3;
use runtime::winit::keyboard::KeyCode;
use runtime::{DebugView, Input, RenderStats, UiDrawList};

use crate::virtual_joystick::VirtualControls;

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
                    y: 200.0,
                    width: 90.0,
                    height: 32.0,
                    view: DebugView::Lit,
                    label: "LIT",
                },
                Button {
                    x: 120.0,
                    y: 200.0,
                    width: 90.0,
                    height: 32.0,
                    view: DebugView::Normals,
                    label: "NORMALS",
                },
                Button {
                    x: 220.0,
                    y: 200.0,
                    width: 90.0,
                    height: 32.0,
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

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut UiDrawList,
        input: &Input,
        stats: RenderStats,
        camera_position: Vec3,
        camera_name: &str,
        fps: f32,
        frame_ms: f32,
        console_open: bool,
    ) {
        if console_open {
            return;
        }
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

        let panel_width = 360.0;
        let panel_height = 360.0;
        let x = 10.0;
        let y = 10.0;

        // Panel background with subtle border
        ui.rect(x, y, panel_width, panel_height, [0.025, 0.03, 0.04, 0.95]);
        ui.rect(x, y, panel_width, 2.0, [1.0, 0.55, 0.1, 1.0]);
        ui.rect(
            x,
            y + panel_height - 1.0,
            panel_width,
            1.0,
            [0.2, 0.2, 0.25, 1.0],
        );

        // Title bar
        ui.rect(x, y + 2.0, panel_width, 36.0, [0.05, 0.06, 0.08, 0.95]);
        ui.text("SOLDATIK3D DEBUG", 20.0, 16.0, 2.0, [1.0, 0.75, 0.2, 1.0]);
        ui.text(
            &format!("CAM: {}", camera_name),
            20.0,
            34.0,
            1.1,
            [0.7, 0.75, 0.8, 1.0],
        );

        let white = [1.0, 1.0, 1.0, 1.0];
        let dim = [0.6, 0.65, 0.7, 1.0];
        let accent = [1.0, 0.75, 0.2, 1.0];
        let _green = [0.3, 0.85, 0.4, 1.0];
        let _red = [0.9, 0.4, 0.3, 1.0];

        // Performance section
        ui.rect(20.0, 55.0, panel_width - 40.0, 1.0, [0.2, 0.2, 0.25, 1.0]);
        ui.text("PERFORMANCE", 20.0, 60.0, 1.2, accent);

        let perf_lines = [
            format!("FPS:      {:>6.0}", fps),
            format!("Frame:    {:>6.2} ms", frame_ms),
            format!("Verts:    {:>8}", stats.vertices),
            format!("Tris:     {:>8}", stats.triangles),
            format!("Draws:    {:>6}", stats.draw_calls),
        ];
        for (index, line) in perf_lines.iter().enumerate() {
            ui.text(line, 20.0, 80.0 + index as f32 * 18.0, 1.3, white);
        }

        // Camera section
        ui.rect(20.0, 175.0, panel_width - 40.0, 1.0, [0.2, 0.2, 0.25, 1.0]);
        ui.text("CAMERA", 20.0, 180.0, 1.2, accent);

        let cam_lines = [
            format!("Name:  {}", camera_name),
            format!(
                "Pos:   {:>7.1} {:>7.1} {:>7.1}",
                camera_position.x, camera_position.y, camera_position.z
            ),
        ];
        for (index, line) in cam_lines.iter().enumerate() {
            ui.text(line, 20.0, 200.0 + index as f32 * 18.0, 1.3, white);
        }

        // Debug view section
        ui.rect(20.0, 245.0, panel_width - 40.0, 1.0, [0.2, 0.2, 0.25, 1.0]);
        ui.text("DEBUG VIEW", 20.0, 250.0, 1.2, accent);

        for button in &self.buttons {
            let active = button.view == self.view;
            let (bg_color, text_color) = if active {
                ([0.15, 0.4, 0.2, 1.0], [0.6, 1.0, 0.7, 1.0])
            } else {
                ([0.08, 0.1, 0.14, 1.0], [0.8, 0.85, 0.9, 1.0])
            };
            ui.rect(button.x, button.y, button.width, button.height, bg_color);
            if active {
                ui.rect(button.x, button.y, 3.0, button.height, accent);
            }
            ui.text(
                button.label,
                button.x + 8.0,
                button.y + 8.0,
                1.3,
                text_color,
            );
        }

        // Controls section
        ui.rect(20.0, 285.0, panel_width - 40.0, 1.0, [0.2, 0.2, 0.25, 1.0]);
        ui.text("CONTROLS", 20.0, 290.0, 1.2, accent);

        let controls = [
            ("WASD", "Move"),
            ("Space", "Jump / Up (fly)"),
            ("Ctrl", "Down (fly)"),
            ("Shift", "Sprint"),
            ("F", "Toggle Fly"),
            ("Mouse", "Look"),
            ("Tab", "Free Mouse"),
            ("F1 / `", "Console"),
            ("F2", "View: LIT"),
            ("F3", "View: NORMALS"),
            ("F4", "View: DEPTH"),
            ("Esc", "Quit / Close Menu"),
        ];

        let col1_x = 20.0;
        let col2_x = 180.0;
        for (i, (key, desc)) in controls.iter().enumerate() {
            let col = if i < 6 { col1_x } else { col2_x };
            let row = if i < 6 { i } else { i - 6 };
            let y_pos = 310.0 + row as f32 * 16.0;
            ui.text(key, col, y_pos, 1.1, dim);
            ui.text(desc, col + 55.0, y_pos, 1.1, white);
        }

        // VSync status
        ui.rect(20.0, 420.0, panel_width - 40.0, 1.0, [0.2, 0.2, 0.25, 1.0]);
        ui.text("F5: VSync Toggle", 20.0, 425.0, 1.2, accent);
    }

    /// Draws virtual touch controls for mobile platforms.
    pub fn draw_virtual_controls(
        &self,
        ui: &mut UiDrawList,
        virtual_controls: &mut VirtualControls,
        screen_size: Vec3,
    ) {
        let is_mobile = cfg!(target_os = "android");
        if is_mobile {
            virtual_controls.draw(ui, screen_size.truncate());
        }
    }
}
