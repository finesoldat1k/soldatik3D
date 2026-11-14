# Cross-Compilation Guide

Build Soldatik3D games for all platforms from a single Linux machine.

## Overview

| Host → Target | Method | Backend | Difficulty |
|---------------|--------|---------|------------|
| Linux → Linux | Native | Vulkan + OpenGL | Easy |
| Linux → Windows | MinGW-w64 | OpenGL only | Medium |
| Linux → Android | NDK | OpenGL ES | Medium |
| Windows → Linux | WSL2/Cross | Vulkan + OpenGL | Hard |
| Windows → Android | NDK | OpenGL ES | Medium |
| macOS → Any | Not supported | - | - |

## Linux Host (Recommended)

### Prerequisites

```bash
# Install cross-compilation toolchains
sudo apt-get update
sudo apt-get install -y \
    mingw-w64 \           # Windows MinGW
    gcc-aarch64-linux-gnu \ # Linux ARM64 (optional)
    gcc-arm-linux-gnueabihf # Linux ARMv7 (optional)

# Android NDK
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c

# Add Rust targets
rustup target add \
    x86_64-pc-windows-gnu \
    aarch64-linux-android \
    armv7-linux-androideabi \
    x86_64-linux-android
```

### Using cross_build.sh

```bash
# From Soldatik3D engine root
./cross_build.sh linux --release      # Linux x86_64
./cross_build.sh windows --release    # Windows MinGW (OpenGL only)
./cross_build.sh android arm64-v8a --release
./cross_build.sh android armeabi-v7a --release
./cross_build.sh all --release        # All platforms
```

Output in `dist/`:
```
dist/
├── linux-x86_64/
│   ├── your_game
│   └── libyour_game.rlib
├── windows-x86_64/
│   ├── your_game.exe
│   └── libyour_game.rlib
├── android-arm64-v8a/
│   └── aarch64-linux-android/
│       └── libyour_game.so
└── android-armeabi-v7a/
    └── armv7-linux-androideabi/
        └── libyour_game.so
```

### Using CLI Tool

```bash
# Build for specific target
soldatik3d build --target windows --release
soldatik3d build --target android --release

# Export all platforms
soldatik3d export --platforms linux,windows,android --out-dir dist
```

## Windows Target (MinGW)

### Toolchain Setup

```bash
# Ubuntu/Debian
sudo apt-get install mingw-w64

# Fedora
sudo dnf install mingw64-gcc mingw64-gcc-c++ mingw64-winpthreads

# Arch
sudo pacman -S mingw-w64-gcc mingw-w64-binutils mingw-w64-winpthreads
```

### Cargo Configuration

```toml
# .cargo/config.toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
runner = "wine64"
rustflags = [
  "-C", "link-arg=-fuse-ld=lld",
  "-C", "link-arg=-static",
]
```

### Build Command

```bash
cargo build --target x86_64-pc-windows-gnu --release

# Binary: target/x86_64-pc-windows-gnu/release/your_game.exe
```

### Limitations

| Feature | Support |
|---------|---------|
| Vulkan | ❌ (shaderc C++ build fails) |
| OpenGL | ✅ Full support |
| Dynamic linking | ⚠️ Static preferred |
| Wine testing | ✅ `wine64 your_game.exe` |

### Testing with Wine

```bash
# Install Wine
sudo apt-get install wine64

# Run
wine64 target/x86_64-pc-windows-gnu/release/your_game.exe

# With OpenGL backend
SOLDATIK3D_BACKEND=opengl wine64 target/.../your_game.exe
```

## Android Target

### NDK Setup

```bash
# Download NDK r26c+
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c

# Verify
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang
```

### Rust Targets

```bash
rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    x86_64-linux-android
```

### Linker Environment

The build script sets these automatically:

```bash
# For aarch64-linux-android
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$NDK_BIN/aarch64-linux-android29-clang
export CC_aarch64_linux_android=$NDK_BIN/aarch64-linux-android29-clang
export AR_aarch64_linux_android=$NDK_BIN/llvm-ar

# For armv7-linux-androideabi
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=$NDK_BIN/armv7a-linux-androideabi29-clang
export CC_armv7_linux_androideabi=$NDK_BIN/armv7a-linux-androideabi29-clang
export AR_armv7_linux_androideabi=$NDK_BIN/llvm-ar
```

### Build Commands

```bash
# ARM64 (modern devices)
cargo build --target aarch64-linux-android --release \
  --no-default-features --features opengl

# ARMv7 (older devices)
cargo build --target armv7-linux-androideabi --release \
  --no-default-features --features opengl

# x86_64 (emulators)
cargo build --target x86_64-linux-android --release \
  --no-default-features --features opengl
```

