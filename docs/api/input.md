# Input API Reference

Complete API documentation for input handling across all platforms.

## Input Struct

```rust
pub struct Input {
    // Keyboard state
    keys: HashMap<KeyCode, KeyState>,
    
    // Mouse state
    mouse_position: Vec2,
    mouse_delta: Vec2,
    mouse_buttons: HashMap<MouseButton, ButtonState>,
    scroll_delta: Vec2,
    
    // Gamepad state
    gamepads: Vec<GamepadState>,
    
    // Touch state (Android)
    touches: Vec<TouchState>,
}
```

## Keyboard

### Key State Queries

```rust
impl Input {
    /// True while key is held down
    pub fn is_key_pressed(&self, key: KeyCode) -> bool;
    
    /// True only on the frame key was pressed
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool;
    
    /// True only on the frame key was released
    pub fn is_key_just_released(&self, key: KeyCode) -> bool;
    
    /// Get key state enum
    pub fn key_state(&self, key: KeyCode) -> KeyState;
}

pub enum KeyState {
    Released,
    Pressed,
    JustPressed,
    JustReleased,
}
```

### Usage Patterns

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let input = ctx.input;
    
    // Continuous movement (held)
    const SPEED: f32 = 5.0;
    if input.is_key_pressed(KeyCode::W) { pos.z -= SPEED * ctx.dt; }
    if input.is_key_pressed(KeyCode::S) { pos.z += SPEED * ctx.dt; }
    if input.is_key_pressed(KeyCode::A) { pos.x -= SPEED * ctx.dt; }
    if input.is_key_pressed(KeyCode::D) { pos.x += SPEED * ctx.dt; }
    
    // One-time actions (just pressed)
    if input.is_key_just_pressed(KeyCode::Space) { jump(); }
    if input.is_key_just_pressed(KeyCode::E) { interact(); }
    if input.is_key_just_pressed(KeyCode::Escape) { ctx.request_exit(); }
    
    // Toggle states
    if input.is_key_just_pressed(KeyCode::F1) {
        ctx.renderer.set_debug_view(DebugView::Wireframe);
    }
}
```

### KeyCode Enum

```rust
pub enum KeyCode {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers (top row)
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    
    // Numpad
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide,
    NumpadEnter, NumpadDecimal, NumpadEqual,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
    
    // Modifiers
    LeftShift, RightShift,
    LeftControl, RightControl,
    LeftAlt, RightAlt,
    LeftMeta, RightMeta,  // Win/Command
    
    // Special
    Escape, Space, Enter, Tab, Backspace, Delete,
    Insert, Home, End, PageUp, PageDown,
    Left, Right, Up, Down,
    CapsLock, NumLock, ScrollLock, PrintScreen, Pause,
    
    // Punctuation (US layout)
    Backquote,        // `
    Minus, Equal,     // - =
    LeftBracket, RightBracket,  // [ ]
    Backslash,        // \
    Semicolon, Quote, // ; '
    Comma, Period, Slash,       // , . /
    
    // Other
    Unknown,
}
```

---

## Mouse

### Position & Movement

```rust
impl Input {
    /// Current cursor position in window coordinates (pixels)
    /// Origin: top-left (0, 0)
    pub fn mouse_position(&self) -> Vec2;
    
    /// Mouse movement since last frame (pixels)
    /// Positive X = right, Positive Y = down
    pub fn mouse_delta(&self) -> Vec2;
    
    /// Scroll wheel delta since last frame
    /// Positive Y = scroll up
    pub fn scroll_delta(&self) -> Vec2;
}
```

### Buttons

```rust
impl Input {
    /// True while button is held
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool;
    
    /// True only on frame button was pressed
    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool;
    
    /// True only on frame button was released
    pub fn is_mouse_button_just_released(&self, button: MouseButton) -> bool;
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}
```

### Usage

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let input = ctx.input;
    
    // Camera rotation (FPS style)
    let delta = input.mouse_delta();
    yaw -= delta.x * 0.002;
    pitch = (pitch - delta.y * 0.002).clamp(-1.5, 1.5);
    
    // Click interaction
    if input.is_mouse_button_just_pressed(MouseButton::Left) {
        let pos = input.mouse_position();
        // Raycast from camera through pos
        if let Some(hit) = raycast(pos) {
            select_object(hit.entity);
        }
    }
    
    // Right-click context
    if input.is_mouse_button_just_pressed(MouseButton::Right) {
        open_context_menu(input.mouse_position());
    }
    
    // Scroll zoom
    let scroll = input.scroll_delta().y;
    camera.distance = (camera.distance - scroll * 0.5).clamp(1.0, 50.0);
}
```

### Cursor Control

