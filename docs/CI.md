# CI and Releases

Soldatik3D has two GitHub Actions workflows:

| Workflow | File | Trigger |
|----------|------|---------|
| CI | `.github/workflows/ci.yml` | Every push, every pull request, manual dispatch |
| Release | `.github/workflows/release.yml` | GitHub Release published, manual dispatch |

Both use `ubuntu-latest` for Linux and Android and `windows-latest` for Windows.
They pin stable Rust and use `Swatinem/rust-cache@v2`.

## CI jobs

### Linux x86_64

1. Installs X11/Wayland, Mesa, Vulkan and `libshaderc-dev`.
2. Points `shaderc-sys` at the system `libshaderc.so.1` because Ubuntu's
   `libshaderc_combined.a` is incomplete.
3. Runs `cargo fmt --all -- --check`.
4. Runs clippy with `--workspace --all-targets --all-features`.
5. Builds `runtime` and `demo` separately.
6. Runs workspace tests.
7. Builds rustdoc with `--no-deps`.
8. Packages debug artifacts and uploads them.

Artifacts, retained for 14 days:

- `soldatik3d-engine-linux-x86_64.tar.gz` — `libruntime.rlib`
- `soldatik3d-demo-linux-x86_64.tar.gz` — `demo`

### Windows x86_64 (MSVC)

Runs clippy, builds `runtime` and `demo`, runs tests, and uploads:

- `soldatik3d-engine-windows-x86_64.zip` — `libruntime.rlib`
- `soldatik3d-demo-windows-x86_64.zip` — `demo.exe`

Windows CI uses the native MSVC toolchain, so both backends are compiled.

### Android (matrix: `arm64-v8a`, `armeabi-v7a`)

Each matrix entry:

1. Installs NDK r26c with `nttld/setup-ndk@v1`.
2. Adds the matching Rust target.
3. Installs JDK 17.
4. Exports the NDK linker, `CC_*` and `AR_*` variables.
5. Builds `runtime` and `demo` with `--no-default-features --features opengl`.
6. Installs Android SDK platform 34 and build-tools 34.0.0.
7. Copies `scripts/apk/gradle` to a temporary `android/gradle`, copies
   `libdemo.so` into `jniLibs/<abi>/`, and runs `assembleDebug`.
8. Verifies the APK contains `lib/<abi>/libdemo.so` and `AndroidManifest.xml`.
9. Uploads engine, native demo and APK artifacts.

Artifacts, retained for 14 days:

- `soldatik3d-engine-android-<abi>.tar.gz` — `libruntime.rlib`
- `soldatik3d-demo-android-<abi>.tar.gz` — `libdemo.so` and
  `app-<abi>-debug.apk`

`fail-fast` is disabled so both ABIs report independently.

## Release jobs

The release workflow listens for `release: types: [published]`, which includes
prereleases. It also has `workflow_dispatch` for building artifacts manually;
in that case the publish job is skipped because it requires
`github.event_name == 'release'`.

### Linux

Builds release `runtime`, `demo` and `s3d`. The demo is linked with:

```text
-C link-arg=-Wl,-rpath,$ORIGIN -C link-arg=-Wl,--enable-new-dtags
```

and packages `libshaderc.so.1` and `libvulkan.so.1` next to the executable.
Uploads:

- `soldatik3d-engine-linux-x86_64.tar.gz` — `libruntime.rlib`
- `soldatik3d-demo-linux-x86_64.tar.gz` — `demo`, bundled libraries, `README.txt`
- `soldatik3d-cli-linux-x86_64.tar.gz` — the `s3d` toolkit (see below)

### Windows

Builds release `runtime`, `demo` and `s3d` with MSVC and uploads:

- `soldatik3d-engine-windows-x86_64.zip` — `libruntime.rlib`
- `soldatik3d-demo-windows-x86_64.zip` — `demo.exe`, `README.txt`
- `soldatik3d-cli-windows-x86_64.zip` — the `s3d` toolkit

### Android

Builds release `runtime` and `demo` for both ABIs, packages them with
`assembleRelease`, verifies the APK and uploads:

