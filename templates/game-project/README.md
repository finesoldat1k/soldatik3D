# My Soldatik3D Game

A game project template for the [Soldatik3D](https://github.com/finesoldat1k/soldatik3D) engine.

## Quick Start

### Prerequisites

- **Rust stable** (edition 2024)
- **Linux**: Vulkan SDK, OpenGL 3.3+, X11/Wayland headers
- **Windows**: MSVC (Visual Studio Build Tools), Vulkan SDK
- **Android**: NDK r26c+, API 29+

### Desktop Development

```bash
# Clone this template (or use `s3d new` if you have the engine)
git clone https://github.com/finesoldat1k/soldatik3d-template my_game
cd my_game

# Run on Linux/macOS/Windows
cargo run

# Run with specific backend
SOLDATIK3D_BACKEND=vulkan cargo run
SOLDATIK3D_BACKEND=opengl cargo run

# Release build
cargo run --release
```

### Using s3d (Soldatik3D CLI)

If you have the engine built locally or installed, use `s3d` for project management:

```bash
# Create a new project from template
s3d new my_game
cd my_game

# Build for all platforms
s3d build --target all --release

# Build Android APK
s3d apk arm64-v8a --release

# Export distributable packages
s3d export --platforms linux,windows,android --out-dir dist

# Validate project configuration
s3d check --target android
```

### Android Development

```bash
# Set up NDK
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build native libraries
cargo build --target aarch64-linux-android
cargo build --target armv7-linux-androideabi

# Build APK (requires Gradle + Android SDK)
./build_apk.sh arm64-v8a
./build_apk.sh armeabi-v7a

# Install to device
adb install -r apk/app-arm64-v8a-debug.apk
```

### Cross-Compilation

```bash
# From Linux to Windows (MinGW)
./cross_build.sh windows

# From Linux to Android
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
./cross_build.sh android arm64-v8a
./cross_build.sh android armeabi-v7a

# All platforms
./cross_build.sh all
```

## Project Structure

```
my_game/
├── .cargo/config.toml      # Cross-compilation settings
├── assets/                 # Game assets (textures, models, shaders)
├── build.rs                # Build script (asset copying, Android setup)
├── Cargo.toml              # Project manifest
├── src/
│   └── lib.rs              # Game entry point (lib + android_main)
├── shaders/                # Custom GLSL shaders (optional)
└── build_apk.sh            # APK build script (Android only)
```

## Configuration

### GameConfig Options

```rust
use soldatik3d::{GameConfig, BackendKind, VsyncMode, WindowHints};

GameConfig::default()
    .with_title("My Game")
    .with_window_size(1920, 1080)
    .with_fullscreen(false)
    .with_vsync(true)
    .with_backend(BackendKind::Vulkan)  // or OpenGl
    .with_window_hints(WindowHints {
        resizable: true,
        vsync: VsyncMode::On,
        ..Default::default()
    })
```

### Environment Variables

| Variable | Values | Description |
|----------|--------|-------------|
| `SOLDATIK3D_BACKEND` | `vulkan`, `opengl` | Force graphics backend |
| `SOLDATIK3D_VSYNC` | `on`, `off`, `adaptive`, `mailbox` | VSync mode |
| `SOLDATIK3D_MAX_FRAMES` | `<number>` | Auto-exit after N frames (benchmarking) |
| `SOLDATIK3D_COMMANDS` | `"cmd1;cmd2"` | Run console commands on startup |
| `SOLDATIK3D_CONSOLE` | `1` | Open console at startup |
| `SOLDATIK3D_DEBUG_VIEW` | `normals`, `depth`, `wireframe` | Initial debug view |
| `SOLDATIK3D_VALIDATION` | `1` | Enable Vulkan validation layers |

## Game Loop

Your game implements the `Game` trait:

```rust
use soldatik3d::{Game, GameConfig, UpdateContext, World};

struct MyGame;

impl Game for MyGame {
    fn config(&self) -> GameConfig { GameConfig::default() }

    fn create_world(&mut self) -> World {
        // Create cameras, lights, spawn entities
        World::default()
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // Called every frame
        // Access: ctx.input, ctx.world, ctx.renderer, ctx.commands, ctx.console
    }
}
```

## Input Handling

```rust
// Keyboard
if input.is_key_pressed(KeyCode::W) { ... }
if input.is_key_just_pressed(KeyCode::Space) { ... }

// Mouse
let mouse_pos = input.mouse_position();
let mouse_delta = input.mouse_delta();
if input.is_mouse_button_pressed(MouseButton::Left) { ... }

// Gamepad
if let Some(gamepad) = input.gamepad(0) {
    let left_stick = gamepad.axis(GamepadAxis::LeftStickX);
}
```

## Console Commands

Built-in commands available at runtime:

```
help                    # Show all commands
clear                   # Clear console
quit / exit             # Exit game
backend                 # Show current backend
backend vulkan          # Switch to Vulkan
backend opengl          # Switch to OpenGL
vsync.on / vsync.off    # Toggle VSync
vsync.adaptive          # Adaptive VSync
vsync.mailbox           # Mailbox VSync
screenshot              # Save PNG
camera.list             # List cameras
camera.select <name>    # Switch camera
camera.spawn <name>     # Create camera
camera.remove <name>    # Delete camera
debug.lit               # Normal rendering
debug.normals           # Show normals
debug.depth             # Show depth
debug.wireframe         # Wireframe mode
```

Add custom commands:

```rust
ctx.commands.register(Command::new(
    "my_command",
    "Description",
    |ctx| {
        ctx.console.print("Hello from my command!");
        CommandResult::Ok(None)
    },
));
```

## Assets

Place assets in the `assets/` directory:

```
assets/
├── textures/
│   ├── albedo.png
│   ├── normal.png
│   └── roughness.png
├── models/
│   └── mesh.glb
├── shaders/
│   └── custom.frag
└── fonts/
    └── font.png
```

Load in your game:

```rust
let texture = ctx.renderer.load_texture("assets/textures/albedo.png")?;
let mesh = ctx.renderer.load_model("assets/models/mesh.glb")?;
```

## Building for Distribution

### Linux

```bash
cargo build --release
# Binary at: target/release/my_game
# Package with: tar -czf my_game-linux.tar.gz target/release/my_game
```

### Windows

```bash
# Cross-compile from Linux
./cross_build.sh windows --release
# Or build natively on Windows
cargo build --release
# Binary at: target/release/my_game.exe
```

### Android

```bash
# Build native libraries
cargo build --release --target aarch64-linux-android
cargo build --release --target armv7-linux-androideabi

# Build APK
./build_apk.sh arm64-v8a --release
./build_apk.sh armeabi-v7a --release
# APK at: apk/app-arm64-v8a-release.apk
```

## Extending the Engine

### Custom Render Passes

```rust
// In your game, access the renderer directly
ctx.renderer.add_custom_pass(|renderer, frame| {
    // Custom rendering logic
    Ok(())
});
```

### Custom Shaders

Place GLSL shaders in `assets/shaders/` and load them:

```rust
let shader = ctx.renderer.load_shader("assets/shaders/custom.frag")?;
```

### Plugin System (Advanced)

```rust
// Create a plugin crate that depends on soldatik3d
// Register systems, resources, and render passes
```

## Troubleshooting

### Shader Compilation Errors (Vulkan)
- Ensure `shaderc` and `glslang` are installed
- Set `SHADERC_LIB_DIR` if using system shaderc
- Check `SOLDATIK3D_VALIDATION=1` for detailed errors

### Android Build Fails
- Verify `ANDROID_NDK_HOME` points to NDK r26c+
- Ensure `rustup target add aarch64-linux-android armv7-linux-androideabi`
- Check Gradle/Android SDK for APK builds

### Linker Errors
- Linux: Install `libshaderc-dev` or build shaderc from source
- Windows: Use MSVC toolchain for Vulkan backend
- Cross-compile: Set `CARGO_TARGET_<TRIPLE>_LINKER` correctly

## License

GPL-3.0 - See [LICENSE](../LICENSE) for details.

---

Generated from [Soldatik3D Game Template](https://github.com/finesoldat1k/soldatik3d-template)