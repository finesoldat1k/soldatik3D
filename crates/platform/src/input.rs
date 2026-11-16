use std::collections::{HashMap, HashSet};

use glam::Vec2;
use winit::keyboard::KeyCode;

/// Maps logical action names to key codes.
///
/// Games define their actions and default bindings here. The engine provides
/// methods to query actions instead of raw key codes, enabling rebindable controls.
#[derive(Default, Clone)]
pub struct InputMap {
    bindings: HashMap<String, Vec<KeyCode>>,
}

impl InputMap {
    /// Creates an empty input map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds an action to one or more key codes.
    pub fn bind(mut self, action: &str, keys: &[KeyCode]) -> Self {
        self.bindings.insert(action.to_owned(), keys.to_vec());
        self
    }

    /// Returns all keys bound to an action.
    pub fn keys_for(&self, action: &str) -> Option<&[KeyCode]> {
        self.bindings.get(action).map(|v| v.as_slice())
    }
}

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
    primary_down: bool,
    primary_pressed: bool,
    primary_released: bool,
    cursor_captured: bool,
    /// Display density reported by the window, in physical pixels per logical
    /// pixel. `None` until a window exists, in which case [`Input::scale_factor`]
    /// reports a neutral `1.0`.
    scale_factor: Option<f32>,
    /// Set once the platform reports a touch; stays true for the session.
    touch_mode: bool,
    suppressed_motion_frames: u8,
    text: String,
    /// Active action bindings for this frame.
    action_map: InputMap,
    /// Touch input state for mobile platforms.
    touches: Vec<TouchState>,
    /// Previous frame touches for gesture detection.
    prev_touches: Vec<TouchState>,
}

/// Represents a single touch point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchState {
    /// Unique identifier for this touch point (persists across frames).
    pub id: u64,
    /// Current position in physical pixels.
    pub position: Vec2,
    /// Previous frame position in physical pixels.
    pub prev_position: Vec2,
    /// Whether this touch started this frame.
    pub started: bool,
    /// Whether this touch ended this frame.
    pub ended: bool,
}

impl TouchState {
    /// Returns the delta movement since the previous frame.
    pub fn delta(&self) -> Vec2 {
        self.position - self.prev_position
    }
}

impl Input {
    pub fn key_down(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    /// Returns true if any key bound to `action` is currently held down.
    pub fn action_down(&self, action: &str) -> bool {
        self.action_map
            .keys_for(action)
            .is_some_and(|keys| keys.iter().any(|key| self.keys.contains(key)))
    }

    /// Returns true if any key bound to `action` was pressed this frame.
    pub fn action_pressed(&self, action: &str) -> bool {
        self.action_map
            .keys_for(action)
            .is_some_and(|keys| keys.iter().any(|key| self.pressed.contains(key)))
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

    /// Whether the left mouse button was pressed this frame.
    pub fn primary_clicked(&self) -> bool {
        self.primary_pressed
    }

    /// Whether the left mouse button is currently held.
    pub fn primary_down(&self) -> bool {
        self.primary_down
    }

    /// Whether the left mouse button was released this frame.
    pub fn primary_released(&self) -> bool {
        self.primary_released
    }

    /// Whether any touch has been seen during this session.
    pub fn touch_mode(&self) -> bool {
        self.touch_mode
    }

    pub fn cursor_captured(&self) -> bool {
        self.cursor_captured
    }

    /// Physical pixels per logical pixel reported by the window.
    ///
    /// Mobile displays commonly report 2–3, desktop HiDPI screens 1.5–2.
    /// GUI code should multiply its logical metrics by this value because the
    /// UI works in physical pixels.
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor.unwrap_or(1.0)
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        if scale_factor.is_finite() && scale_factor > 0.0 {
            self.scale_factor = Some(scale_factor);
        }
    }

    /// Text typed during this frame, as reported by the platform. Used by the
    /// engine console; gameplay code can ignore it.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn push_text(&mut self, text: &str) {
        self.text.push_str(text);
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

    /// Updates the left mouse button. Press and release edges are exposed for
    /// one frame, so widgets can implement click-on-release semantics.
    pub fn set_primary_down(&mut self, down: bool) {
        if down && !self.primary_down {
            self.primary_pressed = true;
        } else if !down && self.primary_down {
            self.primary_released = true;
        }
        self.primary_down = down;
    }

    /// Compatibility shim for callers that only report click state.
    pub fn set_primary_clicked(&mut self, clicked: bool) {
        self.set_primary_down(clicked);
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

    pub fn set_action_map(&mut self, map: InputMap) {
        self.action_map = map;
    }

    /// Returns all active touches for the current frame.
    pub fn touches(&self) -> &[TouchState] {
        &self.touches
    }

    /// Returns the touch with the given ID if it's currently active.
    pub fn touch(&self, id: u64) -> Option<&TouchState> {
        self.touches.iter().find(|t| t.id == id)
    }

    /// Returns the first active touch, if any.
    pub fn first_touch(&self) -> Option<&TouchState> {
        self.touches.first()
    }

    /// Adds a new touch point.
    pub fn add_touch(&mut self, id: u64, position: Vec2) {
        self.touch_mode = true;
        let touch = TouchState {
            id,
            position,
            prev_position: position,
            started: true,
            ended: false,
        };
        self.touches.push(touch);
    }

    /// Updates an existing touch point.
    pub fn update_touch(&mut self, id: u64, position: Vec2) {
        if let Some(touch) = self.touches.iter_mut().find(|t| t.id == id) {
            touch.prev_position = touch.position;
            touch.position = position;
            touch.started = false;
            touch.ended = false;
        }
    }

    /// Marks a touch as ended (will be removed at the end of the frame).
    pub fn end_touch(&mut self, id: u64) {
        if let Some(touch) = self.touches.iter_mut().find(|t| t.id == id) {
            touch.ended = true;
        }
    }

    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = 0.0;
        self.primary_pressed = false;
        self.primary_released = false;
        self.text.clear();
        self.suppressed_motion_frames = self.suppressed_motion_frames.saturating_sub(1);

        // Update touch state: move current to prev, remove ended touches.
        self.prev_touches = self.touches.clone();
        self.touches.retain_mut(|touch| {
            if touch.ended {
                false
            } else {
                touch.prev_position = touch.position;
                touch.started = false;
                true
            }
        });
    }
}

/// Trait implementation for UI widget system to access touch/mouse input.
impl ui::TouchInputProvider for Input {
    type Touch = TouchState;

    fn first_touch(&self) -> Option<&Self::Touch> {
        self.first_touch()
    }

    fn touch(&self, id: u64) -> Option<&Self::Touch> {
        self.touch(id)
    }

    fn touches(&self) -> &[Self::Touch] {
        self.touches()
    }

    fn primary_clicked(&self) -> bool {
        self.primary_clicked()
    }

    fn primary_down(&self) -> bool {
        self.primary_down()
    }

    fn primary_released(&self) -> bool {
        self.primary_released()
    }

    fn scroll_delta(&self) -> f32 {
        self.scroll_delta()
    }

    fn touch_mode(&self) -> bool {
        self.touch_mode()
    }

    fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta()
    }

    fn cursor_position(&self) -> Vec2 {
        self.cursor_position()
    }
}

impl ui::TouchStateProvider for TouchState {
    fn position(&self) -> Vec2 {
        self.position
    }

    fn prev_position(&self) -> Vec2 {
        self.prev_position
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn started(&self) -> bool {
        self.started
    }

    fn ended(&self) -> bool {
        self.ended
    }
}
