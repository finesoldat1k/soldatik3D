# Configuration Reference

Complete reference for all configuration options in Soldatik3D.

## GameConfig

```rust
pub struct GameConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub backend: BackendKind,
    pub window_hints: WindowHints,
    pub max_frames: Option<u64>,
    pub startup_commands: Vec<String>,
    pub open_console: bool,
}
```

### Builder Methods

```rust
GameConfig::default()
    .with_title("My Game")
    .with_window_size(1920, 1080)
    .with_fullscreen(false)
    .with_vsync(true)
    .with_backend(BackendKind::Vulkan)
    .with_window_hints(WindowHints { ... })
    .with_max_frames(1000)
    .with_startup_commands(vec!["vsync.off".into()])
    .with_open_console(true)
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String` | "Soldatik3D Game" | Window title |
| `width` | `u32` | 1280 | Window width in pixels |
| `height` | `u32` | 720 | Window height in pixels |
| `fullscreen` | `bool` | `false` | Start in fullscreen |
| `vsync` | `bool` | `true` | Enable VSync |
| `backend` | `BackendKind` | `Vulkan` | Graphics backend |
| `window_hints` | `WindowHints` | `default()` | Window behavior |
| `max_frames` | `Option<u64>` | `None` | Auto-exit after N frames |
| `startup_commands` | `Vec<String>` | `[]` | Commands to run at startup |
| `open_console` | `bool` | `false` | Open console at startup |

---

## WindowHints

```rust
pub struct WindowHints {
    pub resizable: bool,
    pub vsync: VsyncMode,
    pub transparent: bool,
    pub decorations: bool,
    pub maximized: bool,
    pub visible: bool,
    pub cursor_visible: bool,
    pub cursor_locked: bool,
    pub cursor_grabbed: bool,
    pub min_size: Option<(u32, u32)>,
    pub max_size: Option<(u32, u32)>,
}
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `resizable` | `bool` | `true` | User can resize window |
| `vsync` | `VsyncMode` | `On` | VSync behavior |
| `transparent` | `bool` | `false` | Transparent window bg |
| `decorations` | `bool` | `true` | Title bar, borders |
| `maximized` | `bool` | `false` | Start maximized |
| `visible` | `bool` | `true` | Initially visible |
| `cursor_visible` | `bool` | `true` | Show mouse cursor |
| `cursor_locked` | `bool` | `false` | Lock to center (FPS) |
| `cursor_grabbed` | `bool` | `false` | Confine to window |
| `min_size` | `Option<(u32,u32)>` | `None` | Minimum size |
| `max_size` | `Option<(u32,u32)>` | `None` | Maximum size |

### VsyncMode

```rust
pub enum VsyncMode {
    On,         // Wait for vblank
    Off,        // Immediate present
    Adaptive,   // Sync when > refresh rate
    Mailbox,    // Triple buffer, latest frame
}
```

---

## BackendKind

```rust
#[non_exhaustive]
pub enum BackendKind {
    Vulkan,
    OpenGl,
}

