# Windows Development

## Native MSVC builds

### Requirements

1. **Visual Studio Build Tools** with the *Desktop development with C++*
   workload (MSVC compiler, Windows SDK, CMake).
2. **Rust stable with the MSVC toolchain**:

```powershell
rustup default stable-msvc
rustc --version
```

3. A GPU driver with Vulkan and/or OpenGL 3.3 support.

The engine builds Vulkan shader tooling through CMake; the Vulkan SDK itself is
not required.

### Build and run

```powershell
git clone https://github.com/finesoldat1k/soldatik3D
cd soldatik3D

cargo build --locked -p runtime --all-features
cargo build --locked -p demo --all-features
cargo run -p demo

# OpenGL fallback
$env:SOLDATIK3D_BACKEND = "opengl"
cargo run -p demo
```

From a generated game project:

```powershell
cargo run
s3d build windows --release
```

`s3d build windows` on Windows uses the native MSVC target; on Linux it uses
MinGW and OpenGL only.

### Release artifacts

The release workflow produces:

- `soldatik3d-engine-windows-x86_64.zip` — `libruntime.rlib`
- `soldatik3d-demo-windows-x86_64.zip` — `demo.exe` and `README.txt`
- `soldatik3d-cli-windows-x86_64.zip` — `bin/s3d.exe` plus the project template
  and engine sources needed by `s3d new`

The demo executable is not code-signed. Windows SmartScreen may warn on first
launch; use *More info → Run anyway* or sign the binary yourself.

## Cross-compiling from Linux

MinGW builds use the OpenGL backend only; Vulkan is not built for
`x86_64-pc-windows-gnu`.

```bash
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu

./scripts/cross_build.sh windows --release
ls dist/windows-x86_64/
# demo.exe  libruntime.rlib  s3d.exe
```

Or from a game project:

```bash
s3d build windows --release --out-dir dist
```

The resulting `.exe` needs a Windows machine (or Wine) to run. It links the
MinGW runtime; no additional engine DLLs are required.

## Packaging tips

- Keep `demo.exe` and any data files in the same directory.
- The engine does not load external shader files at runtime on Windows: GLSL is
  compiled by shaderc and embedded by the backend.
- Use `SOLDATIK3D_VSYNC=off` for benchmarks.
- Use `SOLDATIK3D_MAX_FRAMES=N` for automated smoke tests.
- Set a console window title with `GameConfig::with_title`.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `link.exe not found` | Install Visual Studio Build Tools with C++ and run from a Developer PowerShell, or let `rustup` configure the MSVC environment. |
| `LNK1181: cannot open input file 'vulkan.lib'` | The Vulkan loader import library is missing; install the Vulkan SDK or use `SOLDATIK3D_BACKEND=opengl` with a build without the `vulkan` feature. |
| MinGW cross build fails in shaderc | Expected: use `--no-default-features --features opengl`, which `scripts/cross_build.sh windows` does automatically. |
| OpenGL context creation fails | Update the GPU driver; remote desktop sessions may not expose OpenGL 3.3. |
| Vulkan validation errors | Update the driver, or run the OpenGL backend. |

## Next steps

- [Linux](linux.md)
- [Android](android.md)
- [Cross-compilation](cross-compilation.md)
