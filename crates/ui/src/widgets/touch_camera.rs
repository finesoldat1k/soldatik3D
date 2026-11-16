//! Touch camera drag area.
//!
//! On phones the right half of the screen rotates the camera while the left
//! half is used by the joystick. The widget only consumes touch pointers, so
//! desktop mouse-look keeps using the engine's captured cursor.

use glam::Vec2;

use crate::frame::Ui;
use crate::geometry::Rect;
use crate::layout::UiId;
use crate::theme::Theme;

const CAMERA_OWNER: UiId = 0xFFFF_FFF0_0000_0002;

/// Persistent state of the touch camera.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TouchCameraState {
    pub dragging: bool,
    pub pointer: Option<u64>,
    pub last_position: Vec2,
}

impl TouchCameraState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Visual style of the touch camera area.
#[derive(Debug, Clone)]
pub struct TouchCameraStyle {
    pub area: Option<Rect>,
    pub sensitivity: f32,
    /// Allow mouse drags when no touch is present. Off by default so the
    /// captured desktop cursor is not double-applied.
    pub enable_mouse: bool,
}

impl TouchCameraStyle {
    pub fn from_theme(_theme: &Theme) -> Self {
        Self {
            area: None,
            // Radians of rotation per physical pixel. A full-height swipe
            // rotates roughly 120 degrees on a 720 px tall phone.
            sensitivity: 0.003,
            enable_mouse: false,
        }
    }
}

impl Ui {
    /// Tracks a drag in the right half of the screen and returns the camera
    /// delta (already multiplied by `sensitivity`).
    pub fn touch_camera(&mut self, state: &mut TouchCameraState, style: &TouchCameraStyle) -> Vec2 {
        let viewport = self.viewport();
        let area = style.area.unwrap_or_else(|| {
            Rect::new(
                viewport.x + viewport.w * 0.5,
                viewport.y,
                viewport.w * 0.5,
                viewport.h,
            )
        });

        if state.dragging {
            if let Some(pointer) = state.pointer.and_then(|id| self.input.pointer(id).copied())
                && pointer.down
                && self.active_owner(pointer.id) == Some(CAMERA_OWNER)
            {
                let delta = pointer.position - state.last_position;
                state.last_position = pointer.position;
                return delta * style.sensitivity;
            }
            if let Some(id) = state.pointer {
                self.release_pointer(id, CAMERA_OWNER);
            }
            state.reset();
            return Vec2::ZERO;
        }

        let pressed = self
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
            .copied();
        if let Some(pointer) = pressed
            && self.claim_pointer(pointer.id, CAMERA_OWNER)
        {
            state.dragging = true;
            state.pointer = Some(pointer.id);
            state.last_position = pointer.position;
        }
        Vec2::ZERO
    }

    /// True while the camera widget owns a touch.
    pub fn camera_active(&self) -> bool {
        self.owner_active(CAMERA_OWNER)
    }
}
