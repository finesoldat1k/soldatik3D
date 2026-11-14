# Installation Guide

Complete setup instructions for all supported platforms.

## Rust Toolchain

Soldatik3D requires **Rust 2024 edition** (Rust 1.78+).

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Verify
rustc --version  # Should be 1.78+
cargo --version

# Add targets for cross-compilation
rustup target add x86_64-pc-windows-gnu      # Windows (MinGW)
rustup target add aarch64-linux-android     # Android ARM64
rustup target add armv7-linux-androideabi   # Android ARMv7
rustup target add x86_64-linux-android      # Android x86_64 (optional)
```

## Linux (Ubuntu/Debian)

### Minimum Requirements
- Ubuntu 22.04+ / Debian 12+ / Fedora 38+ / Arch Linux
- Kernel 5.15+
- Vulkan-capable GPU with drivers

### Install Dependencies

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

# Optional: for shaderc development
sudo apt-get install -y spirv-tools glslang-tools

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

### Verify Installation

```bash
# Check Vulkan
vulkaninfo --summary | head -20

# Check OpenGL
glxinfo | grep "OpenGL version"

# Test compile
cargo new test_soldatik3d && cd test_soldatik3d
echo 'soldatik3d = { path = "/path/to/soldatik3D" }' >> Cargo.toml
cargo build --release
```

## Windows (MSVC - Recommended)

### Requirements
- Windows 10/11 64-bit
- Visual Studio 2022 Build Tools (or Community/Enterprise)
- Vulkan SDK 1.3+
- Git for Windows

### Install Visual Studio Build Tools

1. Download [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
2. Run installer, select:
   - **Desktop development with C++**
   - **Windows 10/11 SDK** (latest)
   - **C++ CMake tools for Windows**
3. Install

### Install Vulkan SDK

1. Download from [vulkan.lunarg.com](https://vulkan.lunarg.com/sdk/home)
2. Run installer, choose default location
3. Restart terminal for environment variables

### Verify in Developer Command Prompt

```cmd
:: Open "x64 Native Tools Command Prompt for VS 2022"

:: Check tools
cl.exe
cmake --version
vulkaninfo --summary

:: Test compile
cargo new test_soldatik3d && cd test_soldatik3d
echo soldatik3d = { path = "C:\path\to\soldatik3D" } >> Cargo.toml
cargo build --release
```

## Windows (MinGW - Cross-compile from Linux)

### On Linux Host

```bash
# Install MinGW-w64
sudo apt-get install -y mingw-w64

# Add target
rustup target add x86_64-pc-windows-gnu

# Verify
x86_64-w64-mingw32-gcc --version
```

### Limitations
- **OpenGL only** (Vulkan requires `shaderc` C++ build which is complex with MinGW)
- Some Windows-specific features may not work
- Use MSVC for production Windows builds

## Android

### Requirements
- Linux host (for building)
- Android NDK r26c+ (r27+ recommended)
- Android SDK (for APK building)
- Device/emulator with API 29+ (Android 10+)
- OpenGL ES 3.0+ GPU

### Install NDK

```bash
# Option 1: Android Studio (recommended)
# 1. Install Android Studio
# 2. SDK Manager → NDK → Install r26c+

# Option 2: Command line
cd ~
wget https://dl.google.com/android/repository/android-ndk-r26c-linux.zip
unzip android-ndk-r26c-linux.zip
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
echo 'export ANDROID_NDK_HOME=$HOME/android-ndk-r26c' >> ~/.bashrc

# Option 3: rustup (minimal)
rustup target add aarch64-linux-android armv7-linux-androideabi
# Note: Still need NDK for linker
```

### Install Android SDK (for APK)

```bash
# Via Android Studio (easiest)
# 1. Install Android Studio
# 2. SDK Manager → Android SDK → Install API 34, Build Tools 34.0.0
# 3. Set ANDROID_SDK_ROOT or ANDROID_HOME

# Or command line
cd ~
wget https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
unzip commandlinetools-linux-*.zip -d android-sdk
export ANDROID_SDK_ROOT=$HOME/android-sdk
$HOME/android-sdk/cmdline-tools/bin/sdkmanager --licenses
$HOME/android-sdk/cmdline-tools/bin/sdkmanager "platforms;android-34" "build-tools;34.0.0" "platform-tools"
```

### Add Rust Targets

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

### Verify Setup

```bash
# Check NDK
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang

# Check SDK
$ANDROID_SDK_ROOT/platform-tools/adb version

# Test compile (from soldatik3D root)
cd /path/to/soldatik3D
ANDROID_NDK_HOME=$ANDROID_NDK_HOME cargo build --target aarch64-linux-android -p demo
```

## macOS (Not Officially Supported)

> ⚠️ **macOS is not a supported platform** for Soldatik3D.
> The engine targets Linux, Windows, and Android only.
> macOS/iOS support would require Metal backend and significant platform work.

If you need macOS support, consider:
- Running Linux in VM (UTM, Parallels, VMware)
- Using GitHub Actions for macOS builds
- Contributing a Metal backend to the engine

## CI/CD Environments

### GitHub Actions (Reference)

```yaml
# Linux
runs-on: ubuntu-latest
steps:
  - uses: dtolnay/rust-toolchain@stable
    with:
      targets: x86_64-pc-windows-gnu aarch64-linux-android armv7-linux-androideabi
  - name: Install deps
    run: |
      sudo apt-get update
      sudo apt-get install -y mingw-w64 libvulkan-dev libshaderc-dev ...
  - name: Cache
    uses: Swatinem/rust-cache@v2

# Windows
runs-on: windows-latest
steps:
  - uses: dtolnay/rust-toolchain@stable
  - name: Build
    run: cargo build --release

# Android
runs-on: ubuntu-latest
steps:
  - uses: nttld/setup-ndk@v1
    with: { ndk-version: r26c }
  - uses: dtolnay/rust-toolchain@stable
    with: { targets: aarch64-linux-android armv7-linux-androideabi }
```

## Troubleshooting

### `libshaderc-dev` Missing Symbols (Ubuntu 24.04)

```bash
# The system libshaderc_combined.a is incomplete
# Soldatik3D CI works around this automatically
# For local builds:
mkdir -p /tmp/shaderc-lib
ln -sf /usr/lib/x86_64-linux-gnu/libshaderc.so.1 /tmp/shaderc-lib/libshaderc_shared.so
export SHADERC_LIB_DIR=/tmp/shaderc-lib
cargo build
```

### MinGW Linker Errors

```bash
# Use -fuse-ld=lld
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-Clink-arg=-fuse-ld=lld"
```

### Android NDK Not Found

```bash
export ANDROID_NDK_HOME=/path/to/ndk
# Verify
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang
```

### Vulkan Validation Layers

```bash
# Linux
sudo apt-get install vulkan-validationlayers-dev
# Or download from Vulkan SDK

# Enable
SOLDATIK3D_VALIDATION=1 cargo run
```

## Next Steps

- [Quick Start](quick-start.md) - Create your first project
- [Cross-Compilation](guides/cross-compilation.md) - Build for all platforms
- [Android Guide](guides/android.md) - Detailed Android setup