//! Debug overlay and render-view selector for the test polygon demo.
//!
//! F2 toggles the overlay, F3/F4/F5 switch the world shader between lit,
//! normal and depth output. The panel is drawn in physical pixels and scales
//! down on small windows so it never covers the whole viewport.

use runtime::glam::Vec3;
use runtime::ui::Ui;
use runtime::winit::keyboard::KeyCode;
use runtime::{DebugView, Input, RenderStats};

pub struct DebugMenu {
    pub open: bool,
    pub view: DebugView,
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
        Self { open: false, view }
    }

    pub fn handle_shortcuts(&mut self, input: &Input) {
        if input.key_pressed(KeyCode::F2) {
            self.open = !self.open;
        }
        if input.key_pressed(KeyCode::F3) {
            self.view = DebugView::Lit;
        }
        if input.key_pressed(KeyCode::F4) {
            self.view = DebugView::Normals;
        }
        if input.key_pressed(KeyCode::F5) {
            self.view = DebugView::Depth;
        }
    }

    /// Draws the crosshair when the panel is hidden, or the full diagnostics
    /// panel when it is open. `console_open` suppresses both so the console has
    /// an unobstructed command line.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        ui: &mut Ui,
        scale: f32,
        viewport: (f32, f32),
        stats: RenderStats,
        camera_position: Vec3,
        camera_name: &str,
        player_position: Vec3,
        flying: bool,
        area: &str,
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

        let (width, height) = viewport;
        // Fit the panel on short landscape screens (Android) without
        // shrinking desktop text: cap by both viewport axes.
        // Scale up by 1.5x for larger debug menu.
        let s = scale.min(height / 520.0).min(width / 400.0).max(1.0) * 1.5;
        let panel_w = (380.0 * s).min(width - 24.0 * s);
        let panel_h = (420.0 * s).min(height - 24.0 * s);
        let x = 12.0 * s;
        let y = 12.0 * s;

        let panel = [0.020, 0.024, 0.032, 0.94];
        let line = [0.22, 0.24, 0.28, 0.9];
        let white = [0.92, 0.94, 0.97, 1.0];
        let dim = [0.58, 0.63, 0.70, 1.0];
        let accent = [1.0, 0.62, 0.16, 1.0];
        let good = [0.55, 1.0, 0.66, 1.0];

        ui.rect(x, y, panel_w, panel_h, panel);
        ui.rect(x, y, panel_w, 2.0 * s, accent);
        ui.rect_outline(
            runtime::ui::Rect::new(x, y, panel_w, panel_h),
            (1.0 * s).max(1.0),
            [0.28, 0.30, 0.35, 0.85],
        );

        let pad = 12.0 * s;
        let inner_w = panel_w - pad * 2.0;
        let mut cursor = y + 12.0 * s;

        ui.text("SOLDATIK3D // DEBUG", x + pad, cursor, 14.0 * s, accent);
        cursor += 20.0 * s;
        ui.text(
            &format!("CAM {camera_name}"),
            x + pad,
            cursor,
            10.0 * s,
            dim,
        );
        cursor += 16.0 * s;
        ui.rect(x + pad, cursor, inner_w, 1.0 * s, line);
        cursor += 10.0 * s;

        ui.text("PERFORMANCE", x + pad, cursor, 11.0 * s, accent);
        cursor += 16.0 * s;
        let performance = [
            format!("FPS        {:>5.0}", fps),
            format!("Frame      {:>5.2} ms", frame_ms),
            format!("Vertices   {:>7}", stats.vertices),
            format!("Triangles  {:>7}", stats.triangles),
            format!("Draws      {:>5}", stats.draw_calls),
        ];
        for text in &performance {
            ui.text(text, x + pad, cursor, 10.0 * s, white);
            cursor += 15.0 * s;
        }

        cursor += 3.0 * s;
        ui.rect(x + pad, cursor, inner_w, 1.0 * s, line);
        cursor += 10.0 * s;
        ui.text("CAMERA", x + pad, cursor, 11.0 * s, accent);
        cursor += 16.0 * s;
        for text in [
            format!("Name       {camera_name}"),
            format!(
                "Position   {:>7.1} {:>7.1} {:>7.1}",
                camera_position.x, camera_position.y, camera_position.z
            ),
        ] {
            ui.text(&text, x + pad, cursor, 10.0 * s, white);
            cursor += 15.0 * s;
        }

        cursor += 3.0 * s;
        ui.rect(x + pad, cursor, inner_w, 1.0 * s, line);
        cursor += 10.0 * s;
        ui.text("PLAYER", x + pad, cursor, 11.0 * s, accent);
        cursor += 16.0 * s;
        let mode = if flying { "FLIGHT" } else { "WALK" };
        for text in [
            format!(
                "Position   {:>7.1} {:>7.1} {:>7.1}",
                player_position.x, player_position.y, player_position.z
            ),
            format!("Area       {area}"),
            format!("Mode       {mode}"),
        ] {
            ui.text(&text, x + pad, cursor, 10.0 * s, white);
            cursor += 15.0 * s;
        }

        cursor += 3.0 * s;
        ui.rect(x + pad, cursor, inner_w, 1.0 * s, line);
        cursor += 10.0 * s;
        ui.text("DEBUG VIEW", x + pad, cursor, 11.0 * s, accent);
        cursor += 16.0 * s;
        let gap = 6.0 * s;
        let chip_w = (inner_w - gap * 2.0) / 3.0;
        let chip_h = 24.0 * s;
        for (index, (label, view)) in [
            ("LIT", DebugView::Lit),
            ("NORMALS", DebugView::Normals),
            ("DEPTH", DebugView::Depth),
        ]
        .into_iter()
        .enumerate()
        {
            let chip_x = x + pad + index as f32 * (chip_w + gap);
            let active = self.view == view;
            ui.rect_rounded(
                chip_x,
                cursor,
                chip_w,
                chip_h,
                3.0 * s,
                if active {
                    [0.20, 0.42, 0.20, 1.0]
                } else {
                    [0.08, 0.10, 0.14, 1.0]
                },
            );
            if active {
                ui.rect(chip_x, cursor, 2.5 * s, chip_h, good);
            }
            let font = 8.0 * s;
            let measured = ui.measure_text(label, font);
            ui.text(
                label,
                chip_x + (chip_w - measured.x) * 0.5,
                cursor + (chip_h - measured.y) * 0.5,
                font,
                if active { good } else { white },
            );
        }
        cursor += chip_h + 6.0 * s;

        cursor += 3.0 * s;
        ui.rect(x + pad, cursor, inner_w, 1.0 * s, line);
        cursor += 10.0 * s;
        ui.text("CONTROLS", x + pad, cursor, 11.0 * s, accent);
        cursor += 16.0 * s;
        let col1 = x + pad;
        let col2 = x + pad + inner_w * 0.5;
        let controls = [
            ("WASD", "Move"),
            ("SHIFT", "Sprint"),
            ("CTRL", "Crouch / down"),
            ("SPACE", "Jump / ascend"),
            ("F", "Toggle flight"),
            ("TAB", "Release cursor"),
            ("F1 / `", "Console"),
            ("F2", "Toggle debug"),
            ("F3", "View: Lit"),
            ("F4", "View: Normals"),
            ("F5", "View: Depth"),
        ];
        for (index, (key, description)) in controls.iter().enumerate() {
            let column = if index < 6 { col1 } else { col2 };
            let row = if index < 6 { index } else { index - 6 };
            let line_y = cursor + row as f32 * 15.0 * s;
            ui.text(key, column, line_y, 9.5 * s, dim);
            ui.text(description, column + 65.0 * s, line_y, 9.5 * s, white);
        }
    }
}
