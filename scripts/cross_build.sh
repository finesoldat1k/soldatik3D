                                                        й#!/usr/bin/env bash
# Build the Soldatik3D engine and the demo game for the supported platforms.
#
# Usage:
#   ./cross_build.sh linux   [--release]
#   ./cross_build.sh windows [--release]              # MinGW cross-compile
#   ./cross_build.sh android [abi] [--release]        # NDK required
#   ./cross_build.sh all     [--release]
#
# Android ABIs: arm64-v8a (default), armeabi-v7a, x86_64, all.
#
# GitHub Actions is the canonical build environment (see .github/workflows).
# This script is a local convenience wrapper and intentionally builds the
# engine (runtime) and the game (demo) as separate Cargo invocations.

set -euo pipefail

# shaderc-sys 0.8 vendors CMake projects with pre-3.5 policy minimums; CMake 4
# refuses to configure them without this compatibility override.
export CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

log() { printf '\033[1;34m[build]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[build]\033[0m %s\n' "$*" >&2; }

TARGET="${1:-}"
if [[ -z "$TARGET" ]]; then
    echo "usage: $0 <linux|windows|android|all> [abi] [--release]" >&2
    exit 2
fi
shift

RELEASE=false
ABI="all"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) RELEASE=true ;;
        arm64-v8a | armeabi-v7a | x86_64 | all) ABI="$1" ;;
        *)
            echo "unknown argument: $1" >&2
            exit 2
            ;;
    esac
    shift
done

PROFILE="dev"
BUILD_DIR="debug"
PROFILE_ARGS=()
if [[ "$RELEASE" == true ]]; then
    PROFILE="release"
    BUILD_DIR="release"
    PROFILE_ARGS=(--release)
fi

build_engine_and_demo() {
    local out_dir="$1"
    local target="$2"
    shift 2

    local target_args=()
    local out="target/${BUILD_DIR}"
    if [[ -n "$target" ]]; then
        target_args=(--target "$target")
        out="target/${target}/${BUILD_DIR}"
    fi

    # The output directory is generated; clear it so a previous target/profile
    # cannot leave stale `demo`/`libdemo.so` files in a package.
    rm -rf "$out_dir"
    mkdir -p "$out_dir"

    log "engine -> runtime"
    cargo build --locked -p runtime "${PROFILE_ARGS[@]}" "${target_args[@]}" "$@"

    log "game -> demo"
    cargo build --locked -p demo "${PROFILE_ARGS[@]}" "${target_args[@]}" "$@"

    cp "$out/libruntime.rlib" "$out_dir/"
    if [[ "$target" == *android* ]]; then
        # Android loads the demo as a native-activity `cdylib`
        # (android.app.NativeActivity dlopen's libdemo.so and calls
        # android_main), not as a standalone executable.
        cp "$out/libdemo.so" "$out_dir/"
    elif [[ -f "$out/demo.exe" ]]; then
        cp "$out/demo.exe" "$out_dir/"
    else
        cp "$out/demo" "$out_dir/"
    fi
    log "artifacts -> $out_dir"
}

build_linux() {
    build_engine_and_demo "dist/linux-x86_64" "" --all-features
    # Copy required shared libraries for portable distribution
    local out_dir="dist/linux-x86_64"
    cp /usr/lib/x86_64-linux-gnu/libshaderc.so.1 "$out_dir/" 2>/dev/null || \
      cp /usr/lib/libshaderc.so.1 "$out_dir/" 2>/dev/null || true
    cp /usr/lib/x86_64-linux-gnu/libvulkan.so.1 "$out_dir/" 2>/dev/null || true
    log "Copied shared libraries to $out_dir"
}

build_windows() {
    if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
        echo "MinGW-w64 is required. Install it with: sudo apt-get install mingw-w64" >&2
        exit 1
    fi
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="${CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER:-x86_64-w64-mingw32-gcc}"
    # The cross build intentionally uses OpenGL only: shaderc's vendored C++
    # toolchain is not available for every MinGW installation. The native
    # Windows MSVC CI job builds both backends.
    build_engine_and_demo "dist/windows-x86_64" "x86_64-pc-windows-gnu" \
        --no-default-features --features opengl
}

android_linker() {
    local target="$1"
    local api="29"
    local ndk_bin="${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point at an Android NDK}/toolchains/llvm/prebuilt/linux-x86_64/bin"
    case "$target" in
        aarch64-linux-android) echo "$ndk_bin/aarch64-linux-android${api}-clang" ;;
        armv7-linux-androideabi) echo "$ndk_bin/armv7a-linux-androideabi${api}-clang" ;;
        x86_64-linux-android) echo "$ndk_bin/x86_64-linux-android${api}-clang" ;;
        *) echo "unsupported Android target: $target" >&2; exit 1 ;;
    esac
}

build_android() {
    local abi="$1"
    local targets=()
    case "$abi" in
        arm64-v8a) targets=(aarch64-linux-android) ;;
        armeabi-v7a) targets=(armv7-linux-androideabi) ;;
        x86_64) targets=(x86_64-linux-android) ;;
        all) targets=(aarch64-linux-android armv7-linux-androideabi) ;;
        *)
            echo "unknown Android ABI: $abi" >&2
            exit 2
            ;;
    esac

    : "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point at an Android NDK}"
    for target in "${targets[@]}"; do
        local linker env_name target_env abi_dir
        linker="$(android_linker "$target")"
        env_name="CARGO_TARGET_$(echo "$target" | tr 'a-z-' 'A-Z_')_LINKER"
        target_env="$(echo "$target" | tr 'a-z-' 'A-Z_')"
        case "$target" in
            aarch64-linux-android) abi_dir="arm64-v8a" ;;
            armv7-linux-androideabi) abi_dir="armeabi-v7a" ;;
            x86_64-linux-android) abi_dir="x86_64" ;;
        esac
        export "$env_name"="$linker"
        export "CC_${target_env}=$linker"
        export "AR_${target_env}=${linker%/*}/llvm-ar"
        log "Android $target (OpenGL ES)"
        build_engine_and_demo "dist/android-${abi_dir}/${target}" "$target" \
            --no-default-features --features opengl
    done
}

case "$TARGET" in
    linux) build_linux ;;
    windows) build_windows ;;
    android) build_android "$ABI" ;;
    all)
        build_linux
        if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
            build_windows
        else
            warn "MinGW-w64 not found; skipping the Windows cross build"
        fi
        if [[ -n "${ANDROID_NDK_HOME:-}" ]]; then
            build_android all
        else
            warn "ANDROID_NDK_HOME is not set; skipping the Android build"
        fi
        ;;
    *)
        echo "unknown target: $TARGET" >&2
        exit 2
        ;;
esac

log "done (${PROFILE})"