```rust
// Via WindowHints in GameConfig
GameConfig::default()
    .with_window_hints(WindowHints {
        cursor_visible: false,    // Hide cursor
        cursor_locked: true,      // Lock to center (FPS)
        cursor_grabbed: true,     // Confine to window
        ..Default::default()
    })
```

---

## Gamepad

```rust
impl Input {
    /// Get gamepad by index (0-3)
    pub fn gamepad(&self, id: usize) -> Option<&GamepadState>;
    
    /// All connected gamepads
    pub fn gamepads(&self) -> &[GamepadState];
    
    /// Number of connected gamepads
    pub fn gamepad_count(&self) -> usize;
}

pub struct GamepadState {
    pub id: usize,
    pub name: String,
    pub axes: [f32; 6],
    pub buttons: [ButtonState; 16],
    pub connected: bool,
}

impl GamepadState {
    /// Get axis value [-1, 1]
    pub fn axis(&self, axis: GamepadAxis) -> f32;
    
    /// Button state
    pub fn is_button_pressed(&self, button: GamepadButton) -> bool;
    pub fn is_button_just_pressed(&self, button: GamepadButton) -> bool;
    pub fn is_button_just_released(&self, button: GamepadButton) -> bool;
}
```

### Gamepad Axis

```rust
pub enum GamepadAxis {
    LeftStickX,   // [-1, 1] left/right
    LeftStickY,   // [-1, 1] up/down
    RightStickX,  // [-1, 1] left/right
    RightStickY,  // [-1, 1] up/down
    LeftTrigger,  // [0, 1] pressure
    RightTrigger, // [0, 1] pressure
}
```

### Gamepad Buttons

```rust
pub enum GamepadButton {
    South,        // A (Xbox), Cross (PS), A (Switch)
    East,         // B (Xbox), Circle (PS), B (Switch)
    North,        // Y (Xbox), Triangle (PS), X (Switch)
    West,         // X (Xbox), Square (PS), Y (Switch)
    C,            // Extra
    Z,            // Extra
    LeftTrigger,  // LT/L2
    RightTrigger, // RT/R2
    LeftTrigger2, // LB/L1
    RightTrigger2,// RB/R1
    Select,       // Back/Share/View
    Start,        // Start/Options/Plus
    Mode,         // Guide/PS/Menu
    LeftThumb,    // L3
    RightThumb,   // R3
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}
```

### Usage

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    if let Some(pad) = ctx.input.gamepad(0) {
        // Movement
        let move_x = pad.axis(GamepadAxis::LeftStickX);
        let move_y = pad.axis(GamepadAxis::LeftStickY);
        if move_x.abs() > 0.1 || move_y.abs() > 0.1 {
            move_player(Vec2::new(move_x, move_y));
        }
        
        // Camera
        let look_x = pad.axis(GamepadAxis::RightStickX);
        let look_y = pad.axis(GamepadAxis::RightStickY);
        rotate_camera(look_x, look_y);
        
        // Triggers
        let lt = pad.axis(GamepadAxis::LeftTrigger);
        let rt = pad.axis(GamepadAxis::RightTrigger);
        if lt > 0.5 { aim(); }
        if rt > 0.5 { shoot(); }
        
        // Buttons
        if pad.is_button_just_pressed(GamepadButton::South) { jump(); }
        if pad.is_button_just_pressed(GamepadButton::East) { dodge(); }
        if pad.is_button_just_pressed(GamepadButton::Start) { pause(); }
    }
}
```

### Deadzone Handling

```rust
fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
    if value.abs() < deadzone { 0.0 } else { value }
}

// Usage
let move_x = apply_deadzone(pad.axis(GamepadAxis::LeftStickX), 0.15);
```

---

## Touch (Android)

```rust
impl Input {
    /// All active touches
    pub fn touches(&self) -> &[TouchState];
    
    /// Touch count
    pub fn touch_count(&self) -> usize;
}

pub struct TouchState {
    pub id: u64,           // Unique per touch
    pub position: Vec2,    // Normalized [0, 1] window coordinates
    pub delta: Vec2,       // Movement since last frame
    pub phase: TouchPhase,
    pub pressure: f32,     // [0, 1] if supported
}

pub enum TouchPhase {
    Started,   // Finger down
    Moved,     // Finger moving
    Stationary,// Finger down, not moving
    Ended,     // Finger up
    Cancelled, // Interrupted (call, notification)
}
```

### Usage

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    for touch in ctx.input.touches() {
        match touch.phase {
            TouchPhase::Started => {
                // Tap or drag start
                if let Some(entity) = raycast(touch.position) {
                    select_entity(entity);
                }
            }
            TouchPhase::Moved => {
                // Drag/rotate
                rotate_camera(touch.delta * 0.01);
            }
            TouchPhase::Ended => {
                // Tap release
                if touch.delta.length() < 0.01 {
                    // It was a tap, not a drag
                    on_tap(touch.position);
                }
            }
            _ => {}
        }
    }
}
```

