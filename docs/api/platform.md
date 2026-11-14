# Platform API Reference

Complete API documentation for platform abstraction, window management, and platform detection.

## Window Management

### WindowHints

```rust
pub struct WindowHints {
    /// Window can be resized by user
    pub resizable: bool,
    
    /// Vertical sync mode
    pub vsync: VsyncMode,
    
    /// Transparent window background
    pub transparent: bool,
    
    /// Window decorations (title bar, borders)
    pub decorations: bool,
    
    /// Start maximized
    pub maximized: bool,
    
    /// Window initially visible
    pub visible: bool,
    
    /// Cursor visibility
    pub cursor_visible: bool,
    
    /// Cursor locked to center (FPS style)
    pub cursor_locked: bool,
    
    /// Cursor confined to window
    pub cursor_grabbed: bool,
    
    /// Minimum window size
    pub min_size: Option<(u32, u32)>,
    
    /// Maximum window size
    pub max_size: Option<(u32, u32)>,
}

impl Default for WindowHints {
    fn default() -> Self {
        Self {
            resizable: true,
            vsync: VsyncMode::On,
            transparent: false,
            decorations: true,
            maximized: false,
            visible: true,
            cursor_visible: true,
            cursor_locked: false,
            cursor_grabbed: false,
            min_size: None,
            max_size: None,
        }
    }
}
```

### Usage in GameConfig

```rust
GameConfig::default()
    .with_window_hints(WindowHints {
        resizable: true,
        vsync: VsyncMode::On,
        decorations: true,
        cursor_visible: false,    // Hide for FPS
        cursor_locked: true,      // Lock to center
        min_size: Some((800, 600)),
        max_size: None,
        ..Default::default()
    })
```

### Runtime Window Control

```rust
// Not directly exposed in UpdateContext
// Use GameConfig for initial setup
// For runtime changes, use platform-specific APIs or request restart
```

---

## Platform Detection

### TargetOS

```rust
pub enum TargetOS {
    Linux,
    Windows,
    Android,
    Unknown,
}

impl TargetOS {
    pub fn current() -> Self;
    pub fn is_linux(&self) -> bool;
    pub fn is_windows(&self) -> bool;
    pub fn is_android(&self) -> bool;
}
```

### TargetArch

```rust
pub enum TargetArch {
    X86_64,
    Aarch64,
    Arm,
    Unknown,
}

impl TargetArch {
    pub fn current() -> Self;
    pub fn is_x86_64(&self) -> bool;
    pub fn is_aarch64(&self) -> bool;
    pub fn is_arm(&self) -> bool;
}
```

### PlatformCapabilities

```rust
pub struct PlatformCapabilities {
    /// Vulkan available
    pub vulkan: bool,
    
    /// OpenGL available
    pub opengl: bool,
    
    /// OpenGL ES available
    pub opengl_es: bool,
    
    /// Maximum texture size
    pub max_texture_size: u32,
    
    /// Maximum anisotropy
    pub max_anisotropy: f32,
    
    /// Supported texture formats
    pub supported_formats: Vec<TextureFormat>,
    
    /// Multi-GPU support
    pub multi_gpu: bool,
}

impl PlatformCapabilities {
    pub fn detect() -> Self;
}
```

### Usage

```rust
fn create_world(&mut self) -> World {
    let caps = PlatformCapabilities::detect();
    
    if caps.vulkan {
        // Use Vulkan-specific features
    }
    
    if caps.opengl_es {
        // Adjust for mobile GPU limits
        // Lower texture resolution, simpler shaders
    }
    
    // Check max texture size
    if caps.max_texture_size < 4096 {
        // Downscale large textures
    }
    
    World::default()
}
```

### Compile-Time Detection

```rust
#[cfg(target_os = "linux")]
fn linux_only() { }

#[cfg(target_os = "windows")]
fn windows_only() { }

#[cfg(target_os = "android")]
fn android_only() { }

#[cfg(target_arch = "x86_64")]
fn x86_64_only() { }

#[cfg(target_arch = "aarch64")]
fn arm64_only() { }
```

---

## Android Specific

### AndroidApp

```rust
// From android-activity crate
pub struct AndroidApp {
    // JNI environment
    // Activity reference
    // Window/surface
    // Input queue
    // Configuration
}

impl AndroidApp {
    pub fn window(&self) -> Option<&NativeWindow>;
    pub fn activity(&self) -> &JObject;
    pub fn config(&self) -> Configuration;
    pub fn input_queue(&self) -> &InputQueue;
    pub fn looper(&self) -> &Looper;
}
```