- `soldatik3d-engine-android-<abi>.tar.gz` — `libruntime.rlib`
- `soldatik3d-demo-android-<abi>.tar.gz` — `libdemo.so`, release APK, `README.txt`

Release APKs are signed with the Android debug key unless the release
environment provides `SOLDATIK3D_KEYSTORE`, `SOLDATIK3D_KEYSTORE_PASSWORD`,
`SOLDATIK3D_KEY_ALIAS` and `SOLDATIK3D_KEY_PASSWORD`.

### Publish

The `publish` job waits for Linux, Windows and Android, downloads every
`release-*` artifact, and attaches:

- `*.tar.gz`
- `*.zip`
- `*.apk`

to the GitHub Release with `softprops/action-gh-release@v2`.
`fail_on_unmatched_files` is enabled, so a missing artifact fails the job
instead of silently producing an incomplete release. The workflow declares
`contents: write`; prereleases are preserved via
`prerelease: ${{ github.event.release.prerelease }}`.

## CLI toolkit archive

The `soldatik3d-cli-*` archive is not a single binary. `s3d new` needs the game
template and the engine sources that the generated project references. Layout:

```text
soldatik3d-cli-linux-x86_64/
├── bin/
│   └── s3d
├── templates/
│   └── game/
├── crates/
│   ├── assets/
│   ├── scene/
│   ├── ui/
│   ├── render/
│   ├── platform/
│   ├── vulkan/
│   ├── opengl/
│   ├── runtime/
│   └── s3d/
├── examples/
│   └── demo/
├── Cargo.toml
├── Cargo.lock
├── README.md
└── LICENSE
```

`examples/demo` is included because the root `Cargo.toml` lists it as a
workspace member and Cargo requires every member to exist when resolving the
workspace.

`find_template_dir` checks `../templates/game` relative to the executable, so
`bin/s3d` finds `templates/game` in the same archive. `find_engine_root` then
points generated projects at `crates/runtime` inside the extracted toolkit.
Keep the toolkit directory in place if you want generated projects to keep
building.

On Windows the layout is identical with `bin/s3d.exe` inside a `.zip`.

## Creating a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

Then create a GitHub Release from the tag. Marking it as a prerelease still
triggers the workflow. The release is published by GitHub, and the workflow
attaches the artifacts afterwards; refresh the release page when it finishes.

To build release artifacts without publishing, run the Release workflow
manually with **Run workflow**. The publish job will be skipped.

## Local equivalents

| CI step | Local command |
|---------|---------------|
| Formatting | `cargo fmt --all -- --check` |
| Clippy | `cargo clippy --locked --workspace --all-targets --all-features` |
| Build runtime | `cargo build --locked -p runtime --all-features` |
| Build demo | `cargo build --locked -p demo --all-features` |
| Tests | `cargo test --locked --workspace --all-features` |
| Docs | `cargo doc --locked --workspace --no-deps --all-features` |
| Linux package | `./scripts/cross_build.sh linux --release` |
| Debug APK | `./scripts/apk/build_apk.sh arm64-v8a` |
| Release APK | `./scripts/apk/build_apk.sh arm64-v8a --release` |
| Game APK | `./build_apk.sh arm64-v8a --release` in the project |

## Common CI problems

| Symptom | Cause |
|---------|-------|
| Linker cannot find shaderc symbols | Ubuntu's combined static library is incomplete; use the `SHADERC_LIB_DIR` workaround shown in the workflow |
| `Could not find or load main class "-Xmx64m"` | Broken Gradle wrapper; the repository's `gradlew` must be LF-only and executable |
| `attribute android:cx not found` | Unsupported vector drawable element; launcher icons must use `<path>` |
| APK built but no library | `SOLDATIK3D_ANDROID_LIB_NAME` does not match the Cargo `[lib] name` |
| Release asset missing | `fail_on_unmatched_files` makes the publish job fail; check the per-platform job logs |
| `not a valid NDK` | `nttld/setup-ndk` needs a full NDK version such as `r26c`, not a partial one |

## Related

- [Cross-compilation](guides/cross-compilation.md)
- [Android guide](guides/android.md)
- [Installation](getting-started/installation.md)
