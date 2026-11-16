# Changelog

All notable changes to Soldatik3D are documented here.

## Unreleased

### Added

- `text` crate: dependency-free TrueType engine with SFNT/`cmap`/`glyf`/`kern`
  parsing, anti-aliased rasterization, text layout and a growable R8 glyph
  atlas. Open Sans Regular/Bold (Latin + Cyrillic, Apache-2.0) are embedded.
- Hybrid immediate-mode `ui` crate: retained arena tree with stable ids,
  flexbox-style layout, themed buttons, labels, toggles, sliders, dropdowns,
  progress bars, scroll panels, virtual joystick and touch camera.
- `FrameData::ui_atlas` carries optional R8 glyph-atlas updates; OpenGL uploads
  dirty regions, Vulkan re-uploads the single-channel texture on change.
- `platform::Input` exposes press/release edges, `primary_down` and a persistent
  `touch_mode` flag; the runtime forwards mouse press/release events.

### Changed

- `UpdateContext::ui` is now `&mut ui::Ui`. The runtime begins the UI frame
  before `Game::update`, lays out and paints the retained tree afterwards, then
  draws the console on top and uploads the glyph atlas only after a successful
  render.
- `runtime` re-exports the `ui` crate (`runtime::ui`), so games can build
  interfaces without a direct dependency on `ui`.
- The demo HUD and settings panel use engine widgets; touch buttons take
  priority over the right-half touch-camera gesture.

### Removed

- The legacy UI widget system and bitmap-font text path. The old
  `font_atlas_rgba8` helper remains for compatibility.

### Fixed

- Android Gradle builds: corrected the wrapper script, pinned Gradle 8.10.2 and
  AGP 8.7.3, fixed vector launcher icons and removed the unsupported
  `sourceSets` block.
- Android manifest placeholders now drive the native library name, application
  ID and display name.
- CI and release workflows use valid `android-actions/setup-android@v3`
  inputs, JDK 17, SDK 34 and NDK r26c.
- Release APKs fall back to debug signing when no keystore is configured.
- `s3d new` rewrites the runtime path dependency, handles names with mixed case
  and resolves the template relative to the executable or installation prefix.
- Linux release archives bundle `libshaderc.so.1` and `libvulkan.so.1` and use
  an `$ORIGIN` rpath.
- Clippy warnings in the `s3d` binary.

### Documentation

- Rewrote the README and all guides and API references against the actual
  public API, CLI options and console commands.

## 0.1.0 — 2026-11-14

### Engine

- Workspace split into `assets`, `scene`, `ui`, `render`, `platform`,
  `vulkan`, `opengl`, `runtime` and `s3d`.
- Public `runtime` facade: `Game`, `GameConfig`, `UpdateContext`,
  `CommandContext`, `run` and `run_android`.
- Vulkan 1.2 backend (`ash`, `shaderc`) and OpenGL 3.3 / OpenGL ES 3.0 backend
  (`glutin`, `glow`).
- Forward metallic/roughness PBR with a procedural environment cubemap and up
  to eight nearest point lights.
- One orthographic 2048² shadow map with PCF filtering.
- Half-resolution SSAO, bright-pass bloom, ACES tone mapping and gamma
  correction.
- Camera and shadow frustum culling.
- Named cameras with stable IDs: spawn, select, cycle, rename and remove.
- Immediate-mode `UiDrawList` with rectangles, bitmap text and a crosshair.
- Developer console with history, tab completion, camera commands, VSync
  commands, screenshots and game-defined commands.
- Keyboard and mouse input with named `InputMap` actions.
- `DebugView` modes: lit, normals and depth.

### Platforms

- Linux x86_64: Vulkan and OpenGL.
- Windows x86_64 MSVC: Vulkan and OpenGL.
- Windows x86_64 GNU cross builds: OpenGL only.
- Android arm64-v8a, armeabi-v7a and x86_64: OpenGL ES 3.0, NativeActivity
  `cdylib`.
- macOS, iOS and WebAssembly are not supported.

### Tooling

- `s3d` CLI: `new`, `build`, `run`, `apk`, `export`, `info`, `engine`, `check`.
- Self-contained game template with desktop and Android targets, its own
  Gradle project and asset packaging.
- `scripts/cross_build.sh` for local Linux/Windows/Android packages.
- `scripts/apk/build_apk.sh` for local debug and release APKs.
- GitHub Actions CI on every push and pull request.
- GitHub Actions release workflow that attaches engine, demo, CLI and APK
  artifacts to every GitHub Release, including prereleases.

### Known limitations

- No audio, physics, animation, networking or scripting.
- No gamepad or touch input API.
- No runtime mesh or material re-upload; the scene is static after startup.
- No asset manager or hot reload.
- Release APKs are debug-signed unless a keystore is configured.
- `s3d engine update` and `s3d engine link` are placeholders.
- `s3d apk --sign` is accepted but signing happens through Gradle environment
  variables.

## Roadmap

Not implemented; listed for orientation only:

- Dynamic geometry and multi-material scenes.
- Asset manager with runtime loading.
- Audio, physics and animation integrations.
- Gamepad and touch input.
- Release signing defaults for generated projects.
