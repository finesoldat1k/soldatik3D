# Cross-Compilation

The engine ships two tools for building other targets:

- `scripts/cross_build.sh` — builds the engine demo and `s3d` into relocatable
  local packages under `dist/`.
- `s3d build` — builds a generated game project with Cargo and copies the
  artifact to an output directory.

Both are thin wrappers around Cargo. They do not use Docker or a custom build
system.

## Supported targets

| Host | Target | Compiler | Backends |
|------|--------|----------|----------|
| Linux | Linux x86_64 | native `cc` | Vulkan + OpenGL |
| Linux | Windows x86_64 | MinGW-w64 (`x86_64-w64-mingw32-gcc`) | OpenGL |
| Linux | Android arm64-v8a | NDK r26c | OpenGL ES 3.0 |
| Linux | Android armeabi-v7a | NDK r26c | OpenGL ES 3.0 |
| Linux | Android x86_64 | NDK r26c | OpenGL ES 3.0 |
| Windows | Windows x86_64 | MSVC | Vulkan + OpenGL |
| Windows | Android (from Windows) | NDK r26c | OpenGL ES 3.0 |

macOS, iOS and WebAssembly are not supported.

## `scripts/cross_build.sh`

Run it from anywhere; the script resolves the repository root itself.

```bash
./scripts/cross_build.sh <platform> [abi] [--release]
```

| Platform | Extra argument | Output |
|----------|----------------|--------|
| `linux` | — | `dist/linux-x86_64/` |
| `windows` | — | `dist/windows-x86_64/` |
| `android` | `arm64-v8a`, `armeabi-v7a`, `x86_64` | `dist/android-<abi>/` |
| `all` | — | all of the above |

Examples:

```bash
./scripts/cross_build.sh linux --release
./scripts/cross_build.sh windows --release
ANDROID_NDK_HOME=$HOME/android-ndk-r26c \
  ./scripts/cross_build.sh android arm64-v8a --release
ANDROID_NDK_HOME=$HOME/android-ndk-r26c ./scripts/cross_build.sh all --release
```

### Linux package

The Linux script builds `s3d`, the `runtime` rlib and the demo, then copies
them into `dist/linux-x86_64/`:

```text
dist/linux-x86_64/
├── demo
├── libruntime.rlib
├── libshaderc.so.1
├── libvulkan.so.1
└── s3d
```

The executables receive an `$ORIGIN` rpath so `libshaderc.so.1` and
`libvulkan.so.1` are found next to them. The package can be copied to another
Linux machine with a compatible glibc and run without installing shaderc.

If a library is missing on the build machine, the script warns and the binary
falls back to the system loader path.

### Windows package

The Windows cross build uses the `x86_64-pc-windows-gnu` target and the OpenGL
backend only:

```text
dist/windows-x86_64/
├── demo.exe
├── libruntime.rlib
└── s3d.exe
```

Requirements:

```bash
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu
```

The output is not tested automatically: run it on Windows or under Wine.

### Android package

The Android build produces a native `libdemo.so`:

```text
dist/android-arm64-v8a/
└── libdemo.so
```

This is a raw shared library for embedding, not an APK. To create an
installable APK, use `scripts/apk/build_apk.sh` or `s3d apk` in a generated
project. See the [Android guide](android.md).

## `s3d build`

Inside a generated project:

```bash
s3d build linux
s3d build windows --release
s3d build android
s3d build all --release --out-dir dist
```

| Option | Meaning |
|--------|---------|
| `TARGET` | `linux`, `windows`, `android`, `all`; default `linux` |
| `-r`, `--release` | Build with Cargo's release profile |
| `-f`, `--features` | Cargo features; default `vulkan,opengl` |
| `-o`, `--out-dir` | Copy the artifact here; default: leave it in `target/` |

Default feature sets per target:

| Target | Rust target | Features |
|--------|-------------|----------|
| Linux | host | `vulkan,opengl` |
| Windows | `x86_64-pc-windows-gnu` | `opengl` |
| Android | `aarch64-linux-android` | `opengl` |

For Android, `s3d build` uses `ANDROID_NDK_HOME` to configure the linker and
copies `lib<package>.so` to the output directory. It does not build an APK.

`--features` is passed through verbatim for Linux. Use
`--features opengl` if you do not have shaderc installed.

## `s3d export`

`s3d export` is a convenience wrapper that builds the project for several
platforms in release mode:

```bash
s3d export
s3d export --platforms linux,windows --out-dir dist
```

It writes the artifacts directly into the output directory (default `dist/`):

| File | Platform |
|------|----------|
| `<package>` | Linux executable |
| `<package>.exe` | Windows executable |
| `lib<package>-arm64-v8a.so` | Android arm64 |
| `lib<package>-armeabi-v7a.so` | Android armv7 |

No archives or APKs are produced. `s3d export` always uses the release profile.
On Linux, Windows export requires MinGW; Android export requires the NDK. On a
Windows host, `s3d export` uses MSVC for the Windows artifact and Linux
cross-compilation is unavailable, so pass `--platforms windows,android`.

For a relocatable Linux bundle with the shared libraries and `$ORIGIN` rpath,
use `./scripts/cross_build.sh linux --release` instead.

## Manual Cargo commands

The wrappers do not hide anything. Equivalent manual commands:

```bash
# Linux
cargo build --release -p demo --features vulkan,opengl

# Windows from Linux (OpenGL only)
cargo build --release -p demo --target x86_64-pc-windows-gnu \
  --no-default-features --features opengl

# Android from Linux
export NDK=$HOME/android-ndk-r26c/toolchains/llvm/prebuilt/linux-x86_64/bin
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$NDK/aarch64-linux-android29-clang
export CC_aarch64_linux_android=$NDK/aarch64-linux-android29-clang
export AR_aarch64_linux_android=$NDK/llvm-ar
cargo build --release --target aarch64-linux-android \
  --no-default-features --features opengl
```

## Why MinGW uses OpenGL only

The Vulkan backend's shader compiler (`shaderc-sys`) is built through CMake and
does not cross-compile cleanly to `x86_64-pc-windows-gnu`. The OpenGL backend
has no such dependency. Native MSVC Windows builds support both backends.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `linker 'x86_64-w64-mingw32-gcc' not found` | Install `mingw-w64` |
| `can't find crate for 'std'` | `rustup target add <target>` |
| Android linker not found | Set `ANDROID_NDK_HOME` to a complete NDK; check the host tag (`linux-x86_64`, `darwin-x86_64`, `windows-x86_64`) |
| `libshaderc` missing in the Linux package | Install `libshaderc-dev` before building, or set `SHADERC_LIB_DIR` |
| `s3d build android` cannot find `build_apk.sh` | That error belongs to `s3d apk`; run it from the project root |
| Windows binary does not start | Install the MinGW runtime or run it on a system with the matching DLLs; test under Wine |

## Next steps

- [Linux](linux.md)
- [Windows](windows.md)
- [Android](android.md)
- [CI and releases](../CI.md)
