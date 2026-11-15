# Platform API

The `platform` crate owns `winit`, creates the window and aggregates input. Its
public types are re-exported by `runtime`.

```rust
use runtime::{
    Input, InputMap, PlatformCapabilities, TargetArch, TargetOS, WindowHints,
};
use runtime::winit;
```

## `TargetOS`

```rust
pub enum TargetOS {
    Linux,
    Windows,
    Android,
    Unknown,
}
```

| Method | Description |
|--------|-------------|
| `TargetOS::current()` | Compile-time target, as a `const fn` |
| `name()` | `"Linux"`, `"Windows"`, `"Android"`, `"Unknown"` |
| `is_mobile()` | True for Android |
| `is_desktop()` | True for Linux and Windows |
| `supports_vulkan()` | True on desktop |
| `supports_opengl()` | True on all supported targets |
| `default_backend()` | Vulkan on desktop, OpenGL on Android |

The enum contains no macOS, iOS or WebAssembly variants. Unsupported targets
report `Unknown`; the engine does not build there.

```rust
match TargetOS::current() {
    TargetOS::Android => { /* fullscreen UI, physical viewport size */ }
    TargetOS::Linux | TargetOS::Windows => { /* desktop controls */ }
    TargetOS::Unknown => { /* fall back to OpenGL */ }
}
```

Prefer compile-time `#[cfg(target_os = "...")]` when the code needs an
Android-only type such as `AndroidApp`.

## `TargetArch`

```rust
pub enum TargetArch {
    X86_64,
    AArch64,
    ARMv7,
    Unknown,
}
```

| Method | Description |
|--------|-------------|
| `TargetArch::current()` | Compile-time architecture |
| `name()` | `"x86_64"`, `"aarch64"`, `"armv7"`, `"unknown"` |
| `is_64bit()` | True for x86_64 and aarch64 |

## `PlatformCapabilities`

Detected at runtime on the machine that runs the game.

```rust
pub struct PlatformCapabilities {
    pub vulkan_version: Option<u32>,
    pub vulkan_extensions: Vec<String>,
    pub opengl_version: Option<String>,
    pub opengl_extensions: Vec<String>,
    pub virtualized: bool,
    pub cpu_cores: usize,
    pub total_memory_mb: usize,
    pub gpu_vendor: Option<String>,
    pub gpu_renderer: Option<String>,
    pub high_dpi: bool,
    pub touch_support: bool,
    pub gamepad_support: bool,
}
```

`PlatformCapabilities::detect()` fills what it can without creating a graphics
context. CPU cores come from `std::thread::available_parallelism`; on Linux it
also reads `/proc/cpuinfo` and `/proc/meminfo` for virtualization and memory.
GPU and extension fields stay `None`/empty until a backend is created, so do
not treat them as a hardware probe.

`meets_minimum_requirements()` returns true for at least 2 CPU cores and
2 GB of memory.

## `WindowHints`

Configuration values for a future window configuration path. The runtime
currently creates its window from `GameConfig`; `WindowHints` is useful to
platform-aware tools and custom integrations.

```rust
pub struct WindowHints {
    pub preferred_backend: Option<BackendKind>,
    pub vsync: bool,
    pub fullscreen: bool,
    pub transparent: bool,
    pub high_dpi: bool,
    pub resizable: bool,
    pub title: String,
    pub size: Option<(u32, u32)>,
    pub min_size: Option<(u32, u32)>,
    pub max_size: Option<(u32, u32)>,
}
```

`WindowHints::for_platform()` sets Android to fullscreen, non-resizable, VSync
on and the full display size. Desktop keeps a resizable 1280×720 window with a
640×480 minimum.

## `winit` re-export

Games use `runtime::winit` instead of adding `winit` to their own manifest:

```rust
use runtime::winit::keyboard::{KeyCode, PhysicalKey};
use runtime::winit::event::MouseButton;
```

This keeps the `winit` version aligned between the engine and the game. The
Android entry point type is also reached through the re-export:

```rust
#[cfg(target_os = "android")]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    // ...
}
```

Do not create a second `winit` event loop inside a game: `winit` allows only one
per process, and the engine already owns it.

## Window and surface internals

`PlatformWindow` wraps a `winit::window::Window`:

```rust
impl PlatformWindow {
    pub fn new(event_loop: &ActiveEventLoop, title: &str, width: u32, height: u32)
        -> anyhow::Result<Self>;
    pub fn id(&self) -> WindowId;
    pub fn request_redraw(&self);
    pub fn inner_size(&self) -> (u32, u32);
    pub fn set_title(&self, title: &str);
    pub fn surface(&self) -> anyhow::Result<WindowSurface>;
    pub fn set_cursor_captured(&self, captured: bool) -> bool;
}
```

`surface()` produces the backend-neutral `render::WindowSurface` from raw
handles. `set_cursor_captured` returns whether capture succeeded; on compositors
that refuse pointer lock the engine stays in menu mode.

Games never construct these types; they are listed here for engine and tool
development.

## Input ownership

The runtime feeds `winit` events into `Input` and calls `Input::begin_frame`
before each update. Games read the snapshot through `UpdateContext::input` and
cannot mutate it. See the [Input API](input.md).

## Related

- [Rendering API](rendering.md)
- [Runtime API](runtime.md)
- [Android guide](../guides/android.md)
- [Windows guide](../guides/windows.md)
- [Linux guide](../guides/linux.md)
