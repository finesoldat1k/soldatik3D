# Installation

## Supported platforms

| Platform | Target triple | Graphics backend |
|----------|---------------|------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Vulkan 1.2, OpenGL 3.3 |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Vulkan 1.2, OpenGL 3.3 |
| Windows x86_64 (cross) | `x86_64-pc-windows-gnu` | OpenGL 3.3 |
| Android arm64 | `aarch64-linux-android` | OpenGL ES 3.0 |
| Android armv7 | `armv7-linux-androideabi` | OpenGL ES 3.0 |
| Android x86_64 (emulator) | `x86_64-linux-android` | OpenGL ES 3.0 |

macOS, iOS and WebAssembly are not supported.

## Rust

Install the stable toolchain with [rustup](https://rustup.rs/). The workspace
uses Rust edition 2024, so use Rust 1.85 or newer.

```bash
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
```

Verify:

```bash
rustc --version
cargo --version
```

## Linux

The engine needs X11/Wayland development headers, OpenGL/EGL, and a Vulkan
loader. The Vulkan backend compiles GLSL at runtime with shaderc; either install
a system shaderc library or let `shaderc-sys` build it from source (requires a
C++17 compiler, CMake and Python 3).

### Debian / Ubuntu

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential pkg-config cmake ninja-build git python3 \
  libvulkan-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
  libxkbcommon-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev \
  libgles2-mesa-dev libshaderc-dev
```

Ubuntu's `libshaderc-dev` package ships an incomplete
`libshaderc_combined.a`; point `shaderc-sys` at the real shared library before
building:

```bash
mkdir -p /tmp/shaderc-lib
ln -sf /usr/lib/x86_64-linux-gnu/libshaderc.so.1 /tmp/shaderc-lib/libshaderc_shared.so
export SHADERC_LIB_DIR=/tmp/shaderc-lib
```

### Arch Linux

```bash
sudo pacman -S --needed base-devel rust cmake ninja python \
  vulkan-icd-loader vulkan-validation-layers shaderc \
  libx11 libxcursor libxi libxrandr libxkbcommon wayland mesa
```

### Fedora

```bash
sudo dnf install gcc-c++ make cmake ninja-build python3 \
  vulkan-loader vulkan-validation-layers shaderc-devel \
  libX11-devel libXcursor-devel libXi-devel libXrandr-devel \
  libxkbcommon-devel wayland-devel mesa-libGL-devel mesa-libEGL-devel
```

A Vulkan driver (Mesa RADV, Intel ANV or the proprietary NVIDIA driver) is
required to run the Vulkan backend. The OpenGL backend works with Mesa or the
vendor GL driver.

## Windows

1. Install **Visual Studio Build Tools** with the *Desktop development with C++*
   workload (MSVC, Windows SDK, CMake).
2. Install Rust with the MSVC toolchain:

```powershell
rustup default stable-msvc
```

3. Install a Vulkan-capable GPU driver. The Vulkan SDK is not required to build;
`shaderc-sys` builds its dependencies through CMake.

To cross-compile Windows binaries from Linux, install MinGW-w64
(`sudo apt-get install mingw-w64` or the equivalent) and the Rust target:

```bash
rustup target add x86_64-pc-windows-gnu
```

MinGW builds use OpenGL only because the Vulkan backend is not built for the
GNU Windows target.

## Android

Requirements:

- JDK 17
- Android SDK platform 34 and build-tools 34.0.0
- Android NDK r26c or newer
- Rust targets for the ABIs you build

Install the Rust targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

Install SDK components with `sdkmanager`:

```bash
export ANDROID_SDK_ROOT=$HOME/Android/Sdk
"$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager" \
  "platforms;android-34" "build-tools;34.0.0" "platform-tools"
```

Install an NDK (r26c shown; newer releases also work):

```bash
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
```

Environment variables used by the build scripts:

| Variable | Purpose |
|----------|---------|
| `JAVA_HOME` | JDK 17 installation |
| `ANDROID_SDK_ROOT` or `ANDROID_HOME` | SDK root (platform 34, build-tools) |
| `ANDROID_NDK_HOME` | NDK root; if unset, the scripts look under `$ANDROID_SDK_ROOT/ndk` |

The Gradle wrapper downloads Gradle 8.10.2 on first use and the Android Gradle
Plugin 8.7.3 from Google Maven.

## Verifying the installation

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo doc --workspace --no-deps --all-features
```

Run the demo with an auto-exit smoke test:

```bash
SOLDATIK3D_BACKEND=opengl SOLDATIK3D_MAX_FRAMES=2 cargo run -p demo
```

If Vulkan is unavailable, the engine reports the backend error. Select OpenGL
explicitly with `SOLDATIK3D_BACKEND=opengl`.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `Could not find or load main class "-Xmx64m"` | The Gradle wrapper script is broken or has CRLF line endings. Use the bundled `gradlew` and run it with `sh ./gradlew`. |
| `Android NDK not found` | Set `ANDROID_NDK_HOME`, or install an NDK under `$ANDROID_SDK_ROOT/ndk`. |
| `SDK location not found` | Set `ANDROID_SDK_ROOT`/`ANDROID_HOME`, or create `local.properties` with `sdk.dir=/path/to/sdk`. |
| Vulkan validation errors on startup | Update the GPU driver, or run with `SOLDATIK3D_BACKEND=opengl`. |
| Linker cannot find `libshaderc` | Install `libshaderc-dev`, set `SHADERC_LIB_DIR` as shown above, or remove `vulkan` from the runtime features. |
| Linux binary fails on another machine | Use `./scripts/cross_build.sh linux --release`; it bundles `libshaderc.so.1` and `libvulkan.so.1` and sets an `$ORIGIN` rpath. |

Next: [Quick start](quick-start.md).
