# Android Development

Complete guide for building and deploying Soldatik3D games on Android.

## Architecture Overview

Soldatik3D uses **NativeActivity** for Android:

```
┌─────────────────────────────────────────────────────────────┐
│                    Android App (APK)                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ android.app.NativeActivity                            │  │
│  │   • No Java/Kotlin code required                      │  │
│  │   • Loads libyour_game.so via JNI                     │  │
│  │   • Calls android_main(app: AndroidApp)               │  │
│  └───────────────────────────────────────────────────────┘  │
│                           │                                  │
│                           ▼                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ libyour_game.so (cdylib)                              │  │
│  │   • Rust game code compiled for ARM64/ARMv7           │  │
│  │   • Exports #[unsafe(no_mangle)] fn android_main()    │  │
│  │   • Uses android-activity crate for JNI bridge        │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Requirements

### Host Machine (Linux recommended)
- Android NDK r26c+ (r27+ recommended)
- Android SDK (for APK building)
- Rust with Android targets
- Gradle 8.4+ (for APK building)

### Target Device
- Android API 29+ (Android 10+)
- OpenGL ES 3.0+ GPU
- ARM64 (arm64-v8a) or ARMv7 (armeabi-v7a)

## Setup

### 1. Install NDK

```bash
# Option 1: Android Studio (recommended)
# SDK Manager → NDK → Install r26c+

# Option 2: Command line
cd ~
wget https://dl.google.com/android/repository/android-ndk-r26c-linux.zip
unzip android-ndk-r26c-linux.zip
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
echo 'export ANDROID_NDK_HOME=$HOME/android-ndk-r26c' >> ~/.bashrc
source ~/.bashrc
```

### 2. Install Android SDK (for APK)

```bash
# Via Android Studio (easiest)
# 1. Install Android Studio
# 2. SDK Manager → Platforms → Android 34 (API 34)
# 3. SDK Tools → Build Tools 34.0.0, Platform Tools
# 4. Set ANDROID_SDK_ROOT or ANDROID_HOME

# Or command line
cd ~
wget https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
unzip commandlinetools-linux-*.zip -d android-sdk
export ANDROID_SDK_ROOT=$HOME/android-sdk
$HOME/android-sdk/cmdline-tools/bin/sdkmanager --licenses
$HOME/android-sdk/cmdline-tools/bin/sdkmanager "platforms;android-34" "build-tools;34.0.0" "platform-tools"
```

### 3. Add Rust Targets

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

### 4. Verify Setup

```bash
# Check NDK
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang

# Check SDK
$ANDROID_SDK_ROOT/platform-tools/adb version

# Test compile
cd /path/to/soldatik3D
ANDROID_NDK_HOME=$ANDROID_NDK_HOME cargo build --target aarch64-linux-android -p demo
```

## Building Native Libraries

### Debug Build

```bash
# ARM64 (most devices)
cargo build --target aarch64-linux-android \
  --no-default-features --features opengl

# ARMv7 (older devices)
cargo build --target armv7-linux-androideabi \
  --no-default-features --features opengl

# Output:
# target/aarch64-linux-android/debug/libyour_game.so
# target/armv7-linux-androideabi/debug/libyour_game.so
```

### Release Build

```bash
# ARM64
cargo build --target aarch64-linux-android --release \
  --no-default-features --features opengl

# ARMv7
cargo build --target armv7-linux-androideabi --release \
  --no-default-features --features opengl

# Output:
# target/aarch64-linux-android/release/libyour_game.so
# target/armv7-linux-androideabi/release/libyour_game.so
```

### Using the CLI Tool

```bash
soldatik3d build --target android --release
```

## Building APK

### Using build_apk.sh (Included in Template)

```bash
# Debug APK
./build_apk.sh arm64-v8a
./build_apk.sh armeabi-v7a

# Release APK
./build_apk.sh arm64-v8a --release
./build_apk.sh armeabi-v7a --release

