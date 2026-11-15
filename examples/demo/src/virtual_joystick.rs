//! Virtual joystick and touch controls for Android.

use runtime::glam::Vec2;
use runtime::{Input, UiDrawList};

const JOYSTICK_RADIUS: f32 = 80.0;
const JOYSTICK_DEADZONE: f32 = 0.15;
const JOYSTICK_MARGIN: f32 = 40.0;
const BUTTON_SIZE: f32 = 70.0;
const BUTTON_MARGIN: f32 = 40.0;

/// Virtual joystick for movement control.
#[derive(Debug, Clone, Copy)]
struct VirtualJoystick {
    center: Vec2,
    current: Vec2,
    active: bool,
    touch_id: Option<u64>,
}

impl Default for VirtualJoystick {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            current: Vec2::ZERO,
            active: false,
            touch_id: None,
        }
    }
}

impl VirtualJoystick {
    fn update(&mut self, input: &Input, screen_size: Vec2) {
        // Calculate joystick position (bottom-left area)
        self.center = Vec2::new(
            JOYSTICK_RADIUS + JOYSTICK_MARGIN,
            screen_size.y - JOYSTICK_RADIUS - JOYSTICK_MARGIN,
        );

        if self.active {
            if let Some(touch_id) = self.touch_id
                && let Some(touch) = input.touch(touch_id)
            {
                let delta = touch.position - self.center;
                let distance = delta.length();

                if distance > JOYSTICK_RADIUS {
                    self.current = delta.normalize() * JOYSTICK_RADIUS;
                } else {
                    self.current = delta;
                }
                return;
            }
            // Touch ended or not found
            self.active = false;
            self.touch_id = None;
            self.current = Vec2::ZERO;
        } else {
            // Check for new touch in joystick area
            for touch in input.touches() {
                if touch.started {
                    let delta = touch.position - self.center;
                    if delta.length() <= JOYSTICK_RADIUS {
                        self.active = true;
                        self.touch_id = Some(touch.id);
                        self.current = delta;
                        break;
                    }
                }
            }
        }
    }

    fn get_movement(&self) -> Vec2 {
        if !self.active {
            return Vec2::ZERO;
        }
        let distance = self.current.length();
        if distance < JOYSTICK_RADIUS * JOYSTICK_DEADZONE {
            return Vec2::ZERO;
        }
        self.current / JOYSTICK_RADIUS
    }

    fn draw(&self, ui: &mut UiDrawList) {
        let outer_color = if self.active {
            [0.3, 0.5, 0.8, 0.7]
        } else {
            [0.2, 0.3, 0.5, 0.5]
        };
        let inner_color = if self.active {
            [0.4, 0.7, 1.0, 0.9]
        } else {
            [0.3, 0.4, 0.6, 0.6]
        };

        // Draw outer ring as multiple thin rects to approximate a circle
        let segments = 16;
        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let next_angle = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
            let x1 = self.center.x + angle.cos() * JOYSTICK_RADIUS;
            let y1 = self.center.y + angle.sin() * JOYSTICK_RADIUS;
            let x2 = self.center.x + next_angle.cos() * JOYSTICK_RADIUS;
            let y2 = self.center.y + next_angle.sin() * JOYSTICK_RADIUS;

            // Draw a quad for this segment (approximation)
            let thickness = 4.0;
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.0 {
                let nx = -dy / len * thickness * 0.5;
                let ny = dx / len * thickness * 0.5;
                ui.rect(
                    (x1 + x2) * 0.5 - nx - dx * 0.5,
                    (y1 + y2) * 0.5 - ny - dy * 0.5,
                    len,
                    thickness,
                    [
                        outer_color[0],
                        outer_color[1],
                        outer_color[2],
                        outer_color[3] * 0.5,
                    ],
                );
            }
        }

        // Draw inner handle
        if self.active {
            let handle_radius = JOYSTICK_RADIUS * 0.5;
            let handle_x = self.center.x + self.current.x - handle_radius;
            let handle_y = self.center.y + self.current.y - handle_radius;
            ui.rect(
                handle_x,
                handle_y,
                handle_radius * 2.0,
                handle_radius * 2.0,
                inner_color,
            );

            // Handle border
            ui.rect(
                handle_x,
                handle_y,
                handle_radius * 2.0,
                2.0,
                [0.6, 0.9, 1.0, 1.0],
            );
            ui.rect(
                handle_x,
                handle_y + handle_radius * 2.0 - 2.0,
                handle_radius * 2.0,
                2.0,
                [0.2, 0.4, 0.6, 1.0],
            );
            ui.rect(
                handle_x,
                handle_y,
                2.0,
                handle_radius * 2.0,
                [0.6, 0.9, 1.0, 1.0],
            );
            ui.rect(
                handle_x + handle_radius * 2.0 - 2.0,
                handle_y,
                2.0,
                handle_radius * 2.0,
                [0.2, 0.4, 0.6, 1.0],
            );
        } else {
            // Draw center dot when inactive
            let dot_size = 8.0;
            ui.rect(
                self.center.x - dot_size * 0.5,
                self.center.y - dot_size * 0.5,
                dot_size,
                dot_size,
                [0.4, 0.6, 0.8, 0.5],
            );
        }
    }
}

