# Linux Development

## Requirements

- Rust stable (edition 2024)
- X11 or Wayland development headers
- OpenGL 3.3 + EGL/GLX
- Vulkan loader and a Vulkan ICD (for the default desktop backend)
- C++17 compiler, CMake and Python 3 if shaderc is built from source

Debian/Ubuntu:

```bash
sudo apt-get install -y --no-install-recommends \
  build-essential pkg-config cmake ninja-build git python3 \
  libvulkan-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
  libxkbcommon-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev \
  libgles2-mesa-dev libshaderc-dev
```

Arch:

```bash
sudo pacman -S --needed base-devel rust cmake ninja python \
  vulkan-icd-loader vulkan-validation-layers shaderc \
  libx11 libxcursor libxi libxrandr libxkbcommon wayland mesa
```

If the distro provides `libshaderc.so.1`, point `shaderc-sys` at it:

```bash
mkdir -p /tmp/shaderc-lib
ln -sf /usr/lib/x86_64-linux-gnu/libshaderc.so.1 /tmp/shaderc-lib/libshaderc_shared.so
export SHADERC_LIB_DIR=/tmp/shaderc-lib
```

## Build and run

From the engine repository:

```bash
# Engine and demo
cargo build --locked -p runtime --all-features
cargo build --locked -p demo --all-features

# Run the demo
cargo run -p demo

# Explicit backend
SOLDATIK3D_BACKEND=opengl cargo run -p demo
SOLDATIK3D_BACKEND=vulkan cargo run -p demo

# Release
cargo run --release -p demo
```

From a generated game project:

```bash
cargo run
s3d build linux --release
```

## Verify the rendering path

A two-frame smoke test proves the window, surface, renderer and shaders all
work:

```bash
SOLDATIK3D_BACKEND=opengl SOLDATIK3D_MAX_FRAMES=2 cargo run -p demo
SOLDATIK3D_BACKEND=vulkan SOLDATIK3D_MAX_FRAMES=2 cargo run -p demo
```

Expected output includes a backend line (`[engine-opengl] ready: ...` or the
Vulkan equivalent) followed by `[engine] frame budget reached (2 frames)`.

## Backend selection

| Value | Backend | Notes |
|-------|---------|-------|
| `vulkan` / `vk` | Vulkan 1.2 | Desktop default |
| `opengl` / `gl` | OpenGL 3.3 core | Works on Mesa and vendor drivers |

`GameConfig::with_backend` sets the default in code. The environment variable
overrides it at startup.

## VSync and frame pacing

```bash
SOLDATIK3D_VSYNC=off cargo run --release -p demo
SOLDATIK3D_VSYNC=adaptive cargo run -p demo
SOLDATIK3D_VSYNC=mailbox cargo run -p demo
```

`Mailbox` is Vulkan-only; OpenGL falls back to `On`. Console commands
`vsync.on`, `vsync.off`, `vsync.adaptive`, `vsync.mailbox` and `vsync.toggle`
change it at runtime.

## Relocatable local package

`scripts/cross_build.sh linux` builds the runtime, demo and `s3d`, sets an
`$ORIGIN` rpath and bundles `libshaderc.so.1` and `libvulkan.so.1`:

```bash
./scripts/cross_build.sh linux --release
ls dist/linux-x86_64/
# demo  libruntime.rlib  libshaderc.so.1  libvulkan.so.1  s3d
./dist/linux-x86_64/demo
```

The package is relocatable: copy the whole directory and run `./demo`. If a
library is unavailable on the build machine, the script warns and the binary
falls back to the system loader path.

## CI behavior

The Linux CI job installs the packages listed above, points `shaderc-sys` at
`libshaderc.so.1`, then runs:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features
cargo build --locked -p runtime --all-features
cargo build --locked -p demo --all-features
cargo test --locked --workspace --all-features
cargo doc --locked --workspace --no-deps --all-features
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `error while loading shared libraries: libshaderc.so.1` | Use the bundled `scripts/cross_build.sh` package, install `shaderc`, or set `SHADERC_LIB_DIR` before building. |
| `Failed to create surface` on Wayland | Try `WAYLAND_DISPLAY=` and `DISPLAY=:0`, or run under XWayland. |
| Vulkan `ERROR_INCOMPATIBLE_DRIVER` | Install a working Vulkan ICD (`vulkan-radeon`, `vulkan-intel`, `nvidia-utils`). |
| Validation layer warnings | Update the driver; run with `SOLDATIK3D_BACKEND=opengl` to isolate backend issues. |
| Shader compile failure | The Vulkan backend compiles GLSL at runtime; check stderr for the shader name and compiler message. |
| Window opens but is black | Check that the scene has a camera (`World::default()` provides `main`) and lights. |

## Next steps

- [Cross-compilation](cross-compilation.md)
- [Windows](windows.md)
- [Rendering API](../api/rendering.md)
