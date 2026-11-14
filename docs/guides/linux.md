# Linux Development

Complete guide for developing and building Soldatik3D games on Linux.

## Supported Distributions

| Distribution | Status | Notes |
|--------------|--------|-------|
| Ubuntu 22.04+ | ✅ Fully tested | Default CI platform |
| Debian 12+ | ✅ Compatible | Same packages as Ubuntu |
| Fedora 38+ | ✅ Compatible | Use `dnf` equivalents |
| Arch Linux | ✅ Compatible | Rolling, latest packages |
| openSUSE | ⚠️ Untested | Should work |
| NixOS | ⚠️ Untested | Requires FHS compat |

## Development Setup

### Install Rust

```bash
# Recommended: rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy

# Add cross-compilation targets
rustup target add x86_64-pc-windows-gnu      # Windows MinGW
rustup target add aarch64-linux-android     # Android ARM64
rustup target add armv7-linux-androideabi   # Android ARMv7
rustup target add x86_64-linux-android      # Android x86_64
```

### Install System Dependencies

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config cmake ninja-build git python3 \
    libvulkan-dev libshaderc-dev \
    libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
    libxkbcommon-dev libwayland-dev \
    libgl1-mesa-dev libegl1-mesa-dev libgles2-mesa-dev \
    clang lld

# Fedora
sudo dnf install -y \
    gcc-c++ cmake ninja-build pkgconf-pkg-config \
    vulkan-devel shaderc-devel \
    libX11-devel libXcursor-devel libXi-devel libXrandr-devel \
    libxkbcommon-devel wayland-devel \
    mesa-libGL-devel mesa-libEGL-devel mesa-libGLES-devel \
    clang lld

# Arch Linux
sudo pacman -S --needed \
    base-devel cmake ninja pkgconf \
    vulkan-icd-loader vulkan-validation-layers shaderc \
    libx11 libxcursor libxi libxrandr libxkbcommon \
    wayland mesa \
    clang lld
```

### GPU Drivers

| Vendor | Installation |
|--------|--------------|
| NVIDIA | `sudo apt-get install nvidia-driver-550` (or distro equivalent) |
| AMD | Included in `mesa` (open source) |
| Intel | Included in `mesa` (open source) |

Verify:
```bash
vulkaninfo --summary | grep "deviceName"
glxinfo | grep "OpenGL renderer"
```

## Building

### Debug Build (Development)

```bash
cd /path/to/your_game
cargo run
```

### Release Build

```bash
cargo build --release
# Binary at target/release/your_game
```

### Run with Specific Backend

```bash
# Vulkan (default)
cargo run

# OpenGL
SOLDATIK3D_BACKEND=opengl cargo run

# Disable VSync for benchmarking
SOLDATIK3D_VSYNC=off cargo run --release
```

### Enable Validation Layers

```bash
# Install layers
sudo apt-get install vulkan-validationlayers-dev

# Run with validation
SOLDATIK3D_VALIDATION=1 cargo run
```

## Cross-Compilation from Linux

### Windows (MinGW)

```bash
# Install MinGW
sudo apt-get install mingw-w64

# Build
cargo build --target x86_64-pc-windows-gnu --release

# Binary at target/x86_64-pc-windows-gnu/release/your_game.exe
```

**Limitations:**
- OpenGL only (Vulkan needs `shaderc` C++ build)
- Requires Wine for testing: `wine64 target/.../your_game.exe`

### Android

```bash
# Set up NDK
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c

# Build native libraries
cargo build --target aarch64-linux-android --release
cargo build --target armv7-linux-androideabi --release

# Libraries at:
# target/aarch64-linux-android/release/libyour_game.so
# target/armv7-linux-androideabi/release/libyour_game.so

# Build APK (requires Android SDK + Gradle)
./build_apk.sh arm64-v8a --release
./build_apk.sh armeabi-v7a --release
```

### All Platforms at Once

```bash
# Using the CLI tool
soldatik3d export

