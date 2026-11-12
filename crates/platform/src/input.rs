use std::collections::HashSet;

use engine_core::glam::Vec2;
use winit::keyboard::KeyCode;

/// Aggregated input state for one frame.
///
/// Held keys are persistent, while [`Input::key_pressed`], [`Input::mouse_delta`],
/// [`Input::scroll_delta`] and [`Input::primary_clicked`] only describe the
/// current frame and are reset by the engine after the game has updated.
#[derive(Default)]
pub struct Input {
    keys: HashSet<KeyCode>,
    pressed: HashSet<KeyCode>,
    mouse_delta: Vec2,
    scroll_delta: f32,
    cursor_position: Vec2,
    primary_clicked: bool,
    cursor_captured: bool,
    suppressed_motion_frames: u8,
}

impl Input {
    pub fn key_down(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    /// Raw mouse motion accumulated during this frame. Only non-zero while the
    /// cursor is captured.
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub fn scroll_delta(&self) -> f32 {
        self.scroll_delta
    }

    /// Cursor position in physical pixels relative to the top-left corner.
    pub fn cursor_position(&self) -> Vec2 {
        self.cursor_position
    }

    pub fn primary_clicked(&self) -> bool {
        self.primary_clicked
    }

    pub fn cursor_captured(&self) -> bool {
        self.cursor_captured
    }

    pub fn set_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            if self.keys.insert(key) {
                self.pressed.insert(key);
            }
        } else {
            self.keys.remove(&key);
        }
    }

    pub fn add_mouse_delta(&mut self, delta: Vec2) {
        if self.cursor_captured && self.suppressed_motion_frames == 0 {
            self.mouse_delta += delta;
        }
    }

    pub fn add_scroll_delta(&mut self, delta: f32) {
        self.scroll_delta += delta;
    }

    pub fn set_cursor_position(&mut self, position: Vec2) {
        self.cursor_position = position;
    }

    pub fn set_primary_clicked(&mut self, clicked: bool) {
        self.primary_clicked = clicked;
    }

    pub fn set_cursor_captured(&mut self, captured: bool) {
        self.cursor_captured = captured;
        if captured {
            // The compositor can report a large synthetic delta when the
            // pointer is warped into the locked position. Ignore motion for a
            // couple of frames so the camera does not snap on startup.
            self.suppressed_motion_frames = 2;
            self.mouse_delta = Vec2::ZERO;
        }
    }

    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = 0.0;
        self.primary_clicked = false;
        self.suppressed_motion_frames = self.suppressed_motion_frames.saturating_sub(1);
    }
}
