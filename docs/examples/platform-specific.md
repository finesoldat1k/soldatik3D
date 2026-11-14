# Platform-Specific Examples

Examples tailored for each target platform.

## Linux-Specific

### Vulkan Validation Layers

```bash
# Install validation layers
sudo apt-get install vulkan-validationlayers-dev

# Run with validation
SOLDATIK3D_VALIDATION=1 cargo run

# Or in code
std::env::set_var("VK_INSTANCE_LAYERS", "VK_LAYER_KHRONOS_validation");
```

### Wayland vs X11

```bash
# Force Wayland
GDK_BACKEND=wayland cargo run

# Force X11
GDK_BACKEND=x11 cargo run

# Check current
echo $XDG_SESSION_TYPE
```

### Multi-GPU (PRIME)

```bash
# Use discrete GPU (NVIDIA on Optimus)
DRI_PRIME=1 cargo run

# Or with environment
__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia cargo run
```

### Vulkan ICD Selection

```bash
# List available ICDs
ls /usr/share/vulkan/icd.d/

# Force specific ICD
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/radeon_icd.x86_64.json cargo run
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json cargo run
```

### System shaderc Workaround

```bash
# Ubuntu 24.04 workaround for incomplete libshaderc_combined.a
mkdir -p /tmp/shaderc-lib
ln -sf /usr/lib/x86_64-linux-gnu/libshaderc.so.1 /tmp/shaderc-lib/libshaderc_shared.so
export SHADERC_LIB_DIR=/tmp/shaderc-lib
cargo build
```

### Profiling with perf

```bash
# CPU profiling
perf record -g cargo run --release
perf report

# With debug symbols
cargo build --release
perf record -g ./target/release/your_game
```

---

## Windows-Specific

### MSVC Build (Recommended)

```cmd
:: Open "x64 Native Tools Command Prompt for VS 2022"
cd C:\path\to\your_game
cargo build --release

:: Run
target\release\your_game.exe
```

### Environment Setup

```cmd
:: Vulkan SDK (adjust version)
set VULKAN_SDK=C:\VulkanSDK\1.3.283.0
set PATH=%VULKAN_SDK%\Bin;%PATH%

:: Or add to System Environment Variables permanently
```

### MinGW Cross-Compile (from Linux)

```bash
# On Linux host
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --target x86_64-pc-windows-gnu --release \
  --no-default-features --features opengl

# Binary at: target/x86_64-pc-windows-gnu/release/your_game.exe
```

### Test with Wine

```bash
# Install Wine
sudo apt-get install wine64

# Run Windows build on Linux
wine64 target/x86_64-pc-windows-gnu/release/your_game.exe

# With OpenGL backend
SOLDATIK3D_BACKEND=opengl wine64 target/.../your_game.exe
```

### Visual Studio Debugging

```cmd
:: Generate VS project (requires cargo-vs)
cargo install cargo-vs
cargo vs

:: Or create solution manually
:: Open Developer Command Prompt
cmake -G "Visual Studio 17 2022" -A x64 ..
```

### Windows-Specific Features

```rust
#[cfg(target_os = "windows")]
fn windows_specific(ctx: &mut UpdateContext<'_>) {
    // Raw input for high-precision mouse
    // Available via winit's WindowEvent::RawInput
    
    // HDR support check
    let caps = soldatik3d::PlatformCapabilities::detect();
    if caps.hdr {
        // Enable HDR output
    }
    
    // Gamepad (XInput)
    if let Some(pad) = ctx.input.gamepad(0) {
        // Xbox controller layout
    }
}
```

### Distribution

```cmd
:: Inno Setup installer
iscc your_game.iss

:: NSIS installer
makensis your_game.nsi

:: Simple zip
powershell Compress-Archive -Path target\release\your_game.exe, assets\* -DestinationPath dist\your_game-windows.zip
```

---

## Android-Specific

### NativeActivity Setup

```rust
// src/lib.rs
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(MyGame, app).unwrap();
}
```

### Android Manifest (Auto-generated)

```xml
<!-- android/gradle/app/src/main/AndroidManifest.xml -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.yourgame.game">

    <uses-sdk android:minSdkVersion="29" android:targetSdkVersion="34" />
    <uses-feature android:glEsVersion="0x00030000" android:required="true" />

    <application
        android:hasCode="false"
        android:label="@string/app_name"
        android:icon="@mipmap/ic_launcher">

        <meta-data
            android:name="android.app.lib_name"
            android:value="your_game" />

        <activity
            android:name="android.app.NativeActivity"
            android:exported="true"
            android:configChanges="orientation|screenSize|keyboardHidden|smallestScreenSize|screenLayout|uiMode|density"
            android:screenOrientation="landscape"
            android:theme="@style/Theme.Soldatik3D.Fullscreen">

            <meta-data
                android:name="android.app.lib_name"
                android:value="your_game" />

            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
                <category android:name="android.intent.category.LEANBACK_LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

### Building Native Libraries

```bash
# Set up NDK
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c

# Add targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build
cargo build --target aarch64-linux-android --release \
  --no-default-features --features opengl
cargo build --target armv7-linux-androideabi --release \
  --no-default-features --features opengl

