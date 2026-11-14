# Quick Start - Your First Game in 5 Minutes

This guide will get you from zero to a running 3D game in under 5 minutes.

## Prerequisites

- **Rust stable** (2024 edition): `rustup default stable`
- **Linux**: `sudo apt-get install build-essential pkg-config cmake libvulkan-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev`
- **Windows**: Visual Studio Build Tools with C++, Vulkan SDK
- **Android**: NDK r26c+, `rustup target add aarch64-linux-android armv7-linux-androideabi`

## Install the CLI Tool

```bash
# From crates.io (when published)
cargo install soldatik3d-cli

# Or build from source
git clone https://github.com/finesoldat1k/soldatik3D
cd soldatik3D
cargo install --path tools/soldatik3d-cli
```

## Create Your First Project

```bash
# Create a new game project
soldatik3d new my_awesome_game
cd my_awesome_game

# Run it immediately!
cargo run
```

You should see a window with a 3D scene, a rotating camera, and colored lights. Press `F1-F4` for debug views, `F12` for screenshots, `~` for console.

## Project Structure

```
my_awesome_game/
├── .cargo/config.toml      # Cross-compilation settings
├── assets/                 # Your textures, models, shaders
├── build.rs                # Build script (asset copying, Android setup)
├── Cargo.toml              # Project manifest
├── src/
│   └── lib.rs              # Game entry point
├── shaders/                # Custom GLSL shaders
├── build_apk.sh            # Android APK builder
└── README.md               # This file
```

## The Game Code

Open `src/lib.rs` - this is where your game lives:

```rust
use soldatik3d::{run, Game, GameConfig, UpdateContext, World};

struct MyGame;

impl Game for MyGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("My Awesome Game")
            .with_window_size(1280, 720)
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();
        // Add cameras, lights, entities here
        world
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // Called every frame - your game logic goes here
    }
}

fn main() -> anyhow::Result<()> {
    run(MyGame)
}
```

## Run with Different Backends

```bash
# Vulkan (default, best performance)
cargo run

# OpenGL (fallback, wider compatibility)
SOLDATIK3D_BACKEND=opengl cargo run

# Disable VSync for benchmarking
SOLDATIK3D_VSYNC=off cargo run --release
```

## Build for Release

```bash
# Optimized build
soldatik3d build --release

# Or manually
cargo build --release
```

The binary will be at `target/release/my_awesome_game`.

## Build for Android

```bash
# Set up NDK
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c

# Build native libraries
soldatik3d build --target android

# Build APK
soldatik3d apk arm64-v8a --release

# Install on device
adb install -r apk/app-arm64-v8a-release.apk
```

## Build for Windows (from Linux)

```bash
# Install MinGW
sudo apt-get install mingw-w64

# Cross-compile
soldatik3d build --target windows --release
```

## Next Steps

1. **Read [Your First Game](first-game.md)** - Build a complete small game
2. **Learn [Game Loop](guides/game-loop.md)** - Understand update/render cycle
3. **Explore [API Reference](api/runtime.md)** - Full API documentation
4. **Check [Examples](examples/basic.md)** - See more complex games

## Common Issues

| Issue | Solution |
|-------|----------|
| `linker not found` | Install `clang` (Linux) or Visual Studio (Windows) |
| `Vulkan not found` | Install `libvulkan-dev` / Vulkan SDK |
| `Android build fails` | Check `ANDROID_NDK_HOME`, run `rustup target add ...` |
| `Shader compilation error` | Set `SOLDATIK3D_VALIDATION=1` for details |

## Get Help

- **Console**: Press `~` in-game for live commands
- **Docs**: `soldatik3d info` shows engine capabilities
- **Validate**: `soldatik3d check` validates your project setup
- **GitHub**: [Issues](https://github.com/finesoldat1k/soldatik3D/issues) / [Discussions](https://github.com/finesoldat1k/soldatik3D/discussions)

---

*You're now ready to build amazing 3D games with Soldatik3D! 🎮*