# Platform-Specific Code

Soldatik3D targets Linux, Windows and Android. Most game code is shared; this
page covers the parts that differ.

## One game, three entry points

A generated project already has the right structure:

```text
src/lib.rs    game implementation + Android entry point
src/main.rs   desktop entry point
```

`src/main.rs`:

```rust
fn main() -> anyhow::Result<()> {
    my_game::run_desktop()
}
```

`src/lib.rs` exports both desktop and Android paths:

```rust
pub fn run_desktop() -> anyhow::Result<()> {
    runtime::run(MyGame::default())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    if let Err(error) = runtime::run_android(MyGame::default(), app) {
        eprintln!("game failed: {error:?}");
    }
}
```

`android_main` must be `#[unsafe(no_mangle)]` and public. Without it the
`NativeActivity` cannot start the game.

## Conditional compilation

Use `cfg` for code that must not exist on other platforms:

```rust
#[cfg(target_os = "linux")]
const PLATFORM: &str = "Linux";

#[cfg(target_os = "windows")]
const PLATFORM: &str = "Windows";

#[cfg(target_os = "android")]
const PLATFORM: &str = "Android";

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "android")))]
const PLATFORM: &str = "Unsupported";
```

For code that compiles everywhere, use `TargetOS` and `TargetArch`:

```rust
use runtime::{TargetArch, TargetOS};

fn startup_message() -> String {
    let os = TargetOS::current();
    let arch = TargetArch::current();
    format!("{} / {} / mobile: {}", os.name(), arch.name(), os.is_mobile())
}
```

| `TargetOS` | `is_desktop` | `is_mobile` | `supports_vulkan` | `default_backend` |
|------------|--------------|-------------|-------------------|-------------------|
| `Linux` | yes | no | yes | Vulkan |
| `Windows` | yes | no | yes | Vulkan |
| `Android` | no | yes | no | OpenGL |
| `Unknown` | no | no | no | Vulkan |

`supports_vulkan` describes the **compiled engine**, not the device. Android
devices may have Vulkan drivers, but the Android build contains only OpenGL ES.

## Platform-specific controls

Desktop controls use relative mouse motion; Android currently receives pointer
events from the OS but touch controls are not implemented. Keep Android input
simple and do not assume a keyboard:

```rust
impl Game for MyGame {
    fn input_map(&self) -> InputMap {
        InputMap::new()
            .bind("forward", &[KeyCode::KeyW, KeyCode::ArrowUp])
            .bind("jump", &[KeyCode::Space])
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        #[cfg(not(target_os = "android"))]
        {
            let delta = ctx.input.mouse_delta();
            if delta != Vec2::ZERO
                && let Some(camera) = ctx.world.active_camera_mut()
            {
                camera.camera.yaw += delta.x * 0.0025;
                camera.camera.pitch =
                    (camera.camera.pitch - delta.y * 0.0025).clamp(-1.5, 1.5);
            }
        }

        if ctx.input.action_down("forward") {
            // shared movement code
        }
    }
}
```

Cursor capture is requested by `GameConfig::cursor_locked`. On Android there is
no cursor, so the engine logs that capture is unavailable and continues.

## Window and viewport

Android starts fullscreen; desktop opens a resizable window. `ctx.viewport`
contains the physical framebuffer size, which is the correct value for UI:

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let width = ctx.viewport.0 as f32;
    let height = ctx.viewport.1 as f32;
    ctx.ui.text(
        &format!("{}x{}", ctx.viewport.0, ctx.viewport.1),
        width - 120.0,
        height - 28.0,
        1.0,
        [1.0; 4],
    );
}
```

Do not hardcode 1280×720 in UI layout. Use `ctx.viewport` or `ctx.ui.width()` /
`ctx.ui.height()`.

## Backend selection

```rust
use runtime::{BackendKind, GameConfig};

let config = if runtime::TargetOS::current().is_mobile() {
    GameConfig::default().with_backend(BackendKind::OpenGl)
} else {
    GameConfig::default().with_backend(BackendKind::Vulkan)
};
```

On Android `BackendKind::Vulkan` is compiled out, so requesting it does not
work. The runtime already defaults to OpenGL there. `SOLDATIK3D_BACKEND` can
still force OpenGL on desktop:

```bash
SOLDATIK3D_BACKEND=opengl cargo run
```

## Assets

Desktop builds read files from the working directory. Android packages `assets/`
inside the APK through `build.rs`, but the template does not expose an asset
reader. If your game loads files:

```rust
#[cfg(not(target_os = "android"))]
fn load_level() -> std::io::Result<String> {
    std::fs::read_to_string("assets/level.txt")
}
```

On Android, add `android-activity` asset APIs or embed the data with
`include_str!` / `include_bytes!`. Embedding is the simplest portable option for
small files and works on all three platforms.

## Linux

```bash
cargo build -p demo
SOLDATIK3D_BACKEND=opengl cargo run -p demo
./scripts/cross_build.sh linux --release
```

The release package contains `demo` plus `libshaderc.so.1` and
`libvulkan.so.1`. The binary uses an `$ORIGIN` rpath, so keep the files
together.

## Windows

Native MSVC:

```powershell
cargo build -p demo
cargo run -p demo
```

Cross build from Linux:

```bash
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu \
  --no-default-features --features opengl
```

Windows GNU builds are OpenGL-only. A native MSVC build supports both backends.

From a game project:

```bash
s3d build windows --release
s3d run windows --release   # requires Wine on Linux
```

## Android

```bash
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
export ANDROID_SDK_ROOT=$HOME/Android/Sdk

./build_apk.sh arm64-v8a
adb install -r apk/app-arm64-v8a-debug.apk
adb shell am start -n com.soldatik3d.my_game/android.app.NativeActivity
```

The application ID is set in `build_apk.sh`:

```bash
export SOLDATIK3D_ANDROID_APP_ID="com.soldatik3d.$LIB_NAME"
export SOLDATIK3D_ANDROID_APP_NAME="$GAME_NAME"
```

Change those exports to customize the package and label. The library name must
match the Cargo `[lib] name`.

For log output:

```bash
adb logcat -s Soldatik3D RustStdoutStderr AndroidRuntime
```

## Platform-specific dependencies

Put Android-only dependencies in a target table:

```toml
[target.'cfg(target_os = "android")'.dependencies]
android-activity = { version = "0.6", features = ["native-activity"] }
jni = "0.21"
```

The template already includes them. Desktop builds ignore the table.

## Feature flags

```toml
[features]
default = ["vulkan", "opengl"]
vulkan = ["runtime/vulkan"]
opengl = ["runtime/opengl"]
```

Android scripts pass `--no-default-features --features opengl`. MinGW builds do
the same. If you do not need Vulkan, removing it on desktop also removes the
shaderc build dependency.

## Related

- [Linux guide](../guides/linux.md)
- [Windows guide](../guides/windows.md)
- [Android guide](../guides/android.md)
- [Cross-compilation](../guides/cross-compilation.md)
- [Platform API](../api/platform.md)
