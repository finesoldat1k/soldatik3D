//! Dynamic virtual joystick for touch controls.
//!
//! The joystick lives in the left half of the screen by default. Its base
//! follows the finger that started the gesture, which is much more comfortable
//! on phones than a fixed stick.

use glam::Vec2;

use crate::frame::Ui;
use crate::geometry::Rect;
use crate::layout::UiId;
use crate::theme::Theme;

const JOYSTICK_OWNER: UiId = 0xFFFF_FFF0_0000_0001;

/// Persistent state of a virtual joystick. Store it in the game struct.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoystickState {
    /// Normalized output in `-1..=1`, deadzone already applied.
    pub value: Vec2,
    pub active: bool,
    pub pointer: Option<u64>,
    pub base: Vec2,
    pub handle: Vec2,
}

impl Default for JoystickState {
    fn default() -> Self {
        Self {
            value: Vec2::ZERO,
            active: false,
            pointer: None,
            base: Vec2::ZERO,
            handle: Vec2::ZERO,
        }
    }
}

impl JoystickState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Movement as a 3D-style pair `(strafe, forward)` where forward is
    /// positive when the finger is above the base.
    pub fn axes(&self) -> Vec2 {
        Vec2::new(self.value.x, -self.value.y)
    }
}

/// Visual style of the joystick.
#[derive(Debug, Clone)]
pub struct JoystickStyle {
    pub radius: f32,
    pub handle_radius: f32,
    pub deadzone: f32,
    /// Touch region in screen pixels. `None` means the left half of the screen.
    pub area: Option<Rect>,
    pub base_color: [f32; 4],
    pub active_base_color: [f32; 4],
    pub border_color: [f32; 4],
    pub handle_color: [f32; 4],
    pub show_hint: bool,
    pub enable_mouse: bool,
}

impl JoystickStyle {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            radius: 78.0,
            handle_radius: 34.0,
            deadzone: 0.15,
            area: None,
            base_color: [theme.text[0], theme.text[1], theme.text[2], 0.08],
            active_base_color: [theme.accent[0], theme.accent[1], theme.accent[2], 0.18],
            border_color: [theme.text[0], theme.text[1], theme.text[2], 0.30],
            handle_color: [theme.text[0], theme.text[1], theme.text[2], 0.55],
            show_hint: true,
            enable_mouse: true,
        }
    }

    /// Scales the visual sizes for a high-DPI display.
    pub fn scaled(mut self, scale: f32) -> Self {
        let scale = scale.clamp(0.5, 3.0);
        self.radius *= scale;
        self.handle_radius *= scale;
        self
    }
}

impl Ui {
    /// Updates and draws a virtual joystick. Returns the normalized output.
    pub fn virtual_joystick(&mut self, state: &mut JoystickState, style: &JoystickStyle) -> Vec2 {
        let viewport = self.viewport();
        let area = style
            .area
            .unwrap_or_else(|| Rect::new(viewport.x, viewport.y, viewport.w * 0.5, viewport.h));

        if state.active {
            let pointer = state.pointer.and_then(|id| self.input.pointer(id).copied());
            match pointer {
                Some(pointer)
                    if pointer.down && self.active_owner(pointer.id) == Some(JOYSTICK_OWNER) =>
                {
                    let delta = pointer.position - state.base;
                    let distance = delta.length();
                    let clamped = if distance > style.radius {
                        delta * (style.radius / distance)
                    } else {
                        delta
                    };
                    state.handle = state.base + clamped;
                    let magnitude = (distance / style.radius).min(1.0);
                    state.value = if magnitude <= style.deadzone {
                        Vec2::ZERO
                    } else {
                        let scaled = (magnitude - style.deadzone) / (1.0 - style.deadzone);
                        delta.normalize_or_zero() * scaled
                    };
                }
                _ => {
                    if let Some(id) = state.pointer {
                        self.release_pointer(id, JOYSTICK_OWNER);
                    }
                    state.reset();
                }
            }
        } else if let Some(pointer) = self
            .input
            .pointers
            .iter()
            .find(|pointer| {
                pointer.pressed
                    && !self.pointer_claimed(pointer.id)
                    && area.contains(pointer.position)
                    && (pointer.touch() || style.enable_mouse)
                    && !self.position_blocked(pointer.position)
            })
            .copied()
            && self.claim_pointer(pointer.id, JOYSTICK_OWNER)
        {
            state.active = true;
            state.pointer = Some(pointer.id);
            state.base = pointer.position;
            state.handle = pointer.position;
            state.value = Vec2::ZERO;
        }

        if state.active {
            self.draw
                .circle(state.base, style.radius, style.active_base_color);
            self.draw
                .circle_outline(state.base, style.radius, 2.0, style.border_color);
            self.draw
                .circle(state.handle, style.handle_radius, style.handle_color);
            self.draw
                .circle_outline(state.handle, style.handle_radius, 1.5, style.border_color);
        } else if style.show_hint {
            let center = Vec2::new(
                viewport.x + style.radius + 28.0,
                viewport.bottom() - style.radius - 28.0,
            );
            let hint = [
                style.base_color[0],
                style.base_color[1],
                style.base_color[2],
                0.05,
            ];
            self.draw.circle(center, style.radius, hint);
            self.draw.circle_outline(
                center,
                style.radius,
                1.0,
                [
                    style.border_color[0],
                    style.border_color[1],
                    style.border_color[2],
                    0.12,
                ],
            );
        }

        state.value
    }
}
