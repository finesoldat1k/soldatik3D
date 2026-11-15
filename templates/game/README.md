# MyGame

A game project built with the [Soldatik3D](https://github.com/finesoldat1k/soldatik3D)
engine.

## Requirements

- Rust stable (edition 2024)
- Linux: Vulkan or OpenGL drivers plus the usual X11/Wayland development packages
- Windows: MSVC build tools (Visual Studio Build Tools) and a Vulkan/OpenGL driver
- Android: NDK r26c+, Android SDK platform 34, build-tools 34.0.0 and JDK 17+

The engine is used through a local path in `Cargo.toml`. `s3d new` points that
path at the engine checkout that created the project. If you move the project
or the engine, update the `runtime` dependency path.

## Run on desktop

```bash
cargo run
```

Choose a backend with an environment variable:

```bash
SOLDATIK3D_BACKEND=vulkan cargo run
SOLDATIK3D_BACKEND=opengl cargo run
```

Useful variables:

| Variable | Values | Purpose |
|----------|--------|---------|
| `SOLDATIK3D_BACKEND` | `vulkan`, `opengl` | Force a graphics backend |
| `SOLDATIK3D_VSYNC` | `on`, `off`, `adaptive`, `mailbox` | Override VSync |
| `SOLDATIK3D_MAX_FRAMES` | number | Exit after N frames (benchmarks/smoke tests) |
| `SOLDATIK3D_CONSOLE` | `1` | Open the in-game console at startup |
| `SOLDATIK3D_COMMANDS` | `"cmd1;cmd2"` | Run console commands at startup |
| `SOLDATIK3D_DEBUG_VIEW` | `lit`, `normals`, `depth`, `wireframe` | Initial debug view |
| `SOLDATIK3D_VALIDATION` | `1` | Enable Vulkan validation layers |

## Build and export

When `s3d` is installed (or built from the engine checkout):

```bash
s3d build linux --release
s3d build windows --release   # uses MinGW and the OpenGL backend
s3d export --out-dir dist     # Linux + Windows + Android libraries
```

Without `s3d`, use Cargo directly:

```bash
cargo build --release
# Linux binary: target/release/my_game
```

## Android APK

Install the Rust targets and point the build at your NDK:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
export ANDROID_SDK_ROOT=$HOME/Android/Sdk

./build_apk.sh arm64-v8a
./build_apk.sh armeabi-v7a --release
```

The APK is written to `apk/app-<abi>-<debug|release>.apk`. Install it with:

```bash
adb install -r apk/app-arm64-v8a-debug.apk
```

Or let `s3d` invoke the script:

```bash
s3d apk arm64-v8a --release
```

The Android build uses OpenGL ES 3.0 and the `opengl` feature. The native
library is named after the Cargo package (`my_game` → `libmy_game.so`); the
Gradle manifest placeholder and `build_apk.sh` derive it automatically.

Release APKs are debug-signed by default so they are installable immediately.
To use your own key, export:

```bash
export SOLDATIK3D_KEYSTORE=/absolute/path/to/release.keystore
export SOLDATIK3D_KEYSTORE_PASSWORD=...
export SOLDATIK3D_KEY_ALIAS=...
export SOLDATIK3D_KEY_PASSWORD=...
```

## Project layout

```text
my_game/
├── android/gradle/       # Self-contained Android Gradle project
├── assets/               # Game assets (copied into the APK on Android)
├── shaders/              # Custom shaders
├── src/
│   ├── lib.rs            # Game implementation and Android entry point
│   └── main.rs           # Desktop entry point
├── build.rs              # Android asset packaging
├── build_apk.sh          # Local APK builder
└── Cargo.toml
```

## Game code

`src/lib.rs` implements `runtime::Game`:

```rust
impl Game for MyGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("MyGame")
            .with_size(1280, 720)
            .with_vsync(VsyncMode::On)
    }

    fn create_world(&mut self) -> World {
        // Build a Scene and a CameraRig, then return World::with_cameras(...)
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // ctx.dt, ctx.input, ctx.world, ctx.stats, ...
    }
}
```

The desktop binary calls `run_desktop()`; Android loads `android_main` from
`libmy_game.so`.

See the engine documentation in `docs/` for the scene, renderer, input,
console and asset APIs.

## License

GPL-3.0. See the engine repository for the full license text.
