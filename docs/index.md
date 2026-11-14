# Soldatik3D Documentation

Welcome to the official documentation for **Soldatik3D**, a modern, cross-platform 3D game engine written in Rust.

## 📚 Documentation Structure

### Getting Started
- **[Quick Start](getting-started/quick-start.md)** - Build your first game in 5 minutes
- **[Installation](getting-started/installation.md)** - Prerequisites and setup for all platforms
- **[Project Structure](getting-started/project-structure.md)** - Understanding the project layout
- **[Your First Game](getting-started/first-game.md)** - Step-by-step tutorial

### Core Concepts
- **[Architecture Overview](guides/architecture.md)** - Engine design and data flow
- **[Game Loop](guides/game-loop.md)** - Understanding the update/render cycle
- **[ECS & World](guides/ecs-world.md)** - Entity-Component-System and world management
- **[Rendering Pipeline](guides/rendering-pipeline.md)** - Vulkan & OpenGL backends explained

### Platform Guides
- **[Linux Development](guides/linux.md)** - Native Linux development
- **[Windows Development](guides/windows.md)** - MSVC and MinGW builds
- **[Android Development](guides/android.md)** - Native activity, APK building, deployment
- **[Cross-Compilation](guides/cross-compilation.md)** - Building for all platforms from Linux

### API Reference
- **[Runtime API](api/runtime.md)** - `Game`, `GameConfig`, `UpdateContext`, `World`
- **[Rendering API](api/rendering.md)** - `Renderer`, `Camera`, `Light`, `Material`
- **[Scene API](api/scene.md)** - `Scene`, `Mesh`, `Camera`, `Transform`
- **[Input API](api/input.md)** - Keyboard, mouse, gamepad, touch
- **[UI API](api/ui.md)** - Immediate-mode GUI, console, debug overlay
- **[Asset API](api/assets.md)** - Loading textures, models, shaders
- **[Platform API](api/platform.md)** - Window, timing, platform capabilities

### Advanced Topics
- **[Custom Shaders](guides/custom-shaders.md)** - GLSL shader development
- **[Post-Processing](guides/post-processing.md)** - Bloom, SSAO, tone mapping
- **[Shadows](guides/shadows.md)** - Cascaded shadow maps
- **[Performance Optimization](guides/performance.md)** - Profiling, best practices
- **[Debugging Tools](guides/debugging.md)** - Console, validation layers, debug views

### CLI Tool
- **[soldatik3d CLI](reference/cli.md)** - Project creation, building, exporting
- **[Commands Reference](reference/cli-commands.md)** - Complete command list
- **[Configuration](reference/configuration.md)** - `.cargo/config.toml`, environment variables

### Examples
- **[Basic Examples](examples/basic.md)** - Minimal games
- **[Advanced Examples](examples/advanced.md)** - Complex scenes, custom rendering
- **[Platform-Specific](examples/platform-specific.md)** - Android, Windows details

### Migration & Compatibility
- **[Version History](reference/changelog.md)** - Release notes
- **[Migration Guide](reference/migration.md)** - Upgrading between versions

---

## 🚀 Quick Links

| Task | Command |
|------|---------|
| Create new project | `soldatik3d new my_game` |
| Run on desktop | `cargo run` |
| Build release | `soldatik3d build --release` |
| Build Android APK | `soldatik3d apk arm64-v8a` |
| Export all platforms | `soldatik3d export` |
| Check configuration | `soldatik3d check` |

---

## 🎮 Engine Features

- **Dual Backend**: Vulkan 1.2 (via `ash`) and OpenGL 3.3/ES 3.0 (via `glutin`/`glow`)
- **Modern Rendering**: HDR, PBR, SSAO, Bloom, ACES tone mapping, cascaded shadows
- **Cross-Platform**: Linux, Windows (MSVC/MinGW), Android (API 29+)
- **Native Android**: `android.app.NativeActivity` with `android_main` entry point
- **Developer Experience**: In-game console, hot-reload shaders, debug views, screenshot capture
- **Type-Safe**: Rust 2024 edition, zero-cost abstractions
- **No GC**: Predictable performance, suitable for real-time applications

---

## 📖 Learning Path

### Beginner (Start Here)
1. [Installation](getting-started/installation.md) → [Quick Start](getting-started/quick-start.md) → [Your First Game](getting-started/first-game.md)

### Intermediate
2. [Game Loop](guides/game-loop.md) → [ECS & World](guides/ecs-world.md) → [Input Handling](api/input.md)
3. [Rendering Pipeline](guides/rendering-pipeline.md) → [Custom Shaders](guides/custom-shaders.md)

### Advanced
4. [Performance Optimization](guides/performance.md) → [Android Deployment](guides/android.md)
5. [Custom Render Passes](guides/custom-render-passes.md) → [Engine Extension](guides/engine-extension.md)

---

## 🤝 Community & Support

- **GitHub**: [finesoldat1k/soldatik3D](https://github.com/finesoldat1k/soldatik3D)
- **Issues**: [Bug reports & feature requests](https://github.com/finesoldat1k/soldatik3D/issues)
- **Discussions**: [Q&A and community](https://github.com/finesoldat1k/soldatik3D/discussions)
- **License**: GPL-3.0

---

## 📄 License

Soldatik3D is licensed under the **GNU General Public License v3.0**. See [LICENSE](../LICENSE) for details.

---

*Last updated: 2026-11-14 | Engine version: 0.1.0*