impl BackendKind {
    pub fn from_name(name: &str) -> Option<Self>;
    pub fn from_env() -> Self;
}
```

### Selection Priority

1. `GameConfig::backend` (explicit)
2. `SOLDATIK3D_BACKEND` env var
3. Platform default (Vulkan on desktop, OpenGL ES on Android)

---

## Environment Variables

### Runtime Configuration

| Variable | Values | Default | Description |
|----------|--------|---------|-------------|
| `SOLDATIK3D_BACKEND` | `vulkan`, `opengl`, `vk`, `gl` | Auto | Force graphics backend |
| `SOLDATIK3D_VSYNC` | `on`, `off`, `adaptive`, `mailbox` | `on` | VSync mode |
| `SOLDATIK3D_MAX_FRAMES` | `<u64>` | None | Auto-exit after N frames |
| `SOLDATIK3D_COMMANDS` | `"cmd1;cmd2"` | `""` | Startup commands |
| `SOLDATIK3D_CONSOLE` | `1`, `0` | `0` | Open console at startup |
| `SOLDATIK3D_DEBUG_VIEW` | `normals`, `depth`, `wireframe` | None | Initial debug view |
| `SOLDATIK3D_VALIDATION` | `1`, `0` | `0` | Vulkan validation layers |

### Build/Link Configuration

| Variable | Values | Description |
|----------|--------|-------------|
| `ANDROID_NDK_HOME` | `<path>` | Android NDK path |
| `ANDROID_SDK_ROOT` | `<path>` | Android SDK path |
| `ANDROID_HOME` | `<path>` | Android SDK path (legacy) |
| `SHADERC_LIB_DIR` | `<path>` | System shaderc library |
| `VULKAN_SDK` | `<path>` | Vulkan SDK path |
| `VK_ICD_FILENAMES` | `<path>` | Vulkan ICD override |
| `VK_LAYER_PATH` | `<path>` | Vulkan layer path |
| `CARGO_TARGET_<TRIPLE>_LINKER` | `<path>` | Cross-compile linker |
| `CC_<TRIPLE>` | `<path>` | C compiler for target |
| `AR_<TRIPLE>` | `<path>` | Archiver for target |
| `CMAKE_POLICY_VERSION_MINIMUM` | `3.5` | CMake compatibility |

---

## Cargo.toml Configuration

### Package Metadata

```toml
[package]
name = "my_game"
version = "0.1.0"
edition = "2024"
license = "GPL-3.0"
repository = "https://github.com/user/my_game"
description = "My awesome game"
publish = false

# Android requires cdylib
[lib]
crate-type = ["cdylib", "rlib"]
```

### Features

```toml
[features]
default = ["vulkan", "opengl"]
vulkan = ["soldatik3d/vulkan"]
opengl = ["soldatik3d/opengl"]

# Optional game features
# audio = ["rodio"]
# networking = ["tokio", "bevy_net"]
# scripting = ["mlua"]
```

### Dependencies

```toml
[dependencies]
soldatik3d = { path = "../../../.." }  # Local engine
# soldatik3d = "0.1"  # From crates.io
anyhow = "1.0"

# Platform-specific
[target.'cfg(target_os = "android")'.dependencies]
android-activity = { version = "0.6", features = ["api-level-29"] }
jni = "0.21"
```

### Profile Configuration

```toml
[profile.dev]
opt-level = 3
debug = true
split-debuginfo = "unpacked"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "debuginfo"
debug = false
panic = "abort"

# Per-package overrides
[profile.dev.package."*"]
opt-level = 3

[profile.release.package."shaderc-sys"]
opt-level = 3
```

---

## .cargo/config.toml

### Linux

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-unknown-linux-musl]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld", "-C", "target-feature=+crt-static"]
```

### Windows (MinGW Cross-compile)

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
runner = "wine64"
rustflags = [
  "-C", "link-arg=-fuse-ld=lld",
  "-C", "link-arg=-static",
]
```

### Windows (MSVC)

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

### Android

```toml
# Set via build.rs / environment variables
# [target.aarch64-linux-android]
# linker = "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
# ar = "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
```

### Global Settings

```toml
[net]
git-fetch-with-cli = true

[profile.dev]
opt-level = 3
debug = true
split-debuginfo = "unpacked"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "debuginfo"
debug = false
```

---

## build.rs Configuration

### Standard Template

```rust
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");
    
    let target = std::env::var("TARGET").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();
    
    // Android NDK setup
    if target.contains("android") {
        setup_android_ndk(&target);
    }
    
    // Copy assets
    copy_assets(&target, &profile);
}

