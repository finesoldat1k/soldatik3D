# soldatik3d CLI Reference

Complete reference for the `soldatik3d` command-line tool.

## Installation

```bash
# From crates.io (when published)
cargo install soldatik3d-cli

# From source
git clone https://github.com/finesoldat1k/soldatik3D
cd soldatik3D
cargo install --path tools/soldatik3d-cli
```

## Global Options

```bash
soldatik3d [OPTIONS] <COMMAND>

Options:
  -v, --verbose    Verbose output
  -h, --help       Print help
  -V, --version    Print version
```

## Commands

### `new` - Create New Project

```bash
soldatik3d new <NAME> [OPTIONS]

Arguments:
  <NAME>    Project name (directory name)

Options:
  -d, --dir <PATH>        Target directory [default: .]
  -t, --template <PATH>   Custom template path
  --git / --no-git        Initialize git repository [default: true]
```

**Examples:**
```bash
# Create in current directory
soldatik3d new my_game

# Create in specific directory
soldatik3d new my_game --dir ~/projects

# Use custom template
soldatik3d new my_game --template /path/to/custom/template
```

**Created Structure:**
```
my_game/
├── .cargo/config.toml
├── assets/
├── build.rs
├── Cargo.toml
├── src/lib.rs
├── shaders/
├── build_apk.sh
└── README.md
```

---

### `build` - Build Project

```bash
soldatik3d build [OPTIONS]

Options:
  --target <TARGET>       Target platform [default: linux]
                          [linux, windows, android, all]
  -r, --release           Build in release mode
  -f, --features <FEATS>  Features to enable [default: vulkan,opengl]
  -o, --out-dir <PATH>    Output directory
```

**Targets:**
| Target | Platform | Backend |
|--------|----------|---------|
| `linux` | Linux x86_64 | Vulkan + OpenGL |
| `windows` | Windows MinGW | OpenGL only |
| `android` | Android ARM64/ARMv7 | OpenGL ES |
| `all` | All platforms | Platform-appropriate |

**Examples:**
```bash
# Debug build for current platform
soldatik3d build

# Release build for Linux
soldatik3d build --target linux --release

# Cross-compile Windows from Linux
soldatik3d build --target windows --release

# Build Android libraries
soldatik3d build --target android --release

# Build all platforms
soldatik3d build --target all --release

# Custom features
soldatik3d build --features opengl --release
```

**Environment Variables (Android):**
```bash
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
soldatik3d build --target android --release
```

---

### `run` - Run Game

```bash
soldatik3d run [OPTIONS]

Options:
  --target <TARGET>   Target platform [default: linux]
  -r, --release       Run release build
  -- <ARGS>...        Additional cargo arguments
```

**Note:** Android target not supported (use `apk` command).

**Examples:**
```bash
# Run debug build
soldatik3d run

# Run release build
soldatik3d run --release

# Run with OpenGL backend
SOLDATIK3D_BACKEND=opengl soldatik3d run

# Run with custom args
soldatik3d run -- --features vulkan
```

---

### `apk` - Build Android APK

```bash
soldatik3d apk <ABI> [OPTIONS]

Arguments:
  <ABI>    Target ABI [arm64-v8a, armeabi-v7a, x86_64]

Options:
  -r, --release    Build release APK
  --sign           Sign APK (requires keystore config)
```

**Prerequisites:**
- Android NDK (`ANDROID_NDK_HOME`)
- Android SDK (`ANDROID_SDK_ROOT` or `ANDROID_HOME`)
- Gradle 8.4+

**Examples:**
```bash
# Debug APK for ARM64
soldatik3d apk arm64-v8a

# Release APK for ARMv7
soldatik3d apk armeabi-v7a --release

# Build both ABIs
soldatik3d apk arm64-v8a --release
soldatik3d apk armeabi-v7a --release
```

**Output:**
```
apk/app-arm64-v8a-debug.apk
apk/app-armeabi-v7a-release.apk
```

**Install on Device:**
```bash
adb install -r apk/app-arm64-v8a-release.apk
```

---

### `export` - Export Distributable Packages

```bash
soldatik3d export [OPTIONS]

Options:
  -p, --platforms <LIST>  Platforms to export [default: linux,windows,android]
                          Comma-separated: linux,windows,android
  -o, --out-dir <PATH>    Output directory [default: dist]
```

**Examples:**
```bash
# Export all platforms
soldatik3d export

# Export specific platforms
soldatik3d export --platforms linux,android

# Custom output directory
soldatik3d export --out-dir ./releases/v1.0.0
```

**Output Structure:**
```
dist/
├── my_game-linux-x86_64.tar.gz
├── my_game-windows-x86_64.zip
├── my_game-android-arm64-v8a.apk
└── my_game-android-armeabi-v7a.apk
```

---

### `info` - Show Engine Info