### Multi-Touch Gestures

```rust
fn handle_gestures(&mut self, touches: &[TouchState]) {
    if touches.len() == 2 {
        let t1 = &touches[0];
        let t2 = &touches[1];
        
        // Pinch zoom
        let current_dist = (t1.position - t2.position).length();
        let prev_dist = (t1.position - t1.delta - (t2.position - t2.delta)).length();
        let zoom_factor = current_dist / prev_dist;
        camera.distance /= zoom_factor;
        
        // Two-finger rotate
        let current_angle = (t2.position - t1.position).y.atan2((t2.position - t1.position).x);
        let prev_angle = ((t2.position - t2.delta) - (t1.position - t1.delta)).y.atan2(
            (t2.position - t2.delta) - (t1.position - t1.delta)).x);
        camera.yaw += current_angle - prev_angle;
    }
}
```

---

## InputMap (Action Binding)

```rust
pub struct InputMap {
    bindings: HashMap<String, InputBinding>,
}

pub enum InputBinding {
    Key(KeyCode),
    MouseButton(MouseButton),
    GamepadAxis(GamepadAxis),
    GamepadButton(GamepadButton),
    Composite(Vec<InputBinding>),  // Any of these
}

impl InputMap {
    pub fn new() -> Self;
    
    /// Bind action to input
    pub fn bind(&mut self, action: &str, binding: InputBinding);
    
    /// Check if action is active
    pub fn is_active(&self, action: &str, input: &Input) -> bool;
    
    /// Check if action just activated
    pub fn is_just_activated(&self, action: &str, input: &Input) -> bool;
    
    /// Get axis value for action (-1 to 1)
    pub fn axis_value(&self, action: &str, input: &Input) -> f32;
}
```

### Usage

```rust
// In Game::create_world or first update
let mut input_map = InputMap::new();

// Keyboard
input_map.bind("move_forward", InputBinding::Key(KeyCode::W));
input_map.bind("move_back", InputBinding::Key(KeyCode::S));
input_map.bind("move_left", InputBinding::Key(KeyCode::A));
input_map.bind("move_right", InputBinding::Key(KeyCode::D));
input_map.bind("jump", InputBinding::Key(KeyCode::Space));
input_map.bind("sprint", InputBinding::Key(KeyCode::LeftShift));

// Mouse
input_map.bind("attack", InputBinding::MouseButton(MouseButton::Left));
input_map.bind("aim", InputBinding::MouseButton(MouseButton::Right));

// Gamepad
input_map.bind("move_x", InputBinding::GamepadAxis(GamepadAxis::LeftStickX));
input_map.bind("move_y", InputBinding::GamepadAxis(GamepadAxis::LeftStickY));
input_map.bind("look_x", InputBinding::GamepadAxis(GamepadAxis::RightStickX));
input_map.bind("look_y", InputBinding::GamepadAxis(GamepadAxis::RightStickY));
input_map.bind("jump", InputBinding::GamepadButton(GamepadButton::South));

// Composite (keyboard OR gamepad)
input_map.bind("jump", InputBinding::Composite(vec![
    InputBinding::Key(KeyCode::Space),
    InputBinding::GamepadButton(GamepadButton::South),
]));

// In update
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let input = ctx.input;
    
    // Digital actions
    if self.input_map.is_just_activated("jump", input) {
        jump();
    }
    if self.input_map.is_active("sprint", input) {
        speed *= 2.0;
    }
    
    // Axis actions
    let move_x = self.input_map.axis_value("move_x", input);
    let move_y = self.input_map.axis_value("move_y", input);
    move_player(Vec2::new(move_x, move_y));
}
```

---

## Text Input

```rust
// Not directly available in NativeActivity
// Use external crate or implement Java bridge

// For desktop:
// winit provides IME events via WindowEvent::ReceivedCharacter
```

---

## Platform Differences

| Feature | Linux | Windows | Android |
|---------|-------|---------|---------|
| Keyboard | ✅ | ✅ | ✅ (physical) |
| Mouse | ✅ | ✅ | ❌ |
| Touch | ❌ | ✅ (touchscreen) | ✅ |
| Gamepad | ✅ | ✅ | ✅ |
| Raw Input | ✅ | ✅ | ❌ |
| IME | ✅ | ✅ | ⚠️ Limited |

---

*See [Game Loop](guides/game-loop.md) for integration with frame loop, or [Runtime API](runtime.md) for `UpdateContext` input access.*