# Output:
# apk/app-arm64-v8a-debug.apk
# apk/app-armeabi-v7a-release.apk
```

### Manual Gradle Build

```bash
# 1. Prepare Gradle project (from template)
mkdir -p android/gradle
cp -r templates/android/gradle/* android/gradle/

# 2. Copy native library
mkdir -p android/gradle/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/libyour_game.so \
   android/gradle/app/src/main/jniLibs/arm64-v8a/libyour_game.so

# 3. Build
cd android/gradle
chmod +x gradlew
./gradlew assembleRelease

# Output:
# app/build/outputs/apk/release/app-release.apk
```

### Using the CLI Tool

```bash
soldatik3d apk arm64-v8a --release
soldatik3d apk armeabi-v7a --release
```

## Installing on Device

### ADB Install

```bash
# Enable USB Debugging on device
# Settings → Developer Options → USB Debugging

# Connect device
adb devices

# Install
adb install -r apk/app-arm64-v8a-release.apk

# Launch
adb shell am start -n com.yourgame.game/android.app.NativeActivity
```

### Install Multiple ABIs (Universal APK)

The Gradle template builds a universal APK with both ABIs:

```bash
./gradlew assembleRelease
# Output: app/build/outputs/apk/release/app-release.apk
# Contains both arm64-v8a and armeabi-v7a libs
```

## Android Manifest Configuration

The template includes proper manifest:

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

**Key points:**
- `android:hasCode="false"` - No Java code, pure native
- `android.app.lib_name="your_game"` - Matches `libyour_game.so`
- `NativeActivity` - Handles window/input/events
- `android:configChanges` - Prevents activity recreation on rotation

## Game Code for Android

Your `src/lib.rs` needs the Android entry point:

```rust
use soldatik3d::{run, run_android, Game, GameConfig, UpdateContext, World};

// Your game struct
struct MyGame;

impl Game for MyGame {
    fn config(&self) -> GameConfig { GameConfig::default() }
    fn create_world(&mut self) -> World { World::default() }
    fn update(&mut self, ctx: &mut UpdateContext<'_>) { /* ... */ }
}

// Desktop entry point
fn main() -> anyhow::Result<()> { run(MyGame) }

// Android entry point - REQUIRED
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    if let Err(e) = run_android(MyGame, app) {
        eprintln!("Game failed: {e:?}");
    }
}
```

**Key points:**
- `#[unsafe(no_mangle)] pub fn android_main` - Required by `android-activity`
- Parameter is `AndroidApp` from `android-activity` crate
- Calls `run_android()` instead of `run()`

## Asset Loading on Android

Assets are packaged in the APK's `assets/` directory:

```bash
# In your game project
mkdir -p assets/textures assets/models

# Gradle copies assets/ to APK automatically
# Access at runtime:
let texture = ctx.renderer.load_texture("assets/textures/albedo.png")?;
```

**Note:** Use relative paths from `assets/` root. Android assets are read-only.

## Input Handling on Android

### Touch Input

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    for touch in &ctx.input.touches {
        match touch.phase {
            TouchPhase::Started => { /* touch down */ }
            TouchPhase::Moved => { /* drag */ }
            TouchPhase::Ended => { /* touch up */ }
            TouchPhase::Cancelled => { /* interrupted */ }
        }
        let pos = touch.position; // Normalized 0-1
    }
}
```

### Virtual Keyboard

```rust
// Text input not directly supported in NativeActivity
// Use external library or implement Java bridge
```

### Gamepad

```rust
if let Some(pad) = ctx.input.gamepad(0) {
    // Standard gamepad mapping works
}
```

## Performance Considerations

### GPU Features

| Feature | Support |
|---------|---------|
| OpenGL ES 3.0 | ✅ Required |
| OpenGL ES 3.1 | ✅ Most devices |
| OpenGL ES 3.2 | ✅ Modern devices |
| Vulkan | ❌ Not supported in engine |
| Compute Shaders | ✅ ES 3.1+ |
| Geometry Shaders | ✅ ES 3.2+ |

### Memory Management

```rust
// Monitor memory
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    if cfg!(target_os = "android") {
        // Check available memory
        // Reduce texture quality if low
    }
}
```

### Battery Life

```rust
// Limit frame rate
ctx.renderer.set_vsync(VsyncMode::On)?;

// Reduce GPU work when backgrounded
// (NativeActivity handles pause/resume)
```

## Debugging

### Logcat

```bash
# Filter for your game
adb logcat -s "your_game:V" "android-activity:V" "NativeActivity:V"