```bash
soldatik3d info

Output:
Soldatik3D Engine
Version: 0.1.0
Edition: 2024

Supported Platforms:
  • Linux x86_64 (Vulkan + OpenGL)
  • Windows x86_64 MSVC (Vulkan + OpenGL)
  • Windows x86_64 GNU (OpenGL only)
  • Android arm64-v8a (OpenGL ES 3.0)
  • Android armeabi-v7a (OpenGL ES 3.0)

Features:
  • Deferred rendering with HDR, SSAO, Bloom
  • Physically-based rendering (PBR)
  • Dynamic shadows (cascaded shadow maps)
  • In-game console with command system
  • Hot-reloadable shaders
  • Cross-platform input abstraction

Environment Variables:
  SOLDATIK3D_BACKEND=vulkan|opengl
  SOLDATIK3D_VSYNC=on|off|adaptive|mailbox
  SOLDATIK3D_MAX_FRAMES=<number>
  SOLDATIK3D_COMMANDS="cmd1;cmd2"
  SOLDATIK3D_CONSOLE=1
  SOLDATIK3D_DEBUG_VIEW=normals|depth|wireframe
  SOLDATIK3D_VALIDATION=1
```

---

### `engine` - Engine Management

```bash
soldatik3d engine <ACTION>

Actions:
  version     Show engine version and path
  update      Update to latest version
  link <PATH> Link local engine for development
```

**Examples:**
```bash
# Show version
soldatik3d engine version

# Link local engine (for development)
soldatik3d engine link /path/to/soldatik3D

# Update engine
soldatik3d engine update
```

---

### `check` - Validate Project

```bash
soldatik3d check [OPTIONS]

Options:
  --target <TARGET>  Check specific platform [linux, windows, android]
```

**Checks:**
- Cargo.toml has correct `crate-type = ["cdylib", "rlib"]`
- `build.rs` exists
- `.cargo/config.toml` exists
- Android NDK configured (if targeting Android)
- Vulkan/OpenGL dependencies available

**Examples:**
```bash
# Check all platforms
soldatik3d check

# Check Android setup
soldatik3d check --target android
```

---

## Configuration Files

### Project `Cargo.toml`

```toml
[package]
name = "my_game"
version = "0.1.0"
edition = "2024"
license = "GPL-3.0"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["vulkan", "opengl"]
vulkan = ["soldatik3d/vulkan"]
opengl = ["soldatik3d/opengl"]

[dependencies]
soldatik3d = { path = "../../../.." }  # Or version from crates.io
anyhow = "1.0"
```

### `.cargo/config.toml`

```toml
# Cross-compilation settings
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
runner = "wine64"
rustflags = ["-C", "link-arg=-fuse-ld=lld", "-C", "link-arg=-static"]

# Android (set via build.rs / env)
# [target.aarch64-linux-android]
# linker = "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
```

### `build.rs`

```rust
fn main() {
    // Auto-configures Android NDK linker
    // Copies assets/ to target directory
    // Prints build info
}
```

---

## Environment Variables

| Variable | Values | Description |
|----------|--------|-------------|
| `SOLDATIK3D_BACKEND` | `vulkan`, `opengl` | Force graphics backend |
| `SOLDATIK3D_VSYNC` | `on`, `off`, `adaptive`, `mailbox` | VSync mode |
| `SOLDATIK3D_MAX_FRAMES` | `<number>` | Auto-exit after N frames |
| `SOLDATIK3D_COMMANDS` | `"cmd1;cmd2"` | Run commands at startup |
| `SOLDATIK3D_CONSOLE` | `1` | Open console at startup |
| `SOLDATIK3D_DEBUG_VIEW` | `normals`, `depth`, `wireframe` | Initial debug view |
| `SOLDATIK3D_VALIDATION` | `1` | Enable Vulkan validation |
| `ANDROID_NDK_HOME` | `<path>` | Android NDK path |
| `ANDROID_SDK_ROOT` | `<path>` | Android SDK path |
| `SHADERC_LIB_DIR` | `<path>` | System shaderc library |
| `CMAKE_POLICY_VERSION_MINIMUM` | `3.5` | CMake compatibility |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Project not found |
| 4 | Build failed |
| 5 | Missing dependencies |
| 6 | Android NDK not found |
| 7 | Gradle build failed |

---

## Shell Completion

```bash
# Bash
soldatik3d --completion bash > /etc/bash_completion.d/soldatik3d

# Zsh
soldatik3d --completion zsh > ~/.zsh/completions/_soldatik3d

# Fish
soldatik3d --completion fish > ~/.config/fish/completions/soldatik3d.fish
```

---

## Troubleshooting

### `command not found: soldatik3d`

```bash
# Add cargo bin to PATH
export PATH="$HOME/.cargo/bin:$PATH"
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```

### `Android NDK not found`

```bash
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
# Verify
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang
```

### `MinGW not found`

```bash
# Ubuntu/Debian
sudo apt-get install mingw-w64

# Verify
x86_64-w64-mingw32-gcc --version
```

### `Gradle not found`

```bash
# Install Gradle
sudo apt-get install gradle
# Or use included gradlew
chmod +x gradlew
./gradlew assembleRelease
```

---

*See [Getting Started](getting-started/quick-start.md) for quick start guide, or [Platform Guides](guides/) for platform-specific details.*