# Or manually
./cross_build.sh all --release
```

## Debugging

### GDB/LLDB

```bash
# With debug symbols (default in debug builds)
gdb target/debug/your_game
# or
lldb target/debug/your_game

# Run with args
(gdb) run --features vulkan
```

### Vulkan Validation

```bash
# Install
sudo apt-get install vulkan-validationlayers-dev

# Enable
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
cargo run

# Or use environment variable
SOLDATIK3D_VALIDATION=1 cargo run
```

### RenderDoc

```bash
# Install
sudo apt-get install renderdoc

# Capture
renderdoc capture target/release/your_game
```

### Tracy Profiler

```bash
# Add to Cargo.toml
tracy-client = { version = "0.16", features = ["callstack"] }

# Build
cargo build --release --features tracy

# Run with profiler
tracy-profiler &
./target/release/your_game
```

## Common Issues

### `libshaderc-dev` Missing Symbols (Ubuntu 24.04)

```bash
# The system libshaderc_combined.a is incomplete
# Workaround:
mkdir -p /tmp/shaderc-lib
ln -sf /usr/lib/x86_64-linux-gnu/libshaderc.so.1 /tmp/shaderc-lib/libshaderc_shared.so
export SHADERC_LIB_DIR=/tmp/shaderc-lib
cargo build
```

### Wayland vs X11

```bash
# Force X11
GDK_BACKEND=x11 cargo run

# Force Wayland (default on Wayland sessions)
GDK_BACKEND=wayland cargo run

# Check current
echo $XDG_SESSION_TYPE
```

### Missing Vulkan ICD

```bash
# Install ICD loader
sudo apt-get install libvulkan1 vulkan-icd-loader

# For NVIDIA
sudo apt-get install libvulkan1 libvulkan1:i386

# Verify
vulkaninfo | grep "ICD"
```

### Linker Errors with LLD

```bash
# Use mold (faster) or gold
sudo apt-get install mold

# In .cargo/config.toml:
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

## Performance Tuning

### Compiler Flags

```toml
# Cargo.toml
[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "debuginfo"
```

### CPU Governor

```bash
# Set performance governor
sudo cpupower frequency-set -g performance

# Or for specific cores
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

### Vulkan ICD Selection

```bash
# List available ICDs
ls /usr/share/vulkan/icd.d/
ls /etc/vulkan/icd.d/

# Force specific ICD
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/radeon_icd.x86_64.json cargo run
```

## Distribution Packaging

### AppImage

```bash
# Install appimagetool
wget https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
chmod +x appimagetool-x86_64.AppImage

# Create AppDir structure
mkdir -p YourGame.AppDir/usr/bin
cp target/release/your_game YourGame.AppDir/usr/bin/
# Add .desktop file, icon, etc.

# Build
./appimagetool-x86_64.AppImage YourGame.AppDir
```

### Flatpak

```yaml
# com.yourgame.Game.yml
app-id: com.yourgame.Game
runtime: org.freedesktop.Platform//23.08
sdk: org.freedesktop.Sdk//23.08
command: your_game
finish-args:
  - --device=dri
  - --filesystem=home
  - --socket=wayland
  - --socket=x11
modules:
  - name: your-game
    buildsystem: simple
    build-commands:
      - cargo build --release --target=x86_64-unknown-linux-gnu
      - install -D target/release/your_game /app/bin/your_game
    sources:
      - type: dir
        path: .
```

### Snap

```yaml
# snap/snapcraft.yaml
name: your-game
version: '1.0'
summary: A Soldatik3D game
description: |
  An awesome 3D game built with Soldatik3D.
grade: stable
confinement: strict
base: core22

apps:
  your-game:
    command: your_game
    plugs: [opengl, wayland, x11, audio-playback]

parts:
  your-game:
    plugin: rust
    source: .
    rust-channel: stable
```

## CI/CD

See [GitHub Actions CI](https://github.com/finesoldat1k/soldatik3D/blob/main/.github/workflows/ci.yml) for reference configuration.

---

*See [Cross-Compilation](cross-compilation.md) for detailed Windows/Android builds from Linux.*