# Windows Development

Complete guide for developing and building Soldatik3D games on Windows.

## Supported Configurations

| Configuration | Backend | Status | Notes |
|---------------|---------|--------|-------|
| MSVC (Visual Studio) | Vulkan + OpenGL | ✅ **Recommended** | Native performance, full features |
| MinGW-w64 (GNU) | OpenGL only | ✅ Supported | Cross-compile from Linux |
| MSYS2/Clang | OpenGL only | ⚠️ Experimental | Limited testing |

## MSVC Setup (Recommended)

### 1. Install Visual Studio Build Tools

1. Download [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
2. Run installer, select workloads:
   - ✅ **Desktop development with C++**
   - ✅ **Windows 10/11 SDK** (latest)
   - ✅ **C++ CMake tools for Windows**
3. Individual components (ensure these are checked):
   - MSVC v143 - VS 2022 C++ x64/x86 build tools
   - Windows 10 SDK (10.0.22621.0 or later)
   - C++ ATL/MFC build tools (optional)

### 2. Install Vulkan SDK

1. Download from [vulkan.lunarg.com](https://vulkan.lunarg.com/sdk/home)
2. Run installer (default location: `C:\VulkanSDK\1.3.xxx`)
3. **Important**: Check "Add to PATH" during install
4. Restart terminal/PowerShell after install

### 3. Install Rust

```powershell
# In PowerShell (Run as Administrator recommended)
winget install Rustlang.Rustup
# Or download from rustup.rs

# Restart terminal, then:
rustup default stable
rustup component add rustfmt clippy

# Add targets
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-pc-windows-gnu      # For MinGW cross-compile
rustup target add aarch64-linux-android     # Android ARM64
rustup target add armv7-linux-androideabi   # Android ARMv7
```

### 4. Verify Installation

Open **"x64 Native Tools Command Prompt for VS 2022"** (not regular cmd/PowerShell):

```cmd
:: Check tools
rustc --version
cargo --version
cl.exe
cmake --version
vulkaninfo --summary

:: Should show your GPU under "deviceName"
```

## Building

### Debug Build

```cmd
cd C:\path\to\your_game
cargo run
```

### Release Build

```cmd
cargo build --release
:: Binary at target\release\your_game.exe
```

### Run with Specific Backend

```cmd
:: Vulkan (default)
cargo run

:: OpenGL
set SOLDATIK3D_BACKEND=opengl && cargo run

:: Disable VSync for benchmarking
set SOLDATIK3D_VSYNC=off && cargo run --release
```

### Enable Validation Layers

```cmd
set VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
cargo run

:: Or use environment variable
set SOLDATIK3D_VALIDATION=1 && cargo run
```

## MinGW-w64 (Cross-compile from Linux)

### On Linux Host

```bash
# Install MinGW
sudo apt-get install mingw-w64

# Add target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --target x86_64-pc-windows-gnu --release

# Binary at target/x86_64-pc-windows-gnu/release/your_game.exe
```

### On Windows (MSYS2)

```bash
# In MSYS2 terminal
pacman -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-cmake mingw-w64-x86_64-ninja

# Add target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --target x86_64-pc-windows-gnu --release
```

**Limitations:**
- OpenGL only (Vulkan's `shaderc` C++ build is complex with MinGW)
- Some Windows-specific features may not work
- Use MSVC for production Windows builds

## Project Configuration for Windows

### Cargo.toml Adjustments

```toml
[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "debuginfo"

# Windows-specific
[target.'cfg(windows)'.dependencies]
# Windows-only dependencies here
```

### .cargo/config.toml

```toml
# MSVC (default on Windows host)
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# MinGW cross-compile
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
runner = "wine64"
rustflags = [
  "-C", "link-arg=-fuse-ld=lld",
  "-C", "link-arg=-static",
]
```

## Debugging

### Visual Studio Debugger

1. Open `your_game.sln` (generate with `cargo vs` or create manually)
2. Set project as startup
3. Press F5

Or attach to running process:
- Debug → Attach to Process → `your_game.exe`

### WinDbg / CDB

```cmd
:: Install Windows SDK (includes WinDbg)
:: Or download WinDbg Preview from Microsoft Store

cdb -G -G target\release\your_game.exe
```

### RenderDoc

1. Install from [renderdoc.org](https://renderdoc.org/)
2. Launch RenderDoc
3. File → Launch Application → Select `target\release\your_game.exe`
4. Capture with F12

### Tracy Profiler

```toml
# Cargo.toml
tracy-client = { version = "0.16", features = ["callstack"] }
```

```cmd
cargo build --release --features tracy
tracy-profiler.exe
target\release\your_game.exe
```

## Common Issues

### `LINK : fatal error LNK1181: cannot open input file 'vulkan-1.lib'`

```cmd
:: Ensure Vulkan SDK is in PATH
set VULKAN_SDK=C:\VulkanSDK\1.3.xxx
set PATH=%VULKAN_SDK%\Bin;%PATH%
set PATH=%VULKAN_SDK%\Lib;%PATH%

:: Or set in Cargo.toml
[target.'cfg(windows)'.dependencies]
ash = { version = "0.38", features = ["link"] }
```

### `error: linking with 'link.exe' failed: exit code: 1120`

```cmd
:: Usually missing Vulkan library
:: Check VULKAN_SDK environment variable
echo %VULKAN_SDK%

:: Should point to SDK root, e.g. C:\VulkanSDK\1.3.283.0
```

### `error: failed to run custom build command for 'shaderc-sys'`

```cmd
:: shaderc needs CMake, Ninja, Python
:: Install via winget/chocolatey
winget install Kitware.CMake Ninja-build Python.Python.3.11

:: Ensure in PATH
cmake --version
ninja --version
python --version
```

### MinGW: `error: cannot find -lvulkan`

```bash
# On Linux cross-compile
# Install mingw-w64 vulkan headers
sudo apt-get install mingw-w64-vulkan-headers

# Or set PKG_CONFIG_PATH
export PKG_CONFIG_PATH=/usr/x86_64-w64-mingw32/lib/pkgconfig
```

### `SOLDATIK3D_BACKEND=vulkan` fails on Windows

```cmd
:: Check Vulkan driver
vulkaninfo --summary | findstr deviceName

:: Try OpenGL as fallback
set SOLDATIK3D_BACKEND=opengl
cargo run
```

## Distribution Packaging

### Inno Setup Installer

```iss
; your_game.iss
[Setup]
AppName=Your Game
AppVersion=1.0
DefaultDirName={pf}\Your Game
OutputDir=dist
OutputBaseFilename=YourGame-Setup

[Files]
Source: "target\release\your_game.exe"; DestDir: "{app}"
Source: "assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\Your Game"; Filename: "{app}\your_game.exe"
Name: "{commondesktop}\Your Game"; Filename: "{app}\your_game.exe"
```

Build:
```cmd
iscc your_game.iss
```

### NSIS Installer

```nsi
; your_game.nsi
Name "Your Game"
OutFile "dist\YourGame-Installer.exe"
InstallDir "$PROGRAMFILES\Your Game"

Section
  SetOutPath "$INSTDIR"
  File "target\release\your_game.exe"
  File /r "assets\*"
  
  CreateDirectory "$SMPROGRAMS\Your Game"
  CreateShortcut "$SMPROGRAMS\Your Game\Your Game.lnk" "$INSTDIR\your_game.exe"
SectionEnd
```

Build:
```cmd
makensis your_game.nsi
```

### Zip Package (Simple)

```cmd
cd target\release
powershell Compress-Archive -Path your_game.exe, assets\* -DestinationPath ..\..\dist\your_game-windows.zip
```

## CI/CD (GitHub Actions)

```yaml
# .github/workflows/windows.yml
name: Windows Build

on: [push, pull_request]

jobs:
  build:
    runs-on: windows-latest
    timeout-minutes: 60
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc
          
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          key: windows-${{ hashFiles('**/Cargo.lock') }}
          
      - name: Build
        run: cargo build --locked --release
        shell: cmd
        
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: windows-msvc
          path: target/release/*.exe
          retention-days: 14
```

## Windows-Specific Features

### High DPI Support

```rust
// In Game::config
GameConfig::default()
    .with_window_hints(WindowHints {
        // Windows handles DPI scaling automatically
        ..Default::default()
    })
```

### Gamepad (XInput)

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    if let Some(pad) = ctx.input.gamepad(0) {
        // Left stick
        let move_x = pad.axis(GamepadAxis::LeftStickX);
        let move_y = pad.axis(GamepadAxis::LeftStickY);
        
        // Triggers
        let lt = pad.axis(GamepadAxis::LeftTrigger);
        let rt = pad.axis(GamepadAxis::RightTrigger);
        
        // Buttons
        if pad.is_button_pressed(GamepadButton::South) { // A
            jump();
        }
    }
}
```

### File Associations

```rust
// Register .sav extension for save files
// In installer or first run
use windows::Win32::UI::Shell::SHChangeNotify;
```

---

*See [Cross-Compilation](cross-compilation.md) for building Windows from Linux, or [Linux](linux.md) for the primary development platform.*