fn setup_android_ndk(target: &str) {
    if std::env::var("ANDROID_NDK_HOME").is_err() {
        println!("cargo:warning=ANDROID_NDK_HOME not set");
        return;
    }
    
    let ndk = std::env::var("ANDROID_NDK_HOME").unwrap();
    let api = "29";
    let ndk_bin = format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin", ndk);
    
    let (linker, cc, ar) = match target {
        "aarch64-linux-android" => (
            format!("{}/aarch64-linux-android{}-clang", ndk_bin, api),
            format!("{}/aarch64-linux-android{}-clang", ndk_bin, api),
            format!("{}/llvm-ar", ndk_bin),
        ),
        "armv7-linux-androideabi" => (
            format!("{}/armv7a-linux-androideabi{}-clang", ndk_bin, api),
            format!("{}/armv7a-linux-androideabi{}-clang", ndk_bin, api),
            format!("{}/llvm-ar", ndk_bin),
        ),
        "x86_64-linux-android" => (
            format!("{}/x86_64-linux-android{}-clang", ndk_bin, api),
            format!("{}/x86_64-linux-android{}-clang", ndk_bin, api),
            format!("{}/llvm-ar", ndk_bin),
        ),
        _ => return,
    };
    
    let target_upper = target.replace('-', "_").to_uppercase();
    println!("cargo:rustc-env=CARGO_TARGET_{}_LINKER={}", target_upper, linker);
    println!("cargo:rustc-env=CC_{}={}", target_upper, cc);
    println!("cargo:rustc-env=AR_{}={}", target_upper, ar);
}

fn copy_assets(target: &str, profile: &str) {
    let assets_src = std::path::Path::new("assets");
    if !assets_src.exists() { return; }
    
    let assets_dst = if target.contains("android") {
        std::path::PathBuf::from("target").join(target).join(profile).join("assets")
    } else {
        std::path::PathBuf::from("target").join(profile).join("assets")
    };
    
    if let Err(e) = copy_dir_all(assets_src, &assets_dst) {
        println!("cargo:warning=Failed to copy assets: {}", e);
    }
}
```

---

## Gradle Configuration (Android)

### app/build.gradle.kts

```kotlin
plugins {
    id("com.android.application") version "8.4.0"
    id("org.jetbrains.kotlin.android") version "1.9.22"
}

android {
    namespace = "com.yourgame.game"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.yourgame.game"
        minSdk = 29
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"
        
        ndk {
            abiFilters.add("arm64-v8a")
            abiFilters.add("armeabi-v7a")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    sourceSets {
        main {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packagingOptions {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
    implementation("androidx.activity:activity-ktx:1.8.2")
}
```

### gradle.properties

```properties
org.gradle.jvmargs=-Xmx4g -Dfile.encoding=UTF-8
android.useAndroidX=true
android.enableJetifier=true
kotlin.code.style=official
```

---

## GitHub Actions Configuration

### CI Workflow (ci.yml)

```yaml
env:
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: "0"
  RUST_BACKTRACE: "1"
  CMAKE_POLICY_VERSION_MINIMUM: "3.5"

jobs:
  linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install deps
        run: |
          sudo apt-get update
          sudo apt-get install -y libvulkan-dev libshaderc-dev ...
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --locked --all-features

  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --locked --all-features

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
        with: { ndk-version: r26c }
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: ${{ matrix.target }} }
      - name: Configure NDK
        run: |
          NDK_BIN=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin
          echo "CARGO_TARGET_${{ matrix.target | upper | replace('-', '_') }}_LINKER=$NDK_BIN/..." >> $GITHUB_ENV
      - run: cargo build --locked --no-default-features --features opengl --target ${{ matrix.target }}
```

---

## Shader Compilation Configuration

### Vulkan (Runtime)

```rust
// Automatic via shaderc
// GLSL -> SPIR-V at startup
// Configure via ShaderOptions:
let mut options = shaderc::CompileOptions::new()?;
options.set_target_env(shaderc::TargetEnv::Vulkan, 0);
options.set_optimization_level(shaderc::OptimizationLevel::Performance);
options.set_target_spirv(shaderc::SpirvVersion::V1_3);
```

### OpenGL (Driver)

```rust
// Compiled by driver at runtime
// Desktop: #version 330 core
// Android: #version 300 es (auto-translated)
```

### Shaderc System Library

```bash
# Linux (Ubuntu/Debian)
sudo apt-get install libshaderc-dev

# Use system library
mkdir -p /tmp/shaderc-lib
ln -sf /usr/lib/x86_64-linux-gnu/libshaderc.so.1 /tmp/shaderc-lib/libshaderc_shared.so
export SHADERC_LIB_DIR=/tmp/shaderc-lib

# Or build from source (slower)
# cargo build --features build-from-source
```

---

*See [CLI Tool](cli.md) for command-line configuration, [Platform Guides](guides/) for platform-specific setup, or [Getting Started](getting-started/installation.md) for initial setup.*