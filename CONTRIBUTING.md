# Contributing to Soldatik3D

Thanks for taking the time to contribute. This project is a small Rust game
engine with Vulkan, OpenGL and Android build paths; reliable builds and accurate
documentation matter as much as new features.

## Before you start

- Read [README.md](README.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
- Search existing issues and pull requests.
- For large changes, open an issue first so the design can be discussed.
- Supported platforms are Linux x86_64, Windows x86_64 and Android. macOS, iOS
  and WebAssembly are out of scope.

## Development setup

### Linux

```bash
sudo apt-get install -y --no-install-recommends \
  build-essential pkg-config cmake ninja-build git python3 \
  libvulkan-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
  libxkbcommon-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev \
  libgles2-mesa-dev libshaderc-dev

rustup toolchain install stable
rustup component add rustfmt clippy
```

If `libshaderc` is packaged as a shared library, use it instead of Ubuntu's
incomplete `libshaderc_combined.a`:

```bash
mkdir -p /tmp/shaderc-lib
ln -sf /usr/lib/x86_64-linux-gnu/libshaderc.so.1 /tmp/shaderc-lib/libshaderc_shared.so
export SHADERC_LIB_DIR=/tmp/shaderc-lib
```

### Windows

Install Visual Studio Build Tools with the C++ workload, then:

```powershell
rustup default stable-msvc
rustup component add rustfmt clippy
```

### Android

Install JDK 17, SDK platform 34, build-tools 34.0.0, NDK r26c and the Rust
targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
export ANDROID_SDK_ROOT=$HOME/Android/Sdk
```

## Checks

Run the same checks as CI before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features
cargo build --locked -p runtime --all-features
cargo build --locked -p demo --all-features
cargo test --locked --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features
```

Smoke-test both desktop backends when possible:

```bash
SOLDATIK3D_BACKEND=opengl SOLDATIK3D_MAX_FRAMES=2 cargo run -p demo
SOLDATIK3D_BACKEND=vulkan SOLDATIK3D_MAX_FRAMES=2 cargo run -p demo
```

Build an Android APK when touching Android code or Gradle files:

```bash
./scripts/apk/build_apk.sh arm64-v8a
unzip -l apk/app-arm64-v8a-debug.apk | grep 'lib/arm64-v8a/libdemo.so'
```

## Code style

- Rust edition 2024, formatted by `rustfmt` with default settings.
- Keep the dependency direction described in `docs/ARCHITECTURE.md`.
- Games and the template must depend only on `runtime`.
- Backend crates must not depend on `winit`.
- Prefer `anyhow::Result` at API boundaries and `thiserror` only if a typed
  error is genuinely useful.
- Avoid new dependencies unless they replace a meaningful amount of code.
- Public items need a short doc comment. Run rustdoc with `-D warnings`.
- Do not add `TODO` comments without an issue reference.

### Error messages

Actionable, short and specific:

```rust
anyhow::bail!(
    "Android NDK not found. Set ANDROID_NDK_HOME to your NDK path (e.g. $HOME/android-ndk-r26c)."
);
```

## Project-specific rules

### Gradle projects stay in sync

There are two copies of the Android Gradle project:

- `scripts/apk/gradle` (canonical, used by CI and the engine demo)
- `templates/game/android/gradle` (shipped to generated games)

When you change either, change both and verify:

```bash
diff -r scripts/apk/gradle templates/game/android/gradle \
  --exclude=build --exclude=.gradle --exclude=jniLibs --exclude=local.properties
```

### Template behavior

`s3d new` copies `templates/game` and rewrites `my_game`/`MyGame` and the
runtime path. Do not hardcode a package name in the template. Verify a generated
project builds outside the repository:

```bash
cargo run -p s3d -- new My-Test-Game --dir /tmp
cd /tmp/My-Test-Game
cargo check
./build_apk.sh arm64-v8a
```

Do not create `test_game*` directories inside the repository.

### Windows cross builds are OpenGL-only

The Vulkan backend's shaderc dependency does not cross-compile cleanly to
`x86_64-pc-windows-gnu`. Keep the `opengl`-only feature set for MinGW builds.

### Android is OpenGL ES-only

Keep Vulkan dependencies target-gated with `cfg(not(target_os = "android"))`.
The Android demo is a native-activity `cdylib` exporting `android_main`, not an
executable.

### Release portability

Linux release binaries depend on `libshaderc.so.1` and `libvulkan.so.1`. The
release workflow bundles both and links with an `$ORIGIN` rpath. Keep this
behavior when changing release packaging.

### CLI archives

The CLI archive must contain `bin/s3d`, `templates/game`, `crates/`,
`examples/demo`, `Cargo.toml` and `Cargo.lock`. `s3d new` resolves the template
and the engine root relative to the executable. Test an extracted archive
before changing its layout.

## Documentation

- Documentation must match the source, not an imagined API.
- Update `README.md`, the relevant `docs/` page and `docs/reference/changelog.md`
  in the same pull request.
- Use English in repository documentation and code comments.
- Include a small, compilable Rust example when documenting an API.
- Do not claim gamepad, touch, audio, physics, hot reload or other features
  that are not implemented.

## Commit messages

Use a short imperative subject and a body that explains *why*:

```text
Fix Android wrapper script and pin Gradle 8.10.2

The previous wrapper passed JVM flags as the main class...
```

Keep commits focused. Formatting-only changes should be separate.

## Pull requests

Include:

1. What changed and why.
2. How it was tested, with commands and platforms.
3. Screenshots for visual changes.
4. Notes for reviewers, including known limitations.

A pull request is ready when:

- fmt, clippy, tests and rustdoc pass,
- desktop builds and the demo smoke test pass,
- Android changes were built locally,
- docs and changelog are updated,
- no new warnings or dead code were introduced.

## Reporting bugs

Include:

- operating system and architecture,
- `rustc --version`, `cargo --version`,
- GPU and driver version for rendering bugs,
- backend (`SOLDATIK3D_BACKEND`) and command line,
- full stderr output, including `[engine-*]` lines,
- a minimal reproduction if possible.

For Android bugs, include the ABI, device/emulator, Android version,
`adb logcat` output and whether the APK was debug or release signed.

## License

By contributing you agree that your contributions are licensed under the
GPL-3.0, the same license as the project. See [LICENSE](LICENSE).
