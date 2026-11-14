#!/usr/bin/env bash
# Build APK for Soldatik3D game project
#
# Usage:
#   ./build_apk.sh <abi> [--release]
#   ./build_apk.sh arm64-v8a
#   ./build_apk.sh armeabi-v7a --release
#
# This script:
# 1. Builds the native library (libdemo.so) for the target ABI
# 2. Copies it to the Gradle project's jniLibs directory
# 3. Runs Gradle to build the APK
# 4. Outputs the APK to apk/app-<abi>-<debug|release>.apk

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

log() { printf '\033[1;34m[apk]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[apk]\033[0m %s\n' "$*" >&2; }
err() { printf '\033[1;31m[apk]\033[0m %s\n' "$*" >&2; }

ABI="${1:-}"
if [[ -z "$ABI" ]]; then
    echo "Usage: $0 <abi> [--release]" >&2
    echo "  abi: arm64-v8a | armeabi-v7a | x86_64" >&2
    exit 2
fi
shift

RELEASE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) RELEASE=true ;;
        *) warn "Unknown argument: $1" ;;
    esac
    shift
done

# Map ABI to Rust target
case "$ABI" in
    arm64-v8a) RUST_TARGET="aarch64-linux-android" ;;
    armeabi-v7a) RUST_TARGET="armv7-linux-androideabi" ;;
    x86_64) RUST_TARGET="x86_64-linux-android" ;;
    *)
        err "Unknown ABI: $ABI"
        exit 2
        ;;
esac

PROFILE="debug"
CARGO_PROFILE_ARGS=()
if [[ "$RELEASE" == true ]]; then
    PROFILE="release"
    CARGO_PROFILE_ARGS=(--release)
fi

# Check for Android NDK
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
    err "ANDROID_NDK_HOME not set. Please set it to your NDK path."
    err "Example: export ANDROID_NDK_HOME=\$HOME/android-ndk-r26c"
    exit 1
fi

# Check for Android SDK (for Gradle)
if [[ -z "${ANDROID_SDK_ROOT:-}" && -z "${ANDROID_HOME:-}" ]]; then
    warn "ANDROID_SDK_ROOT/ANDROID_HOME not set. Gradle will try to download SDK."
fi

# Check for Gradle
if ! command -v gradle &>/dev/null && [[ ! -f "./gradlew" ]]; then
    err "Gradle not found. Install Gradle or use the included gradlew wrapper."
    exit 1
fi

GRADLE_CMD="./gradlew"
if command -v gradle &>/dev/null; then
    GRADLE_CMD="gradle"
fi

log "Building for ABI: $ABI (Rust target: $RUST_TARGET, profile: $PROFILE)"

# Step 1: Build the native library
log "Building native library (libdemo.so)..."
export ANDROID_NDK_HOME

# Set up linker environment variables
NDK_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
case "$RUST_TARGET" in
    aarch64-linux-android)
        export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_BIN/aarch64-linux-android29-clang"
        export CC_aarch64_linux_android="$NDK_BIN/aarch64-linux-android29-clang"
        export AR_aarch64_linux_android="$NDK_BIN/llvm-ar"
        ;;
    armv7-linux-androideabi)
        export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$NDK_BIN/armv7a-linux-androideabi29-clang"
        export CC_armv7_linux_androideabi="$NDK_BIN/armv7a-linux-androideabi29-clang"
        export AR_armv7_linux_androideabi="$NDK_BIN/llvm-ar"
        ;;
    x86_64-linux-android)
        export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK_BIN/x86_64-linux-android29-clang"
        export CC_x86_64_linux_android="$NDK_BIN/x86_64-linux-android29-clang"
        export AR_x86_64_linux_android="$NDK_BIN/llvm-ar"
        ;;
esac

# Build with only OpenGL ES features for Android
cargo build --locked "${CARGO_PROFILE_ARGS[@]}" \
    --no-default-features --features opengl \
    --target "$RUST_TARGET"

NATIVE_LIB="target/$RUST_TARGET/$PROFILE/libdemo.so"
if [[ ! -f "$NATIVE_LIB" ]]; then
    err "Native library not found: $NATIVE_LIB"
    exit 1
fi

log "Native library built: $NATIVE_LIB"

# Step 2: Prepare Gradle project
log "Preparing Gradle project..."

GRADLE_DIR="android/gradle"
mkdir -p "$GRADLE_DIR"

# Source of truth for Gradle template location
# Can be overridden with GRADLE_TEMPLATE_DIR env var
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_DIR="${GRADLE_TEMPLATE_DIR:-$SCRIPT_DIR/gradle}"
if [[ -d "$TEMPLATE_DIR" ]]; then
    rsync -a --delete "$TEMPLATE_DIR/" "$GRADLE_DIR/"
else
    err "Gradle template not found at $TEMPLATE_DIR"
    exit 1
fi

# Copy native library to jniLibs
JNI_LIBS_DIR="$GRADLE_DIR/app/src/main/jniLibs/$ABI"
mkdir -p "$JNI_LIBS_DIR"
cp "$NATIVE_LIB" "$JNI_LIBS_DIR/libdemo.so"
log "Copied libdemo.so to $JNI_LIBS_DIR"

# Step 3: Build APK with Gradle
log "Building APK with Gradle..."

cd "$GRADLE_DIR"

# Make gradlew executable
chmod +x gradlew

# Build
if [[ "$RELEASE" == true ]]; then
    $GRADLE_CMD assembleRelease
    APK_PATH="app/build/outputs/apk/release/app-release.apk"
    OUTPUT_APK="../../apk/app-${ABI}-release.apk"
else
    $GRADLE_CMD assembleDebug
    APK_PATH="app/build/outputs/apk/debug/app-debug.apk"
    OUTPUT_APK="../../apk/app-${ABI}-debug.apk"
fi

cd ../..

# Step 4: Copy APK to output directory
mkdir -p apk
if [[ -f "$GRADLE_DIR/$APK_PATH" ]]; then
    cp "$GRADLE_DIR/$APK_PATH" "$OUTPUT_APK"
    log "APK built successfully: $OUTPUT_APK"
else
    err "APK not found at $GRADLE_DIR/$APK_PATH"
    exit 1
fi

# Step 5: Verify APK
if command -v aapt2 &>/dev/null || command -v aapt &>/dev/null; then
    log "Verifying APK contents..."
    AAPT_CMD="aapt2"
    command -v aapt2 &>/dev/null || AAPT_CMD="aapt"
    $AAPT_CMD list "$OUTPUT_APK" | grep -E "libdemo\.so|AndroidManifest" | head -10
fi

log "Done! APK ready at: $OUTPUT_APK"
log "Install with: adb install -r $OUTPUT_APK"