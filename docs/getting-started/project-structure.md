# Project Structure

Understanding the layout of a Soldatik3D game project.

## Generated Project Layout

When you run `soldatik3d new my_game`, this structure is created:

```
my_game/
├── .cargo/
│   └── config.toml          # Cross-compilation settings
├── assets/                   # Game assets (git-tracked)
│   ├── textures/            # PNG, JPG, KTX textures
│   ├── models/              # GLTF/GLB models
│   ├── shaders/             # Custom GLSL shaders
│   └── fonts/               # Bitmap fonts
├── build.rs                  # Build script (asset handling, Android setup)
├── Cargo.toml                # Project manifest
├── src/
│   └── lib.rs                # Main game entry point (cdylib for Android)
├── shaders/                  # Additional custom shaders
├── build_apk.sh              # Android APK build script
├── README.md                 # Project documentation
└── target/                   # Build artifacts (git-ignored)
    ├── debug/
    ├── release/
    ├── aarch64-linux-android/
    │   └── debug/
    │       └── libmy_game.so
    └── x86_64-pc-windows-gnu/
        └── release/
            └── my_game.exe
```

## File Purposes

### `Cargo.toml` - Project Manifest

```toml
[package]
name = "my_game"
version = "0.1.0"
edition = "2024"
license = "GPL-3.0"

[lib]
# Required for Android native-activity
crate-type = ["cdylib", "rlib"]

[features]
default = ["vulkan", "opengl"]
vulkan = ["soldatik3d/vulkan"]
opengl = ["soldatik3d/opengl"]

[dependencies]
soldatik3d = { path = "../../../.." }  # Or version from crates.io
anyhow = "1.0"
```

**Key points:**
- `crate-type = ["cdylib", "rlib"]` - Required for Android `android_main` export
- `cdylib` produces `libmy_game.so` for Android
- `rlib` allows desktop binary to link the same crate

### `src/lib.rs` - Game Entry Point

```rust
use soldatik3d::{run, Game, GameConfig, UpdateContext, World};

struct MyGame;

impl Game for MyGame {
    fn config(&self) -> GameConfig { GameConfig::default() }
    fn create_world(&mut self) -> World { World::default() }
    fn update(&mut self, ctx: &mut UpdateContext<'_>) { /* per-frame logic */ }
}

// Desktop entry point
fn main() -> anyhow::Result<()> { run(MyGame) }

// Android entry point (exported as `android_main`)
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(MyGame, app).unwrap();
}
```

**Key points:**
- `run()` for desktop, `run_android()` for Android
- `#[unsafe(no_mangle)] pub fn android_main` - Required by `android-activity`
- Game logic in `Game` trait implementation

### `build.rs` - Build Script

Handles platform-specific tasks automatically:

```rust
fn main() {
    // 1. Detect target platform
    // 2. Set Android NDK linker env vars (if ANDROID_NDK_HOME set)
    // 3. Copy assets/ to target directory for runtime access
    // 4. Print helpful build info
}
```

**What it does:**
- Detects `TARGET` env var for cross-compilation
- Sets `CARGO_TARGET_<TRIPLE>_LINKER`, `CC_<TRIPLE>`, `AR_<TRIPLE>` for Android
- Copies `assets/` → `target/<target>/<profile>/assets/`
- Runs before every build

### `.cargo/config.toml` - Cross-Compilation Config

Pre-configured for all targets:

```toml
# Linux
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# Windows (MinGW)
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
runner = "wine64"

# Android (set via build.rs / env vars)
# [target.aarch64-linux-android]
# linker = "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
```

### `build_apk.sh` - Android APK Builder

```bash
#!/usr/bin/env bash
# Usage: ./build_apk.sh arm64-v8a [--release]
# 1. Builds libmy_game.so for target ABI
# 2. Copies to android/gradle/app/src/main/jniLibs/<abi>/
# 3. Runs Gradle to build APK
# 4. Outputs to apk/app-<abi>-<debug|release>.apk
```

### `assets/` - Game Assets

```
assets/
├── textures/
│   ├── albedo.png        # Base color
│   ├── normal.png        # Normal map
│   ├── roughness.png     # Roughness map
│   └── metallic.png      # Metallic map
├── models/
│   ├── character.glb     # GLTF 2.0 binary
│   └── environment.gltf  # GLTF 2.0 JSON
├── shaders/
│   ├── custom.frag       # Fragment shader
│   └── custom.vert       # Vertex shader
└── fonts/
    └── font.png          # Bitmap font atlas
```

**Loading at runtime:**
```rust
// In your Game::update or create_world
let texture = ctx.renderer.load_texture("assets/textures/albedo.png")?;
let mesh = ctx.renderer.load_model("assets/models/character.glb")?;
let shader = ctx.renderer.load_shader("assets/shaders/custom.frag")?;
```

## Build Artifacts

### Desktop (Linux/Windows)

```
target/
├── debug/
│   ├── my_game           # Linux executable
│   ├── my_game.exe       # Windows executable (MinGW)
│   ├── libmy_game.rlib   # Rust library
│   └── deps/             # Dependency artifacts
└── release/
    ├── my_game           # Optimized Linux binary
    ├── my_game.exe       # Optimized Windows binary
    └── libmy_game.rlib
```

### Android

```
target/
├── aarch64-linux-android/
│   └── release/
│       └── libmy_game.so    # ARM64 native library
├── armv7-linux-androideabi/
│   └── release/
│       └── libmy_game.so    # ARMv7 native library
└── x86_64-linux-android/
    └── release/
        └── libmy_game.so    # x86_64 native library
```

## Distribution Packages

After `soldatik3d export` or manual packaging:

```
dist/
├── my_game-linux-x86_64.tar.gz      # Linux: binary + libmy_game.rlib
├── my_game-windows-x86_64.zip       # Windows: .exe + libmy_game.rlib
├── my_game-android-arm64-v8a.apk    # Android ARM64 APK
├── my_game-android-armeabi-v7a.apk  # Android ARMv7 APK
└── my_game-android-x86_64.apk       # Android x86_64 APK (optional)
```

## Git Ignore

```gitignore
# Build artifacts
/target

# IDE
.idea/
.vscode/
*.swp

# OS
.DS_Store

# Local config
.cargo/config.toml.local

# Android
android/gradle/local.properties
android/gradle/app/build/
android/gradle/.gradle/
apk/
```

## Multi-Crate Workspaces

For larger projects, split into workspace:

```
my_game/
├── Cargo.toml              # Workspace root
├── game-core/              # Game logic library
│   ├── Cargo.toml
│   └── src/lib.rs
├── game-desktop/           # Desktop binary
│   ├── Cargo.toml
│   └── src/main.rs
├── game-android/           # Android wrapper (minimal)
│   ├── Cargo.toml
│   └── src/lib.rs          # Just exports android_main
└── assets/                 # Shared assets
```

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["game-core", "game-desktop", "game-android"]
resolver = "3"

[workspace.dependencies]
soldatik3d = { path = "../../../.." }
anyhow = "1.0"
```

## Template Customization

The template at `templates/game-project/` can be customized:

1. **Fork the engine repo** and modify `templates/game-project/`
2. **Or copy locally** and use `soldatik3d new --template /path/to/custom`
3. **Or publish** your own template crate

### Template Variables

| Placeholder | Replaced With |
|-------------|---------------|
| `my_game` | Project name (snake_case) |
| `MyGame` | Project name (PascalCase) |
| `My Awesome Game` | Project title (from Cargo.toml) |

---

*See [Quick Start](quick-start.md) to create your first project, or [Your First Game](first-game.md) for a complete tutorial.*