# Output:
# target/aarch64-linux-android/release/libyour_game.so
# target/armv7-linux-androideabi/release/libyour_game.so
```

### Building APK

```bash
# Using build script
./build_apk.sh arm64-v8a --release
./build_apk.sh armeabi-v7a --release

# Or CLI tool
soldatik3d apk arm64-v8a --release
soldatik3d apk armeabi-v7a --release

# Output:
# apk/app-arm64-v8a-release.apk
# apk/app-armeabi-v7a-release.apk
```

### Install & Run

```bash
# Connect device (USB debugging enabled)
adb devices

# Install
adb install -r apk/app-arm64-v8a-release.apk

# Launch
adb shell am start -n com.yourgame.game/android.app.NativeActivity

# View logs
adb logcat -s "your_game:V" "android-activity:V" "NativeActivity:V"
```

### Android-Specific Code

```rust
#[cfg(target_os = "android")]
fn android_specific(ctx: &mut UpdateContext<'_>) {
    // Touch input
    for touch in ctx.input.touches() {
        match touch.phase {
            TouchPhase::Started => { /* tap/drag start */ }
            TouchPhase::Moved => { /* drag */ }
            TouchPhase::Ended => { /* tap end */ }
            _ => {}
        }
    }
    
    // Virtual keyboard (limited)
    // Text input not directly supported in NativeActivity
    
    // Sensors (via JNI bridge)
    // Not directly exposed, implement Java bridge
    
    // Lifecycle handled automatically by NativeActivity
    // onPause/onResume -> game pauses/resumes
}
```

### Asset Loading

```rust
// Assets packaged in APK automatically
let texture = ctx.renderer.load_texture("assets/textures/albedo.png")?;

// Note: assets/ directory packed into APK automatically by Gradle
// Access same as desktop
```

### Performance Optimization

```rust
fn android_optimize(ctx: &mut UpdateContext<'_>) {
    // Reduce shadow map resolution
    // ctx.renderer.set_shadow_map_size(1024);
    
    // Lower SSAO resolution
    // Disable bloom on low-end
    // Reduce particle count
    
    // Check device capabilities
    let caps = soldatik3d::PlatformCapabilities::detect();
    if caps.max_texture_size < 4096 {
        // Use lower resolution textures
    }
}
```

### Debugging

```bash
# Logcat filtering
adb logcat -s "your_game:V" "android-activity:V" "NativeActivity:V"

# GDB debugging
# 1. Build with debug symbols
cargo build --target aarch64-linux-android

# 2. Start gdbserver on device
adb shell run-as com.yourgame.game \
  /data/local/tmp/gdbserver :5039 /data/app/~~/lib/arm64/libyour_game.so

# 3. Forward port
adb forward tcp:5039 tcp:5039

# 4. Connect
gdb-multiarch target/aarch64-linux-android/debug/your_game
(gdb) target remote localhost:5039

# Crash dumps
adb shell ls /data/tombstones/
adb pull /data/tombstones/tombstone_00 .
$ANDROID_NDK_HOME/ndk-stack -sym target/aarch64-linux-android/debug/your_game -dump tombstone_00
```

### Play Store Publishing

```bash
# Build App Bundle (AAB)
cd android/gradle
./gradlew bundleRelease

# Output: app/build/outputs/bundle/release/app-release.aab

# Signing (in build.gradle.kts)
android {
    signingConfigs {
        create("release") {
            storeFile = file("keystore.jks")
            storePassword = "storepass"
            keyAlias = "keyalias"
            keyPassword = "keypass"
        }
    }
    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("release")
        }
    }
}
```

---

## Cross-Platform Patterns

### Platform Detection

```rust
fn platform_specific(ctx: &mut UpdateContext<'_>) {
    #[cfg(target_os = "linux")]
    { /* Linux-specific */ }
    
    #[cfg(target_os = "windows")]
    { /* Windows-specific */ }
    
    #[cfg(target_os = "android")]
    { /* Android-specific */ }
    
    // Runtime detection
    let caps = soldatik3d::PlatformCapabilities::detect();
    if caps.opengl_es {
        // Mobile GPU path
    }
}
```

### Input Abstraction

```rust
fn handle_input(ctx: &mut UpdateContext<'_>) {
    // Unified across platforms
    let input = ctx.input;
    
    // Keyboard (all platforms)
    if input.is_key_pressed(KeyCode::W) { move_forward(); }
    
    // Mouse (desktop only)
    #[cfg(not(target_os = "android"))]
    {
        let delta = input.mouse_delta();
        rotate_camera(delta);
    }
    
    // Touch (Android only)
    #[cfg(target_os = "android")]
    for touch in input.touches() {
        handle_touch(touch);
    }
    
    // Gamepad (all platforms)
    if let Some(pad) = input.gamepad(0) {
        let stick = pad.axis(GamepadAxis::LeftStickX);
    }
}
```

### Asset Loading

```rust
fn load_assets(ctx: &mut UpdateContext<'_>) {
    // Same paths work on all platforms
    // Android: assets/ packed in APK
    // Desktop: assets/ relative to working dir
    
    let texture = ctx.renderer.load_texture("assets/textures/albedo.png")?;
    let model = ctx.renderer.load_model("assets/models/character.glb")?;
}
```

---

*See [Platform Guides](guides/) for detailed setup instructions, or [API Reference](api/platform.md) for platform API documentation.*