# Or all Rust logs
adb logcat | grep -E "your_game|error|Error"

# Save to file
adb logcat -d > game_log.txt
```

### GDB Debugging

```bash
# 1. Build with debug symbols
cargo build --target aarch64-linux-android

# 2. Start gdbserver on device
adb shell run-as com.yourgame.game \
  /data/local/tmp/gdbserver :5039 /data/app/~~/lib/arm64/libyour_game.so

# 3. Forward port
adb forward tcp:5039 tcp:5039

# 4. Connect from host
gdb-multiarch target/aarch64-linux-android/debug/your_game
(gdb) target remote localhost:5039
```

### Graphics Debugging

```bash
# Android GPU Inspector (AGI)
# 1. Install AGI from https://github.com/google/agi
# 2. Connect device
# 3. Trace your game

# Or use RenderDoc (limited Android support)
```

### Crash Dumps

```bash
# Get tombstones
adb shell ls /data/tombstones/
adb pull /data/tombstones/tombstone_00 .

# Use ndk-stack to symbolicate
$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-objdump \
  -d target/aarch64-linux-android/debug/your_game > symbols.txt
$ANDROID_NDK_HOME/ndk-stack -sym target/aarch64-linux-android/debug/your_game \
  -dump tombstone_00
```

## Publishing to Play Store

### App Bundle (AAB)

```bash
# In Gradle project
cd android/gradle
./gradlew bundleRelease

# Output: app/build/outputs/bundle/release/app-release.aab
# Upload to Play Console
```

### Signing

```gradle
# android/gradle/app/build.gradle.kts
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

### Versioning

```gradle
defaultConfig {
    versionCode = 1
    versionName = "1.0.0"
}
```

## Troubleshooting

### `android_main` not found

```bash
# Check export
nm -D target/aarch64-linux-android/release/libyour_game.so | grep android_main
# Should show: T android_main

# Ensure in src/lib.rs:
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: ...) { ... }
```

### Black screen / crash on startup

```bash
# Check logcat
adb logcat -s "NativeActivity:V" "android-activity:V"

# Common issues:
# - Missing android.app.lib_name meta-data
# - Wrong lib name (must match lib_name)
# - OpenGL ES version not supported
# - Missing permissions
```

### `dlopen failed: library "libyour_game.so" not found`

```bash
# Check jniLibs in APK
unzip -l apk/app-arm64-v8a-release.apk | grep libyour_game.so
# Should be in lib/arm64-v8a/libyour_game.so

# Check build.gradle includes jniLibs
sourceSets {
    main {
        jniLibs.srcDirs("src/main/jniLibs")
    }
}
```

### OpenGL ES errors

```bash
# Enable debug output
# In Game::create_world or first update
// Not directly available in NativeActivity
# Use SOLDATIK3D_VALIDATION=1 for Vulkan (not applicable)
// For OpenGL: check logcat for EGL errors
```

### Performance issues

```bash
# Profile with Android GPU Inspector
# Check:
# - GPU frame time
# - Memory usage
# - Shader compilation stalls
# - Draw call count
```

## CI/CD for Android

```yaml
# .github/workflows/android.yml
name: Android Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        abi: [arm64-v8a, armeabi-v7a]
        include:
          - abi: arm64-v8a
            target: aarch64-linux-android
          - abi: armeabi-v7a
            target: armv7-linux-androideabi
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup NDK
        uses: nttld/setup-ndk@v1
        with:
          ndk-version: r26c
          
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
          
      - name: Build native
        run: |
          cargo build --locked --release \
            --no-default-features --features opengl \
            --target ${{ matrix.target }}
          
      - name: Build APK
        run: |
          # Setup Gradle project
          # Copy lib to jniLibs
          # ./gradlew assembleRelease
        # (Use the full build_apk.sh logic)
        
      - name: Upload APK
        uses: actions/upload-artifact@v4
        with:
          name: android-${{ matrix.abi }}
          path: apk/
          retention-days: 14
```

---

*See [Cross-Compilation](cross-compilation.md) for building Android from Linux, or [CLI Tool](reference/cli.md) for `soldatik3d apk` command.*