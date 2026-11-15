# Configuration

## Runtime environment variables

These are read by the engine or the demo when the game starts.

| Variable | Values | Default | Effect |
|----------|--------|---------|--------|
| `SOLDATIK3D_BACKEND` | `vulkan`, `vk`, `opengl`, `gl` | platform default | Force the graphics backend |
| `SOLDATIK3D_VSYNC` | `on`, `off`, `adaptive`, `mailbox` (plus `1/0`, `true/false`, `enabled/disabled`, `adapt`, `triple`) | `on` | Swapchain pacing |
| `SOLDATIK3D_MAX_FRAMES` | unsigned integer | unset | Exit after N rendered frames |
| `SOLDATIK3D_COMMANDS` | `"cmd1;cmd2"` | empty | Execute console commands at startup |
| `SOLDATIK3D_CONSOLE` | `1`, `true`, `yes`, `on` | closed | Open the console on startup |
| `SOLDATIK3D_VALIDATION` | `1`, `true`, `yes`, `on` | debug builds on | Request Vulkan validation layers |
| `SOLDATIK3D_DEBUG_VIEW` | `lit`, `normals`, `depth` | `lit` | Demo's initial debug view |

Notes:

- `SOLDATIK3D_BACKEND` overrides `GameConfig::backend`. Unknown values are
  ignored. On Android only OpenGL ES is compiled, regardless of the value.
- `SOLDATIK3D_VSYNC` overrides `GameConfig::vsync`. `SOLDATIK3D_VSYNC=off` is
  the intended benchmark mode; combine it with `SOLDATIK3D_MAX_FRAMES`.
- `SOLDATIK3D_MAX_FRAMES` counts frames that reached presentation, not update
  calls.
- `SOLDATIK3D_COMMANDS` runs after the world and renderer exist. Semicolons are
  the separator; empty entries are ignored.
- `SOLDATIK3D_VALIDATION` only affects Vulkan. Validation is enabled
  automatically in debug builds when the layer is installed; releasing with
  this variable set also requests it.
- `SOLDATIK3D_DEBUG_VIEW` is read by `examples/demo`, not by the engine core.
  A game can implement its own initial `DebugView`.

Examples:

```bash
# Force OpenGL and exit after two frames
SOLDATIK3D_BACKEND=opengl SOLDATIK3D_MAX_FRAMES=2 cargo run -p demo

# Benchmark without VSync
SOLDATIK3D_VSYNC=off SOLDATIK3D_MAX_FRAMES=600 cargo run --release -p demo

# Open the console and select a camera at startup
SOLDATIK3D_CONSOLE=1 \
SOLDATIK3D_COMMANDS="camera.select overview" \
cargo run -p demo
```

## `s3d` environment variables

| Variable | Used by | Effect |
|----------|---------|--------|
| `SOLDATIK3D_ENGINE` | `s3d new` | Engine checkout used for the generated project's `runtime` path |
| `ANDROID_NDK_HOME` | `s3d build android`, `s3d check` | NDK root |
| `ANDROID_SDK_ROOT` / `ANDROID_HOME` | build scripts | SDK root |
| `JAVA_HOME` | Gradle | JDK 17 |
| `GRADLE_USER_HOME` | Gradle | Cache directory |

## Android build variables

Used by `build_apk.sh` and the Gradle project:

| Variable | Default | Effect |
|----------|---------|--------|
| `SOLDATIK3D_ANDROID_LIB_NAME` | `demo` / Cargo package | Native library name without `lib`/`.so` |
| `SOLDATIK3D_ANDROID_APP_ID` | `com.soldatik3d.game` / `com.soldatik3d.<package>` | Application ID |
| `SOLDATIK3D_ANDROID_APP_NAME` | `Soldatik3D Game` / game name | Display name |
| `SOLDATIK3D_KEYSTORE` | unset | Release keystore path |
| `SOLDATIK3D_KEYSTORE_PASSWORD` | unset | Keystore password |
| `SOLDATIK3D_KEY_ALIAS` | unset | Key alias |
| `SOLDATIK3D_KEY_PASSWORD` | unset | Key password |

When `SOLDATIK3D_KEYSTORE` is unset, release builds use the Android debug key.

## Build variables

| Variable | Effect |
|----------|--------|
| `SHADERC_LIB_DIR` | Directory containing `libshaderc_shared.so`; used by `shaderc-sys` to link a system shaderc instead of building one |
| `CARGO_TARGET_<TRIPLE>_LINKER` | Linker for a cross target |
| `CC_<triple>` / `AR_<triple>` | C compiler and archiver for native dependencies |
| `RUSTDOCFLAGS` | Extra rustdoc flags; CI uses `-D warnings` |

`build_apk.sh` sets the Android linker, `CC_*` and `AR_*` variables for the
child Cargo process. `scripts/cross_build.sh` does the same.

## Cargo features

### `runtime`

| Feature | Default | Effect |
|---------|---------|--------|
| `vulkan` | yes | Compile the Vulkan 1.2 backend (desktop only) |
| `opengl` | yes | Compile the OpenGL 3.3 / ES 3.0 backend |

```toml
# OpenGL-only build, no shaderc dependency
runtime = { path = "...", default-features = false, features = ["opengl"] }
```

A build with neither feature compiles but cannot create a renderer.

### Generated game

The template mirrors the runtime features:

```toml
[features]
default = ["vulkan", "opengl"]
vulkan = ["runtime/vulkan"]
opengl = ["runtime/opengl"]
```

`build_apk.sh` always passes `--no-default-features --features opengl`.

### Workspace crates

| Crate | Notable features |
|-------|------------------|
| `vulkan` | Backend implementation; pulls `ash`, `ash-window`, `shaderc` |
| `opengl` | Backend implementation; pulls `glutin`, `glow` |
| `platform` | `winit` window and input |
| `scene`, `ui`, `assets`, `render` | No optional features |

## Cargo profiles

Workspace `Cargo.toml`:

```toml
[profile.dev.package."*"]
opt-level = 3      # optimized dependencies in debug builds

[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
```

The generated template adds:

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
```

Debug builds are therefore usable for gameplay, while release builds are
LTO-optimized and stripped.

## Window configuration

`GameConfig` is the supported way to configure the window:

```rust
GameConfig::default()
    .with_title("My Game")
    .with_size(1280, 720)
    .with_cursor_locked(true)
    .with_backend(BackendKind::Vulkan)
    .with_vsync(VsyncMode::On)
```

The environment variables above override backend and VSync at startup.

## Logging

The engine prints initialization and error lines with a backend prefix:

```text
[engine-vulkan] ready: ...
[engine-opengl] ready: ...
[engine] cursor capture is unavailable; menu mode is active (Tab retries)
[engine] frame budget reached (2 frames)
```

There is no log-level or log-file configuration. Use the shell to redirect
stderr when collecting diagnostics:

```bash
cargo run -p demo 2>engine.log
```

## Related

- [CLI reference](cli.md)
- [Console commands](cli-commands.md)
- [Installation](../getting-started/installation.md)
