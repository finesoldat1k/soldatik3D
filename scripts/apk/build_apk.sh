#!/usr/bin/env bash
# Build the Soldatik3D demo APK locally.
#
# This is the local counterpart of the Android CI job. It builds the native
# demo library with the OpenGL ES backend and packages it into an APK using the
# Gradle project in scripts/apk/gradle (the source of truth for CI as well).
#
# Usage:
#   ./scripts/apk/build_apk.sh <abi> [--release]
#   ./scripts/apk/build_apk.sh arm64-v8a
#   ./scripts/apk/build_apk.sh armeabi-v7a --release
#
# Requirements:
#   - Rust target installed (rustup target add <target>)
#   - Android NDK r26c or newer (ANDROID_NDK_HOME, or SDK_ROOT/ndk/<version>)
#   - Android SDK with platform 34 and build-tools 34.0.0
#   - JDK 17+ (JAVA_HOME or java on PATH)
#
# Output: apk/app-<abi>-<debug|release>.apk
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT"

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
        *)
            err "Unknown argument: $1"
            exit 2
            ;;
    esac
    shift
done

case "$ABI" in
    arm64-v8a)
        RUST_TARGET="aarch64-linux-android"
        NDK_LINKER="aarch64-linux-android29-clang"
        LINKER_ENV="CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
        CC_ENV="CC_aarch64_linux_android"
        AR_ENV="AR_aarch64_linux_android"
        ;;
    armeabi-v7a)
        RUST_TARGET="armv7-linux-androideabi"
        NDK_LINKER="armv7a-linux-androideabi29-clang"
        LINKER_ENV="CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER"
        CC_ENV="CC_armv7_linux_androideabi"
        AR_ENV="AR_armv7_linux_androideabi"
        ;;
    x86_64)
        RUST_TARGET="x86_64-linux-android"
        NDK_LINKER="x86_64-linux-android29-clang"
        LINKER_ENV="CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER"
        CC_ENV="CC_x86_64_linux_android"
        AR_ENV="AR_x86_64_linux_android"
        ;;
    *)
        err "Unknown ABI: $ABI"
        exit 2
        ;;
esac

PROFILE="debug"
CARGO_PROFILE_ARGS=()
GRADLE_TASK="assembleDebug"
APK_PATH="app/build/outputs/apk/debug/app-debug.apk"
if [[ "$RELEASE" == true ]]; then
    PROFILE="release"
    CARGO_PROFILE_ARGS=(--release)
    GRADLE_TASK="assembleRelease"
    APK_PATH="app/build/outputs/apk/release/app-release.apk"
fi

# Resolve the NDK. Prefer an explicit ANDROID_NDK_HOME, then the newest NDK
# installed under the Android SDK.
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
    for sdk in "${ANDROID_SDK_ROOT:-}" "${ANDROID_HOME:-}" "$HOME/Android/Sdk" "/opt/android-sdk"; do
        [[ -n "$sdk" && -d "$sdk/ndk" ]] || continue
        ANDROID_NDK_HOME="$(find "$sdk/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
        [[ -n "$ANDROID_NDK_HOME" ]] && break
    done
fi
if [[ -z "${ANDROID_NDK_HOME:-}" || ! -d "$ANDROID_NDK_HOME" ]]; then
    err "Android NDK not found."
    err "Set ANDROID_NDK_HOME to your NDK path, e.g.:"
    err "  export ANDROID_NDK_HOME=\$HOME/android-ndk-r26c"
    exit 1
fi
export ANDROID_NDK_HOME

case "$(uname -s)" in
    Linux*) HOST_TAG="linux-x86_64" ;;
    Darwin*) HOST_TAG="darwin-x86_64" ;;
    MINGW* | MSYS* | CYGWIN*) HOST_TAG="windows-x86_64" ;;
    *)
        err "Unsupported host OS: $(uname -s)"
        exit 1
        ;;
esac

NDK_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG/bin"
if [[ "$HOST_TAG" == "windows-x86_64" && -x "$NDK_BIN/$NDK_LINKER.cmd" ]]; then
    NDK_LINKER="$NDK_LINKER.cmd"
fi
if [[ ! -x "$NDK_BIN/$NDK_LINKER" ]]; then
    err "NDK linker not found: $NDK_BIN/$NDK_LINKER"
    exit 1
fi

export "$LINKER_ENV=$NDK_BIN/$NDK_LINKER"
export "$CC_ENV=$NDK_BIN/$NDK_LINKER"
export "$AR_ENV=$NDK_BIN/llvm-ar"

# JDK is required by Gradle/AGP.
if ! command -v java >/dev/null 2>&1 && [[ -z "${JAVA_HOME:-}" ]]; then
    err "Java not found. Install JDK 17+ and set JAVA_HOME (or put java on PATH)."
    exit 1
fi

log "Building Soldatik3D demo for $ABI ($RUST_TARGET, $PROFILE)"

log "Building native library (libdemo.so)..."
cargo build --locked "${CARGO_PROFILE_ARGS[@]}" \
    -p demo \
    --no-default-features --features opengl \
    --target "$RUST_TARGET"

NATIVE_LIB="target/$RUST_TARGET/$PROFILE/libdemo.so"
if [[ ! -f "$NATIVE_LIB" ]]; then
    err "Native library not found: $NATIVE_LIB"
    exit 1
fi

log "Preparing Gradle project..."
GRADLE_DIR="$ROOT/target/apk-gradle"
TEMPLATE_DIR="$SCRIPT_DIR/gradle"
rm -rf "$GRADLE_DIR"
mkdir -p "$GRADLE_DIR"
cp -a "$TEMPLATE_DIR/." "$GRADLE_DIR/"

JNI_LIBS_DIR="$GRADLE_DIR/app/src/main/jniLibs/$ABI"
mkdir -p "$JNI_LIBS_DIR"
cp "$NATIVE_LIB" "$JNI_LIBS_DIR/libdemo.so"

# Values consumed by app/build.gradle.kts.
export SOLDATIK3D_ANDROID_LIB_NAME="demo"
export SOLDATIK3D_ANDROID_APP_ID="com.soldatik3d.demo"
export SOLDATIK3D_ANDROID_APP_NAME="Soldatik3D Demo"

if [[ -n "${ANDROID_SDK_ROOT:-}" ]]; then
    printf 'sdk.dir=%s\n' "$ANDROID_SDK_ROOT" > "$GRADLE_DIR/local.properties"
elif [[ -n "${ANDROID_HOME:-}" ]]; then
    printf 'sdk.dir=%s\n' "$ANDROID_HOME" > "$GRADLE_DIR/local.properties"
fi

log "Building APK with Gradle ($GRADLE_TASK)..."
cd "$GRADLE_DIR"
chmod +x gradlew
./gradlew --no-daemon "$GRADLE_TASK"

cd "$ROOT"
mkdir -p apk
OUTPUT_APK="$ROOT/apk/app-${ABI}-${PROFILE}.apk"
if [[ ! -f "$GRADLE_DIR/$APK_PATH" ]]; then
    err "APK not found at $GRADLE_DIR/$APK_PATH"
    exit 1
fi
cp "$GRADLE_DIR/$APK_PATH" "$OUTPUT_APK"

if command -v aapt2 >/dev/null 2>&1; then
    log "APK contents:"
    aapt2 dump badging "$OUTPUT_APK" 2>/dev/null | grep -E "package:|native-code|launchable-activity" || true
fi

log "Done! APK ready at: $OUTPUT_APK"
log "Install with: adb install -r $OUTPUT_APK"
