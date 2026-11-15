# Project Structure

## Engine workspace

The repository is a Cargo workspace. Each crate has one responsibility and the
dependency direction is acyclic:

```text
examples/demo ──► runtime ──► scene ──► assets
                     │           ▲
                     ├─► ui ─────┤
                     ├─► render  │
                     ├─► platform│
                     ├─► vulkan ─┘   (optional feature)
                     └─► opengl ──   (optional feature)
```

| Path | Package | Responsibility |
|------|---------|----------------|
| `crates/assets` | `assets` | `TextureData`, bitmap font atlas, environment cubemap |
| `crates/scene` | `scene` | `World`, `Scene`, `Camera`, `CameraRig`, `Mesh`, lights, material |
| `crates/ui` | `ui` | `UiDrawList`, `UiVertex`, rectangles and bitmap text |
| `crates/render` | `render` | Backend-neutral `Renderer` trait, `BackendKind`, `FrameData`, `VsyncMode` |
| `crates/platform` | `platform` | `winit` window, input snapshot, raw surface handles, platform detection |
| `crates/vulkan` | `vulkan` | Vulkan 1.2 backend, pipelines and GLSL shaders |
| `crates/opengl` | `opengl` | OpenGL 3.3 / ES 3.0 backend, FBO pipeline and shaders |
| `crates/runtime` | `runtime` | Public facade: `Game`, event loop, world ownership, console, UI submission |
| `crates/s3d` | `s3d` | Host CLI: project creation, builds, APK, export |
| `examples/demo` | `demo` | Demo game built only on `runtime` |
| `templates/game` | — | Self-contained game project template (desktop + Android) |
| `scripts/apk` | — | `build_apk.sh` and the canonical Gradle project used by CI |
| `scripts/cross_build.sh` | — | Local Linux/Windows/Android builds |

Rules enforced by the design:

- Games depend on `runtime`, never on a backend crate.
- `vulkan` and `opengl` do not depend on `winit`; they receive a
  `render::WindowSurface` containing raw window/display handles.
- The runtime is the only crate that knows which concrete backend it creates.
- `scene`, `assets` and `ui` have no GPU or windowing dependency.

## Generated game project

`s3d new my_game` copies `templates/game` and rewrites:

- the package name placeholder (`my_game` becomes the snake_case name, so
  `My-Game` becomes `my_game`),
- the Rust type placeholder (`MyGame` becomes the PascalCase name),
- the `runtime` path dependency to point at the engine checkout.

Resulting layout:

```text
my_game/
├── android/
│   └── gradle/              # NativeActivity APK project
│       ├── app/
│       │   ├── build.gradle.kts
│       │   └── src/main/AndroidManifest.xml
│       ├── build.gradle.kts
│       ├── gradle/wrapper/  # Gradle 8.10.2 wrapper
│       ├── gradlew
│       └── settings.gradle.kts
├── assets/                  # Copied into the APK by build.rs
├── shaders/                 # Your GLSL shaders
├── src/
│   ├── lib.rs               # Game implementation + android_main
│   └── main.rs              # Desktop entry point
├── build.rs                 # Android asset packaging
├── build_apk.sh             # Local APK builder
├── Cargo.toml
├── README.md
└── .gitignore
```

### Desktop entry point

`src/main.rs` is intentionally tiny:

```rust
fn main() -> anyhow::Result<()> {
    my_game::run_desktop()
}
```

### Android entry point

`src/lib.rs` is compiled as both `rlib` and `cdylib`:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

The `cdylib` is loaded by `android.app.NativeActivity`, which calls the exported
`android_main` function. The Gradle manifest receives the library name through
the `SOLDATIK3D_ANDROID_LIB_NAME` placeholder.

## Build outputs

| Command | Output |
|---------|--------|
| `cargo build -p demo` | `target/debug/demo` (Linux) / `target/debug/demo.exe` (Windows) |
| `cargo build -p runtime` | `target/debug/libruntime.rlib` |
| `s3d build linux --out-dir dist` | `dist/<package>` |
| `s3d build windows --out-dir dist` | `dist/<package>.exe` |
| `s3d build android --out-dir dist` | `dist/lib<package>-arm64-v8a.so`, `dist/lib<package>-armeabi-v7a.so` |
| `./build_apk.sh arm64-v8a` | `apk/app-arm64-v8a-debug.apk` |
| `./scripts/cross_build.sh linux --release` | `dist/linux-x86_64/` with `demo`, `libruntime.rlib`, `s3d` and bundled libraries |

## Generated Android project vs. engine Gradle project

There are two copies of the Gradle project:

- `scripts/apk/gradle` — canonical engine demo project used by CI and
  `scripts/apk/build_apk.sh`.
- `templates/game/android/gradle` — copy shipped inside generated games.

Both are kept identical. If you change the manifest, resource files, wrapper or
Gradle scripts, update both copies. The generated project builds its own native
library name from the Cargo package name.
