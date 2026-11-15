# Android Development

Soldatik3D games are packaged as a native-activity Android app. There is no
Java or Kotlin gameplay code: the Rust crate is compiled as a `cdylib`, Gradle
packages it into the APK, and `android.app.NativeActivity` loads it and calls
the exported `android_main` function.

Android builds use **OpenGL ES 3.0 only**. The Vulkan backend and its
dependencies are target-gated out by `cfg(not(target_os = "android"))`, so the
`vulkan` Cargo feature is not compiled for Android targets.

## Requirements

| Component | Version |
|-----------|---------|
| JDK | 17 |
| Android SDK platform | 34 |
| Android build-tools | 34.0.0 |
| Android NDK | r26c or newer |
| Gradle | 8.10.2 (downloaded by the wrapper) |
| Android Gradle Plugin | 8.7.3 |
| `minSdk` / `targetSdk` | 29 / 34 |

Install the Rust targets for the ABIs you need:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

| APK ABI | Rust target | NDK linker |
|---------|-------------|------------|
| `arm64-v8a` | `aarch64-linux-android` | `aarch64-linux-android29-clang` |
| `armeabi-v7a` | `armv7-linux-androideabi` | `armv7a-linux-androideabi29-clang` |
| `x86_64` | `x86_64-linux-android` | `x86_64-linux-android29-clang` |

`x86_64` is intended for emulators. Most devices use `arm64-v8a`.

Install SDK components:

```bash
export ANDROID_SDK_ROOT=$HOME/Android/Sdk
"$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager" \
  "platforms;android-34" "build-tools;34.0.0" "platform-tools"
```

## Environment variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `JAVA_HOME` | yes | JDK 17 used by Gradle |
| `ANDROID_SDK_ROOT` or `ANDROID_HOME` | yes | SDK location; also written to `local.properties` |
| `ANDROID_NDK_HOME` | recommended | NDK root. If unset, the build scripts look for the newest NDK under `$ANDROID_SDK_ROOT/ndk` |
| `GRADLE_USER_HOME` | optional | Gradle cache location |
| `SOLDATIK3D_ANDROID_LIB_NAME` | set by script | Native library name without the `lib` prefix and `.so` suffix |
| `SOLDATIK3D_ANDROID_APP_ID` | set by script | Android application ID |
| `SOLDATIK3D_ANDROID_APP_NAME` | set by script | Display name (`app_name` string resource) |
| `SOLDATIK3D_KEYSTORE` | optional | Path to a release keystore |
| `SOLDATIK3D_KEYSTORE_PASSWORD` | optional | Keystore password |
| `SOLDATIK3D_KEY_ALIAS` | optional | Key alias |
| `SOLDATIK3D_KEY_PASSWORD` | optional | Key password |

The `SOLDATIK3D_ANDROID_*` values are normally exported by `build_apk.sh`.
They exist so the same Gradle project can package the engine demo and any
generated game.

## Gradle projects

There are two identical copies of the Gradle project:

- `scripts/apk/gradle` — canonical project used by `scripts/apk/build_apk.sh`
  and the CI workflow.
- `templates/game/android/gradle` — copied into every project created by
  `s3d new`.

If you change the manifest, resources, wrapper or Gradle scripts, update both.
Run `diff -r scripts/apk/gradle templates/game/android/gradle --exclude=build
--exclude=.gradle --exclude=jniLibs --exclude=local.properties` to confirm they
match.

The manifest uses the `${soldatik3DLibName}` placeholder:

```xml
<activity
    android:name="android.app.NativeActivity"
    android:configChanges="orientation|keyboardHidden|screenSize"
    android:exported="true">
    <meta-data
        android:name="android.app.lib_name"
        android:value="${soldatik3DLibName}" />
</activity>
```

## Building the engine demo APK

```bash
export JAVA_HOME=/path/to/jdk-17
export ANDROID_SDK_ROOT=$HOME/Android/Sdk
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c

./scripts/apk/build_apk.sh arm64-v8a
./scripts/apk/build_apk.sh arm64-v8a --release
```

The script:

1. resolves the NDK and NDK linker for the ABI,
2. builds `demo` with `--no-default-features --features opengl`,
3. copies `libdemo.so` into
   `scripts/apk/gradle/app/src/main/jniLibs/<abi>/`,
