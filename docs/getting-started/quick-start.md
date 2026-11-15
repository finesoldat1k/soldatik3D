# Quick Start

## 1. Build and run the demo

```bash
git clone https://github.com/finesoldat1k/soldatik3D
cd soldatik3D

cargo run -p demo
```

The demo opens a 1280×720 window and uses Vulkan by default on desktop. If
Vulkan is not available, select OpenGL:

```bash
SOLDATIK3D_BACKEND=opengl cargo run -p demo
```

Useful smoke-test mode (renders two frames and exits):

```bash
SOLDATIK3D_BACKEND=opengl SOLDATIK3D_MAX_FRAMES=2 cargo run -p demo
```

Demo controls are listed in the [README](../../README.md#demo-controls).

## 2. Build the s3d CLI

```bash
cargo build --release -p s3d
./target/release/s3d --help
```

`s3d` is a host tool. It is not needed to build the engine itself, but it
creates and packages game projects. The examples below assume
`target/release` is on `PATH`; otherwise prefix commands with
`./target/release/` or run `cargo install --path crates/s3d`.

## 3. Create a game project

```bash
./target/release/s3d new my_game
cd my_game
cargo run
```

`s3d new` copies `templates/game`, renames the Rust package, points the
`runtime` dependency at the engine checkout that created the project, and makes
an initial git commit.

Generated project layout:

```text
my_game/
├── android/gradle/     # NativeActivity Gradle project
├── assets/             # Assets copied into the APK by build.rs
├── shaders/            # Your shaders
├── src/
│   ├── lib.rs          # Game implementation + Android entry point
│   └── main.rs         # Desktop entry point
├── build.rs            # Android asset packaging
├── build_apk.sh        # Local APK builder
├── Cargo.toml
└── README.md
```

## 4. Build and run

```bash
# Debug desktop build
cargo run

# Release desktop build
s3d build linux --release

# Windows cross build (MinGW, OpenGL only)
s3d build windows --release

# Android APK
s3d apk arm64-v8a --release
```

`s3d build` and `s3d apk` must run from the generated project root.

## 5. Choose a graphics backend

```bash
SOLDATIK3D_BACKEND=vulkan cargo run
SOLDATIK3D_BACKEND=opengl cargo run
```

Or in code:

```rust
use runtime::{BackendKind, GameConfig};

let config = GameConfig::default().with_backend(BackendKind::OpenGl);
```

Desktop builds enable both backends by default. Android builds enable OpenGL ES
only; the Vulkan dependency is target-gated out.

## 6. Build an APK

```bash
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
export ANDROID_SDK_ROOT=$HOME/Android/Sdk

./build_apk.sh arm64-v8a
./build_apk.sh arm64-v8a --release
```

The APK is written to `apk/app-<abi>-<debug|release>.apk`. Install it with:

```bash
adb install -r apk/app-arm64-v8a-debug.apk
```

Release APKs are debug-signed unless `SOLDATIK3D_KEYSTORE*` variables are set.
See the [Android guide](../guides/android.md) for details.

## 7. Run the checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo doc --workspace --no-deps --all-features
```

## Next steps

- [Your first game](first-game.md) — write a complete game from scratch.
- [Game loop](../guides/game-loop.md) — understand `Game` and `UpdateContext`.
- [Scene API](../api/scene.md) — meshes, cameras, lights and materials.
- [Android guide](../guides/android.md) — APK packaging and signing.
