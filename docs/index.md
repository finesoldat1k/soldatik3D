# Soldatik3D Documentation

Soldatik3D is a modular 3D game engine written in Rust. It targets **Linux,
Windows and Android** with two production backends: **Vulkan 1.2** and
**OpenGL 3.3 core / OpenGL ES 3.0**.

The engine owns the window, event loop, input snapshot, renderer and developer
console. A game implements the `Game` trait, builds a `World` once, and mutates
it every frame.

## Start here

1. [Installation](getting-started/installation.md) — toolchains and system packages.
2. [Quick start](getting-started/quick-start.md) — build and run the demo, create a project.
3. [Your first game](getting-started/first-game.md) — a complete small game.
4. [Project structure](getting-started/project-structure.md) — engine workspace and generated projects.

## Guides

| Guide | Contents |
|-------|----------|
| [Architecture](guides/architecture.md) | Crates, dependency direction, renderer abstraction |
| [Game loop](guides/game-loop.md) | `Game`, `UpdateContext`, world ownership, UI and input |
| [Linux](guides/linux.md) | Packages, build, run, relocatable packages |
| [Windows](guides/windows.md) | MSVC builds, MinGW cross builds |
| [Android](guides/android.md) | NativeActivity, NDK/SDK setup, APK builds, signing |
| [Cross-compilation](guides/cross-compilation.md) | `scripts/cross_build.sh` and `s3d` |

## API reference

| Reference | Types |
|-----------|-------|
| [Runtime](api/runtime.md) | `Game`, `GameConfig`, `UpdateContext`, `run`, `run_android` |
| [Scene](api/scene.md) | `World`, `Camera`, `CameraRig`, `Scene`, `Mesh`, lights, material |
| [Rendering](api/rendering.md) | `BackendKind`, `VsyncMode`, `DebugView`, `RenderStats`, `Renderer` |
| [Input](api/input.md) | `Input`, `InputMap`, keyboard, mouse, actions |
| [UI and console](api/ui.md) | `UiDrawList`, text, rectangles, console commands |
| [Assets](api/assets.md) | `TextureData`, font atlas, environment cubemap |
| [Platform](api/platform.md) | `TargetOS`, `TargetArch`, capabilities, `winit` re-export |

## Reference

- [s3d CLI](reference/cli.md) — exact commands and options.
- [Console commands](reference/cli-commands.md) — built-in commands and arguments.
- [Configuration](reference/configuration.md) — environment variables and Cargo features.
- [CI and releases](CI.md) — GitHub Actions jobs, artifacts and local equivalents.
- [Changelog](reference/changelog.md) — version history.
- [Migration](reference/migration.md) — moving older projects to the current layout.

## Examples

- [Basic](examples/basic.md) — scene, camera, input and UI.
- [Advanced](examples/advanced.md) — procedural geometry, cameras and game commands.
- [Platform-specific](examples/platform-specific.md) — Linux, Windows and Android code.

## Supported platforms

| Platform | Backend | Status |
|----------|---------|--------|
| Linux x86_64 | Vulkan 1.2, OpenGL 3.3 | Supported |
| Windows x86_64 (MSVC) | Vulkan 1.2, OpenGL 3.3 | Supported |
| Windows x86_64 (MinGW cross build) | OpenGL 3.3 | Supported |
| Android arm64-v8a / armeabi-v7a | OpenGL ES 3.0 | Supported |
| macOS, iOS, WebAssembly | — | Not supported |

## Quick commands

```bash
git clone https://github.com/finesoldat1k/soldatik3D
cd soldatik3D
cargo run -p demo                 # run the demo
cargo build --release -p s3d      # build the CLI
./target/release/s3d new my_game  # create a game
```

## Community

- [GitHub repository](https://github.com/finesoldat1k/soldatik3D)
- [Issues](https://github.com/finesoldat1k/soldatik3D/issues)
- [Discussions](https://github.com/finesoldat1k/soldatik3D/discussions)

License: GPL-3.0. See [LICENSE](../LICENSE).