4. exports the `SOLDATIK3D_ANDROID_*` variables,
5. runs `./gradlew --no-daemon assembleDebug` or `assembleRelease`,
6. copies the APK to `apk/app-<abi>-<debug|release>.apk`.

Inspect the result:

```bash
unzip -l apk/app-arm64-v8a-debug.apk | grep lib/
# lib/arm64-v8a/libdemo.so

$ANDROID_SDK_ROOT/build-tools/34.0.0/aapt2 dump badging \
  apk/app-arm64-v8a-debug.apk | grep -E 'package:|native-code|sdkVersion|targetSdkVersion'
```

## Building a game APK

Every project created by `s3d new` contains its own `android/gradle` and a
`build_apk.sh` at the project root:

```bash
cd my_game
export ANDROID_NDK_HOME=$HOME/android-ndk-r26c
export ANDROID_SDK_ROOT=$HOME/Android/Sdk

./build_apk.sh arm64-v8a
./build_apk.sh arm64-v8a --release

# Equivalent:
s3d apk arm64-v8a --release
```

The generated `build_apk.sh` derives the library name from the Cargo package
name (`my_game` → `libmy_game.so`) and the application ID from
`com.soldatik3d.<package>`. To change them, edit the exports near the end of
the script.

Output: `my_game/apk/app-<abi>-<debug|release>.apk`.

Install and run:

```bash
adb install -r apk/app-arm64-v8a-debug.apk
adb shell am start -n com.soldatik3d.my_game/android.app.NativeActivity
adb logcat -s Soldatik3D RustStdoutStderr
```

## Assets

On Android, game assets must live inside the APK. `build.rs` in the generated
project copies `assets/` into `android/gradle/app/src/main/assets` before the
Rust compilation, and Gradle packages that directory. Desktop builds read
assets relative to the working directory, so no copy happens there.

Shader source files are not loaded from disk at runtime: both backends embed
their GLSL at compile time.

## Signing

Release APKs are signed with the standard Android debug key unless a keystore is
configured. This makes every release APK installable out of the box, but it is
not suitable for Play Store distribution.

To sign with your own key:

```bash
export SOLDATIK3D_KEYSTORE=$HOME/keys/release.jks
export SOLDATIK3D_KEYSTORE_PASSWORD=...
export SOLDATIK3D_KEY_ALIAS=release
export SOLDATIK3D_KEY_PASSWORD=...

./build_apk.sh arm64-v8a --release
```

Verify the signature:

```bash
$ANDROID_SDK_ROOT/build-tools/34.0.0/apksigner verify --print-certs \
  apk/app-arm64-v8a-release.apk
```

`s3d apk --sign` currently prints a notice; signing is controlled by the
environment variables above and happens inside Gradle.

## CI and release APKs

The Android CI job runs on every push and pull request:

1. installs JDK 17, SDK 34 and NDK r26c,
2. adds the Android Rust targets,
3. copies `scripts/apk/gradle` to a temporary build directory,
4. builds the demo with the OpenGL ES backend,
5. runs `assembleDebug` and verifies the APK contains
   `lib/arm64-v8a/libdemo.so`,
6. uploads `app-arm64-v8a-debug.apk` as a workflow artifact.

The release workflow builds `assembleRelease` for `arm64-v8a` and
`armeabi-v7a` and attaches both APKs directly to the GitHub Release.

## Troubleshooting

| Symptom | Cause and fix |
|---------|---------------|
| `Android NDK not found` | Set `ANDROID_NDK_HOME`, or install an NDK under `$ANDROID_SDK_ROOT/ndk` |
| `SDK location not found` | Set `ANDROID_SDK_ROOT`/`ANDROID_HOME`; the script writes `local.properties` |
| `Could not find or load main class "-Xmx64m"` | Broken or CRLF `gradlew`; use the bundled wrapper and `sh ./gradlew` |
| `error: attribute android:cx not found` | Launcher icons use unsupported vector elements; only `<path>` is portable |
| App installs but exits immediately | Check `adb logcat` for a missing library or renderer initialization error |
| `lib<name>.so` not found in the APK | The Cargo `[lib] name` and `SOLDATIK3D_ANDROID_LIB_NAME` differ |
| Gradle cannot download the distribution | The wrapper needs network access to `services.gradle.org`; set a proxy or pre-populate `GRADLE_USER_HOME` |

## Next steps

- [Cross-compilation](cross-compilation.md)
- [Configuration](../reference/configuration.md)
- [CI and releases](../CI.md)