### APK Building

```bash
# Requires Android SDK + Gradle
./build_apk.sh arm64-v8a --release
./build_apk.sh armeabi-v7a --release
```

## CI/CD Pipeline

### GitHub Actions Matrix

```yaml
# .github/workflows/cross-compile.yml
name: Cross-Platform Build

on: [push, pull_request]

jobs:
  linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --locked --release
      - uses: actions/upload-artifact@v4
        with:
          name: linux-x86_64
          path: target/release/*

  windows:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install MinGW
        run: sudo apt-get install -y mingw-w64
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-gnu
      - uses: Swatinem/rust-cache@v2
        with:
          key: windows-gnu
      - run: cargo build --locked --release --target x86_64-pc-windows-gnu
      - uses: actions/upload-artifact@v4
        with:
          name: windows-x86_64
          path: target/x86_64-pc-windows-gnu/release/*

  android:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
          - target: aarch64-linux-android
            abi: arm64-v8a
          - target: armv7-linux-androideabi
            abi: armeabi-v7a
    steps:
      - uses: actions/checkout@v4
      - uses: nttld/setup-ndk@v1
        with:
          ndk-version: r26c
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: android-${{ matrix.abi }}
      - name: Configure NDK linker
        run: |
          NDK_BIN=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin
          echo "CARGO_TARGET_${{ matrix.target | upper | replace('-', '_') }}_LINKER=$NDK_BIN/${{ matrix.abi == 'arm64-v8a' && 'aarch64-linux-android29-clang' || 'armv7a-linux-androideabi29-clang' }}" >> $GITHUB_ENV
      - run: cargo build --locked --release --no-default-features --features opengl --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: android-${{ matrix.abi }}
          path: target/${{ matrix.target }}/release/lib*.so
```

## Docker for Reproducible Builds

### Dockerfile

```dockerfile
# Dockerfile.cross
FROM ubuntu:24.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl git build-essential pkg-config cmake ninja-build \
    libvulkan-dev libshaderc-dev \
    libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
    libxkbcommon-dev libwayland-dev \
    libgl1-mesa-dev libegl1-mesa-dev libgles2-mesa-dev \
    mingw-w64 \
    clang lld \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup default stable && \
    rustup target add x86_64-pc-windows-gnu aarch64-linux-android armv7-linux-androideabi

# Install Android NDK
ARG NDK_VERSION=r26c
RUN cd /opt && \
    wget -q https://dl.google.com/android/repository/android-ndk-${NDK_VERSION}-linux.zip && \
    unzip -q android-ndk-${NDK_VERSION}-linux.zip && \
    rm android-ndk-${NDK_VERSION}-linux.zip
ENV ANDROID_NDK_HOME=/opt/android-ndk-${NDK_VERSION}

WORKDIR /workspace
```

### Build with Docker

```bash
# Build image
docker build -f Dockerfile.cross -t soldatik3d-cross .

# Run build
docker run --rm -v $(pwd):/workspace soldatik3d-cross \
  bash -c "cd /workspace && ./cross_build.sh all --release"

# Artifacts in ./dist/
```

## Troubleshooting

### MinGW: `error: linking with 'cc' failed`

```bash
# Ensure static linking
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-Clink-arg=-static -Clink-arg=-fuse-ld=lld"

# Or use mold linker (faster)
sudo apt-get install mold
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-Clink-arg=-fuse-ld=mold"
```

### Android: `failed to find tool "aarch64-linux-android-clang"`

```bash
# NDK not in PATH or wrong version
export ANDROID_NDK_HOME=/correct/path/to/ndk
# Verify:
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang
```

### Windows: `LINK : fatal error LNK1181`

```bash
# Missing Vulkan library for MinGW
# OpenGL only for MinGW - ensure features = ["opengl"]
cargo build --target x86_64-pc-windows-gnu --no-default-features --features opengl
```

### Cross-compile standard library

```bash
# If you need custom std
rustup component add rust-src --toolchain stable
cargo build -Zbuild-std=std,panic_abort --target x86_64-pc-windows-gnu
# Requires nightly
```

## Best Practices

1. **Use `cross_build.sh`** - Handles all environment setup
2. **Cache cargo registry** - Speeds up CI significantly
3. **Test on target** - Wine for Windows, emulator/device for Android
4. **Static linking for Windows** - Avoids DLL hell
5. **Release profile optimization** - `lto = "thin"`, `codegen-units = 1`
6. **Separate cargo targets** - Different `target/` dirs per platform

---

*See platform-specific guides: [Linux](linux.md), [Windows](windows.md), [Android](android.md)*