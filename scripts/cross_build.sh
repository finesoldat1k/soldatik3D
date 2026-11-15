#!/usr/bin/env bash
# Build the Soldatik3D engine and the demo game for the supported platforms.
#
# Usage:
#   ./scripts/cross_build.sh linux   [--release]
#   ./scripts/cross_build.sh windows [--release]              # MinGW cross-compile
#   ./scripts/cross_build.sh android [abi] [--release]        # NDK required
#   ./scripts/cross_build.sh all     [--release]
#
# Android ABIs: arm64-v8a, armeabi-v7a, x86_64, or all (default).
#
# GitHub Actions is the canonical build environment (see .github/workflows).
# This script is a local convenience wrapper and intentionally builds the
# engine (runtime) and the game (demo) as separate Cargo invocations.

set -euo pipefail

# shaderc-sys 0.8 vendors CMake projects with pre-3.5 policy minimums; CMake 4
# refuses to configure them without this compatibility override.
export CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

log() { printf '\033[1;34m[build]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[build]\033[0m %s\n' "$*" >&2; }
err() { printf '\033[1;31m[build]\033[0m %s\n' "$*" >&2; }

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
            err "unknown argument: $1"
            exit 2
            ;;
    esac
    shift
done

PROFILE="debug"
PROFILE_ARGS=()
if [[ "$RELEASE" == true ]]; then
    PROFILE="release"
    PROFILE_ARGS=(--release)
fi

build_engine_and_demo() {
    local out_dir="$1"
    local target="$2"
    shift 2

    local target_args=()
    local out="target/${PROFILE}"
    if [[ -n "$target" ]]; then
        target_args=(--target "$target")
        out="target/${target}/${PROFILE}"
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

    # Ship the CLI with desktop packages. Cross-target packages get a native
    # executable for that target, not the host binary.
    if [[ "$target" != *android* ]]; then
        log "tool -> s3d"
        if [[ -n "$target" ]]; then
            cargo build --locked -p s3d "${PROFILE_ARGS[@]}" --target "$target"
            cp "target/${target}/${PROFILE}/s3d.exe" "$out_dir/"
        else
            cargo build --locked -p s3d "${PROFILE_ARGS[@]}"
            cp "target/${PROFILE}/s3d" "$out_dir/"
        fi
    fi

    log "artifacts -> $out_dir"
}

copy_shared_library() {
    local destination="$1"
    local name="$2"
    shift 2
    for candidate in "$@"; do
        if [[ -e "$candidate" ]]; then
            cp -L "$candidate" "$destination/$name"
            return 0
        fi
    done
    return 1
}

build_linux() {
    # Bundle the demo's shared-library dependencies next to the executable and
    # set $ORIGIN rpath so the package is relocatable.
    local saved_rustflags="${RUSTFLAGS-}"
    export RUSTFLAGS="-C link-arg=-Wl,-rpath,\$ORIGIN -C link-arg=-Wl,--enable-new-dtags"
    build_engine_and_demo "dist/linux-x86_64" "" --all-features
    if [[ -n "$saved_rustflags" ]]; then
        export RUSTFLAGS="$saved_rustflags"
    else
        unset RUSTFLAGS
    fi

    local out_dir="dist/linux-x86_64"
    copy_shared_library "$out_dir" "libshaderc.so.1" \
        /usr/lib/x86_64-linux-gnu/libshaderc.so.1 \
        /usr/lib/libshaderc.so.1 \
        /usr/lib/x86_64-linux-gnu/libshaderc_shared.so.1 \
        /usr/lib/libshaderc_shared.so.1 || \
        warn "libshaderc not found; the demo may need a system shaderc at runtime"
    copy_shared_library "$out_dir" "libvulkan.so.1" \
        /usr/lib/x86_64-linux-gnu/libvulkan.so.1 \
        /usr/lib/libvulkan.so.1 || \
        warn "libvulkan not found; the demo will need a system Vulkan loader"
    log "Copied shared libraries to $out_dir"
}

build_windows() {
    if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
        err "MinGW-w64 is required. Install it with: sudo apt-get install mingw-w64"
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
    local host_tag
    case "$(uname -s)" in
        Linux*) host_tag="linux-x86_64" ;;
        Darwin*) host_tag="darwin-x86_64" ;;
        MINGW* | MSYS* | CYGWIN*) host_tag="windows-x86_64" ;;
        *)
            err "Unsupported host OS: $(uname -s)"
            exit 1
            ;;
    esac
    local ndk_bin="${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point at an Android NDK}/toolchains/llvm/prebuilt/${host_tag}/bin"
    case "$target" in
        aarch64-linux-android) echo "$ndk_bin/aarch64-linux-android${api}-clang" ;;
        armv7-linux-androideabi) echo "$ndk_bin/armv7a-linux-androideabi${api}-clang" ;;
        x86_64-linux-android) echo "$ndk_bin/x86_64-linux-android${api}-clang" ;;
        *)
            err "unsupported Android target: $target"
            exit 1
            ;;
    esac
}

build_android() {
    local abi="$1"
    local targets=()
    case "$abi" in
        arm64-v8a) targets=(aarch64-linux-android) ;;
        armeabi-v7a) targets=(armv7-linux-androideabi) ;;
        x86_64) targets=(x86_64-linux-android) ;;
        all) targets=(aarch64-linux-android armv7-linux-androideabi x86_64-linux-android) ;;
        *)
            err "unknown Android ABI: $abi"
            exit 2
            ;;
    esac

    : "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point at an Android NDK}"
    for target in "${targets[@]}"; do
        local linker env_name target_env abi_dir
        linker="$(android_linker "$target")"
        env_name="CARGO_TARGET_$(echo "$target" | tr 'a-z-' 'A-Z_')_LINKER"
        # cc-rs reads CC_<triple> / AR_<triple> with '-' replaced by '_' and
        # the triple kept lowercase.
        target_env="$(echo "$target" | tr '-' '_')"
        case "$target" in
            aarch64-linux-android) abi_dir="arm64-v8a" ;;
            armv7-linux-androideabi) abi_dir="armeabi-v7a" ;;
            x86_64-linux-android) abi_dir="x86_64" ;;
        esac
        export "$env_name"="$linker"
        export "CC_${target_env}=$linker"
        export "AR_${target_env}=${linker%/*}/llvm-ar"
        log "Android $target (OpenGL ES)"
        build_engine_and_demo "dist/android-${abi_dir}" "$target" \
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
        err "unknown target: $TARGET"
        exit 2
        ;;
esac

log "done (${PROFILE})"
