//! Visual theme shared by every widget.
//!
//! Colours are straight RGBA in linear-ish sRGB space (the backends expect
//! values in `0.0..=1.0`). Sizes are logical pixels.

/// Engine UI theme. Start from [`Theme::dark`] or [`Theme::light`] and tweak
/// individual fields.
#[derive(Debug, Clone)]
pub struct Theme {
    pub text: [f32; 4],
    pub text_muted: [f32; 4],
    pub text_disabled: [f32; 4],
    pub accent: [f32; 4],
    pub accent_hovered: [f32; 4],
    pub accent_pressed: [f32; 4],

    pub panel: [f32; 4],
    pub panel_border: [f32; 4],
    pub window: [f32; 4],
    pub shadow: [f32; 4],

    pub button: [f32; 4],
    pub button_hovered: [f32; 4],
    pub button_pressed: [f32; 4],
    pub button_border: [f32; 4],

    pub track: [f32; 4],
    pub track_fill: [f32; 4],
    pub toggle_off: [f32; 4],
    pub toggle_on: [f32; 4],
    pub knob: [f32; 4],

    pub danger: [f32; 4],
    pub success: [f32; 4],

    /// Default body font size in pixels.
    pub font_size: f32,
    /// Smaller hint text.
    pub small_font_size: f32,
    /// Titles and headers.
    pub title_font_size: f32,
    pub radius: f32,
    /// Minimum height of a touch-friendly control.
    pub control_height: f32,
    /// Minimum touch target for icon buttons.
    pub touch_target: f32,
    pub spacing: f32,
    pub scrollbar_width: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            text: [0.93, 0.94, 0.96, 1.0],
            text_muted: [0.66, 0.69, 0.75, 1.0],
            text_disabled: [0.45, 0.47, 0.52, 1.0],
            accent: [0.22, 0.55, 0.95, 1.0],
            accent_hovered: [0.30, 0.63, 1.0, 1.0],
            accent_pressed: [0.16, 0.45, 0.82, 1.0],

            panel: [0.075, 0.085, 0.105, 0.96],
            panel_border: [0.18, 0.20, 0.25, 1.0],
            window: [0.055, 0.062, 0.078, 0.98],
            shadow: [0.0, 0.0, 0.0, 0.45],

            button: [0.14, 0.16, 0.20, 1.0],
            button_hovered: [0.19, 0.22, 0.28, 1.0],
            button_pressed: [0.10, 0.12, 0.16, 1.0],
            button_border: [0.26, 0.29, 0.36, 1.0],

            track: [0.16, 0.18, 0.23, 1.0],
            track_fill: [0.22, 0.55, 0.95, 1.0],
            toggle_off: [0.20, 0.22, 0.27, 1.0],
            toggle_on: [0.20, 0.62, 0.42, 1.0],
            knob: [0.95, 0.96, 0.98, 1.0],

            danger: [0.88, 0.31, 0.31, 1.0],
            success: [0.30, 0.76, 0.48, 1.0],

            font_size: 16.0,
            small_font_size: 13.0,
            title_font_size: 22.0,
            radius: 8.0,
            control_height: 44.0,
            touch_target: 48.0,
            spacing: 8.0,
            scrollbar_width: 6.0,
        }
    }

    pub fn light() -> Self {
        Self {
            text: [0.09, 0.10, 0.13, 1.0],
            text_muted: [0.36, 0.39, 0.45, 1.0],
            text_disabled: [0.60, 0.62, 0.66, 1.0],
            accent: [0.15, 0.45, 0.90, 1.0],
            accent_hovered: [0.10, 0.38, 0.82, 1.0],
            accent_pressed: [0.05, 0.30, 0.70, 1.0],

            panel: [0.96, 0.965, 0.98, 0.98],
            panel_border: [0.78, 0.80, 0.84, 1.0],
            window: [0.99, 0.99, 1.0, 1.0],
            shadow: [0.0, 0.0, 0.0, 0.18],

            button: [0.90, 0.91, 0.94, 1.0],
            button_hovered: [0.84, 0.87, 0.93, 1.0],
            button_pressed: [0.75, 0.79, 0.88, 1.0],
            button_border: [0.70, 0.73, 0.80, 1.0],

            track: [0.82, 0.84, 0.88, 1.0],
            track_fill: [0.15, 0.45, 0.90, 1.0],
            toggle_off: [0.72, 0.74, 0.78, 1.0],
            toggle_on: [0.22, 0.65, 0.42, 1.0],
            knob: [1.0, 1.0, 1.0, 1.0],

            danger: [0.80, 0.20, 0.20, 1.0],
            success: [0.18, 0.62, 0.35, 1.0],

            font_size: 16.0,
            small_font_size: 13.0,
            title_font_size: 22.0,
            radius: 8.0,
            control_height: 44.0,
            touch_target: 48.0,
            spacing: 8.0,
            scrollbar_width: 6.0,
        }
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self.small_font_size = (size * 0.82).max(9.0);
        self.title_font_size = (size * 1.35).max(12.0);
        self
    }

    /// Multiplies every control metric by `scale`. Useful when the OS reports
    /// a non-default DPI.
    pub fn scaled(mut self, scale: f32) -> Self {
        let scale = scale.clamp(0.5, 3.0);
        self.font_size *= scale;
        self.small_font_size *= scale;
        self.title_font_size *= scale;
        self.radius *= scale;
        self.control_height *= scale;
        self.touch_target *= scale;
        self.spacing *= scale;
        self.scrollbar_width *= scale;
        self
    }
}