/// Virtual buttons for jump, fly toggle, etc.
#[derive(Debug, Clone, Copy)]
struct VirtualButton {
    center: Vec2,
    size: f32,
    pressed: bool,
    touch_id: Option<u64>,
    label: &'static str,
}

impl VirtualButton {
    fn new(label: &'static str) -> Self {
        Self {
            center: Vec2::ZERO,
            size: BUTTON_SIZE,
            pressed: false,
            touch_id: None,
            label,
        }
    }

    fn contains(&self, point: Vec2) -> bool {
        let half = self.size * 0.5;
        point.x >= self.center.x - half
            && point.x <= self.center.x + half
            && point.y >= self.center.y - half
            && point.y <= self.center.y + half
    }

    fn update(&mut self, input: &Input) {
        if self.pressed {
            if let Some(touch_id) = self.touch_id
                && let Some(touch) = input.touch(touch_id)
            {
                self.pressed = self.contains(touch.position);
                return;
            }
            self.pressed = false;
            self.touch_id = None;
        } else {
            for touch in input.touches() {
                if touch.started && self.contains(touch.position) {
                    self.pressed = true;
                    self.touch_id = Some(touch.id);
                    break;
                }
            }
        }
    }

    fn is_pressed(&self) -> bool {
        self.pressed
    }

    fn draw(&mut self, ui: &mut UiDrawList, screen_size: Vec2, index: usize) {
        // Position buttons on the right side
        let spacing = BUTTON_SIZE + 15.0;
        let start_x = screen_size.x - BUTTON_MARGIN - BUTTON_SIZE;
        let start_y = screen_size.y - BUTTON_MARGIN - BUTTON_SIZE;
        self.center = Vec2::new(start_x, start_y - index as f32 * spacing);

        let color = if self.pressed {
            [0.4, 0.8, 0.4, 0.9]
        } else {
            [0.3, 0.5, 0.3, 0.7]
        };

        // Button background
        let half = self.size * 0.5;
        ui.rect(
            self.center.x - half,
            self.center.y - half,
            self.size,
            self.size,
            color,
        );

        // Button border
        ui.rect(
            self.center.x - half,
            self.center.y - half,
            self.size,
            2.0,
            [0.6, 0.9, 0.6, 1.0],
        );
        ui.rect(
            self.center.x - half,
            self.center.y + half - 2.0,
            self.size,
            2.0,
            [0.2, 0.4, 0.2, 1.0],
        );
        ui.rect(
            self.center.x - half,
            self.center.y - half,
            2.0,
            self.size,
            [0.6, 0.9, 0.6, 1.0],
        );
        ui.rect(
            self.center.x + half - 2.0,
            self.center.y - half,
            2.0,
            self.size,
            [0.2, 0.4, 0.2, 1.0],
        );

        // Label
        let text_scale = 1.5;
        let text_width = UiDrawList::text_width(self.label, text_scale);
        ui.text(
            self.label,
            self.center.x - text_width * 0.5,
            self.center.y - 10.0,
            text_scale,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
}

/// Complete virtual touch control system.
pub struct VirtualControls {
    joystick: VirtualJoystick,
    jump_button: VirtualButton,
    fly_button: VirtualButton,
}

impl Default for VirtualControls {
    fn default() -> Self {
        Self {
            joystick: VirtualJoystick::default(),
            jump_button: VirtualButton::new("JUMP"),
            fly_button: VirtualButton::new("FLY"),
        }
    }
}

impl VirtualControls {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the virtual controls based on touch input.
    pub fn update(&mut self, input: &Input, screen_size: Vec2) {
        self.joystick.update(input, screen_size);
        self.jump_button.update(input);
        self.fly_button.update(input);
    }

    /// Returns the movement vector from the virtual joystick (-1 to 1).
    pub fn movement(&self) -> Vec2 {
        self.joystick.get_movement()
    }

    /// Returns true if the virtual jump button was pressed this frame.
    pub fn jump_pressed(&self) -> bool {
        self.jump_button.is_pressed()
    }

    /// Returns true if the virtual fly toggle button was pressed this frame.
    pub fn fly_pressed(&self) -> bool {
        self.fly_button.is_pressed()
    }

    /// Draws the virtual controls.
    pub fn draw(&mut self, ui: &mut UiDrawList, screen_size: Vec2) {
        if screen_size.x > 0.0 && screen_size.y > 0.0 {
            self.joystick.draw(ui);
            self.jump_button.draw(ui, screen_size, 0);
            self.fly_button.draw(ui, screen_size, 1);
        }
    }
}