### Android Entry Point

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(MyGame, app).unwrap();
}
```

### Android-Specific Features

```rust
#[cfg(target_os = "android")]
fn android_specific(ctx: &mut UpdateContext<'_>) {
    // Touch input
    for touch in ctx.input.touches() {
        // Handle touch
    }
    
    // Sensor access (via JNI)
    // Not directly exposed, use JNI bridge
    
    // Lifecycle events handled by NativeActivity
    // onPause/onResume automatic
}
```

---

## Windows Specific

### Windows-Specific Features

```rust
#[cfg(target_os = "windows")]
fn windows_specific(ctx: &mut UpdateContext<'_>) {
    // Raw input (high precision mouse)
    // Available via winit
    
    // HDR support
    // Check PlatformCapabilities
    
    // DirectX/D3D12 interop
    // Not directly exposed
}
```

---

## Linux Specific

### Linux-Specific Features

```rust
#[cfg(target_os = "linux")]
fn linux_specific() {
    // Wayland vs X11
    // Check via std::env::var("XDG_SESSION_TYPE")
    
    // DRI/PRIME for multi-GPU
    // DRI_PRIME=1 cargo run
    
    // Vulkan ICD selection
    // VK_ICD_FILENAMES=...
}
```

---

## Window Surface

```rust
pub struct WindowSurface {
    /// Raw window handle (platform-specific)
    pub raw_handle: RawWindowHandle,
    
    /// Raw display handle
    pub raw_display: RawDisplayHandle,
    
    /// Window size in pixels
    pub size: (u32, u32),
    
    /// Framebuffer size (may differ on HiDPI)
    pub framebuffer_size: (u32, u32),
    
    /// Content scale factor (HiDPI)
    pub scale_factor: f32,
    
    /// Is window minimized
    pub minimized: bool,
    
    /// Is window focused
    pub focused: bool,
}
```

### Accessing Surface

```rust
// In Game::config
GameConfig::default()
    .with_window_size(1280, 720)

// Runtime access via Renderer
impl Renderer {
    pub fn extent(&self) -> (u32, u32);  // Window size
    pub fn framebuffer_extent(&self) -> (u32, u32);  // FB size
    pub fn scale_factor(&self) -> f32;  // HiDPI scale
}
```

---

## Input Platform Differences

| Feature | Linux | Windows | Android |
|---------|-------|---------|---------|
| Keyboard | ✅ | ✅ | ✅ (physical) |
| Mouse | ✅ | ✅ | ❌ |
| Touch | ❌ | ✅ (touchscreen) | ✅ |
| Gamepad | ✅ | ✅ | ✅ |
| Raw Input | ✅ | ✅ | ❌ |
| IME | ✅ | ✅ | ⚠️ |
| High DPI | ✅ | ✅ | ✅ |

---

## Display/Monitor

```rust
pub struct Monitor {
    pub name: String,
    pub size: (u32, u32),        // Physical mm
    pub resolution: (u32, u32),  // Current mode
    pub position: (i32, i32),    // Virtual desktop coords
    pub scale_factor: f32,       // HiDPI
    pub is_primary: bool,
    pub refresh_rate: u32,       // Hz
    pub video_modes: Vec<VideoMode>,
}

pub struct VideoMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub bit_depth: u32,
}

impl Window {
    pub fn current_monitor(&self) -> Option<Monitor>;
    pub fn available_monitors() -> Vec<Monitor>;
    pub fn primary_monitor() -> Option<Monitor>;
}
```

---

## Clipboard

```rust
pub struct Clipboard {
    pub fn read_text(&self) -> Option<String>;
    pub fn write_text(&mut self, text: &str);
}

// Access via Window
impl Window {
    pub fn clipboard(&self) -> Clipboard;
}
```

---

## Cursor

```rust
pub enum CursorIcon {
    Default,
    Crosshair,
    Hand,
    Arrow,
    Move,
    Text,
    Wait,
    Help,
    Progress,
    NotAllowed,
    ContextMenu,
    Cell,
    VerticalText,
    Alias,
    Copy,
    NoDrop,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

impl Window {
    pub fn set_cursor_icon(&self, icon: CursorIcon);
    pub fn set_cursor_visible(&self, visible: bool);
    pub fn set_cursor_grab(&self, grab: CursorGrabMode) -> Result<(), GrabError>;
    pub fn set_cursor_position(&self, x: f64, y: f64) -> Result<(), CursorError>;
}

pub enum CursorGrabMode {
    None,
    Confined,
    Locked,
}
```

---

## Fullscreen Modes

```rust
pub enum Fullscreen {
    /// Windowed
    Windowed,
    
    /// Borderless fullscreen on specific monitor
    Borderless(Monitor),
    
    /// Exclusive fullscreen (mode change)
    Exclusive(VideoMode),
}

impl Window {
    pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>);
    pub fn fullscreen(&self) -> Option<Fullscreen>;
}
```

---

## Platform-Specific Environment Variables

| Variable | Platform | Description |
|----------|----------|-------------|
| `DISPLAY` | Linux (X11) | X server display |
| `WAYLAND_DISPLAY` | Linux (Wayland) | Wayland socket |
| `XDG_SESSION_TYPE` | Linux | `x11` or `wayland` |
| `VK_ICD_FILENAMES` | All | Vulkan ICD override |
| `VK_LAYER_PATH` | All | Vulkan layer path |
| `ANDROID_NDK_HOME` | Android | NDK path |
| `ANDROID_SDK_ROOT` | Android | SDK path |
| `VULKAN_SDK` | Windows | Vulkan SDK path |

---

*See [Runtime API](runtime.md) for `GameConfig` and `WindowHints`, or [Platform Guides](guides/) for platform-specific setup.*