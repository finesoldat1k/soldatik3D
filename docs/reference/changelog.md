# Changelog

All notable changes to Soldatik3D are documented here.

## [0.1.0] - 2026-11-14

### Added - Initial Release

#### Core Engine
- **Runtime Facade** - `soldatik3d` crate with `Game`, `GameConfig`, `UpdateContext`, `World` traits
- **Dual Backend Architecture** - Vulkan 1.2 (`ash`) and OpenGL 3.3/ES 3.0 (`glutin`/`glow`)
- **Scene System** - ECS-lite `World` with cameras, entities, lights, meshes, materials
- **Input Abstraction** - Unified keyboard, mouse, gamepad, touch across platforms
- **Immediate-Mode UI** - Console, debug overlay, custom UI rendering
- **Command System** - Extensible console with built-in and custom commands

#### Rendering Pipeline
- **Deferred Rendering** - Shadow map → HDR scene → SSAO → Bloom → Composite → UI
- **HDR Rendering** - RGBA16F targets with ACES tone mapping and gamma correction
- **SSAO** - Screen-space ambient occlusion with bilateral blur
- **Bloom** - Bright pass + separable Gaussian blur at half resolution
- **Cascaded Shadow Maps** - Directional light shadows with PCF filtering
- **PBR Materials** - Metallic/roughness workflow with normal/roughness/metallic maps
- **Image-Based Lighting** - Procedural environment cubemap for ambient lighting
- **Debug Views** - Lit, Normals, Depth, Wireframe, Shadow map visualization

#### Platform Support
- **Linux x86_64** - Vulkan + OpenGL, Wayland/X11, full feature set
- **Windows x86_64 MSVC** - Vulkan + OpenGL, native performance
- **Windows x86_64 GNU/MinGW** - OpenGL only (cross-compile from Linux)
- **Android arm64-v8a / armeabi-v7a** - OpenGL ES 3.0, NativeActivity, APK packaging

#### Android Support
- **NativeActivity** - Pure native `android_main` entry point
- **OpenGL ES 3.0** - GLSL 300 ES with precision qualifiers
- **APK Packaging** - Gradle template with jniLibs, automatic signing
- **Touch Input** - Multi-touch with gesture recognition
- **Asset Loading** - Assets packaged in APK, loaded from assets/

#### Developer Experience
- **In-Game Console** - Tab completion, history, custom commands
- **Built-in Commands** - Camera, backend, vsync, debug, screenshot, entity management
- **CLI Tool** - `soldatik3d new/build/run/apk/export/check/info/engine`
- **Hot-Reload Shaders** - `reload` command for development
- **Frame Budget** - `SOLDATIK3D_MAX_FRAMES` for CI/benchmarking
- **Validation Layers** - `SOLDATIK3D_VALIDATION=1` for Vulkan debugging

#### Build System
- **Cross-Compilation** - `cross_build.sh` for Linux/Windows/Android from Linux
- **CI/CD Ready** - GitHub Actions workflows for Linux, Windows, Android
- **Release Workflow** - Automatic APK/binary packaging on GitHub Release
- **Shader Compilation** - Runtime GLSL→SPIR-V (Vulkan) or driver (OpenGL)
- **System shaderc** - Workaround for Ubuntu's incomplete `libshaderc_combined.a`

#### Documentation
- **Comprehensive Guides** - Installation, quick start, first game tutorial
- **API Reference** - Runtime, rendering, scene, input, UI, assets, platform
- **Platform Guides** - Linux, Windows, Android, cross-compilation
- **CLI Reference** - Complete command documentation
- **Architecture Overview** - Engine design and data flow

---

## Upcoming Features (Post 0.1.0)

### Planned for 0.2.0
- [ ] Audio system (rodio integration)
- [ ] Physics integration (rapier/parry)
- [ ] Animation system (skinned meshes)
- [ ] Particle system
- [ ] Level editor integration
- [ ] Asset hot-reload
- [ ] WebAssembly target (experimental)

### Planned for 0.3.0
- [ ] Networking/multiplayer (bevy_net/tokio)
- [ ] Scripting (Lua via mlua)
- [ ] Visual scripting / node editor
- [ ] Advanced post-processing (DOF, motion blur, SSR)
- [ ] GPU-driven rendering (meshlets, culling)
- [ ] Ray tracing (Vulkan RT extensions)

### Platform Expansion
- [ ] macOS (Metal backend)
- [ ] iOS (Metal backend)
- [ ] WebGPU backend
- [ ] Console targets (via middleware)

---

## Breaking Changes Policy

- **Major version** (0.x → 1.0): Breaking changes allowed
- **Minor version** (0.x.y): New features, no breaking changes
- **Patch version** (0.x.y): Bug fixes only

Since we're in 0.x, breaking changes may occur between minor versions. Check migration guide when upgrading.

---

## Migration from Pre-Release

If you were using a pre-0.1 development version:

1. **Crate renamed** - `runtime` → `soldatik3d` (facade crate)
2. **Android entry point** - Now uses `android_main` with `run_android()`
3. **Crate types** - Added `cdylib` for Android, `rlib` for desktop
4. **Shader paths** - Moved to `crates/vulkan/shaders/`, `crates/opengl/shaders/`
5. **Config structure** - `GameConfig` now uses builder pattern
6. **Console commands** - New command registry system
7. **CLI tool** - New `soldatik3d` command-line tool

See [Migration Guide](migration.md) for detailed steps.

---

*See [Migration Guide](migration.md) for upgrade instructions, or [Installation](getting-started/installation.md) for fresh install.*