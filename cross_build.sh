#!/usr/bin/env bash
# cross_build.sh - Cross-platform build script for soldatik3D
# Usage: ./cross_build.sh [target] [options]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Default values
TARGET="${1:-host}"
RELEASE=false
ALL_FEATURES=true
VERBOSE=false
RUN_TESTS=false
RUN_CLIPPY=true
CHECK_FMT=true

# Parse arguments
shift || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --release) RELEASE=true ;;
        --no-default-features) ALL_FEATURES=false ;;
        --verbose) VERBOSE=true ;;
        --test) RUN_TESTS=true ;;
        --no-clippy) RUN_CLIPPY=false ;;
        --no-fmt) CHECK_FMT=false ;;
        *) log_error "Unknown option: $1"; exit 1 ;;
    esac
    shift
done

# Platform detection
detect_host() {
    case "$(uname -s)" in
        Linux*) echo "linux" ;;
        Darwin*) echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *) echo "unknown" ;;
    esac
}

HOST_OS=$(detect_host)

# Target configurations - focused on Linux, Windows, Android
declare -A TARGET_CONFIG=(
    # Linux targets (priority)
    ["x86_64-unknown-linux-gnu"]="native|linux|gnu"
    ["x86_64-unknown-linux-musl"]="static|linux|musl"
    ["aarch64-unknown-linux-gnu"]="cross|linux|gnu"
    ["aarch64-unknown-linux-musl"]="cross|linux|musl"
    ["armv7-unknown-linux-gnueabihf"]="cross|linux|gnu"
    ["armv7-unknown-linux-musleabihf"]="cross|linux|musl"
    
    # Windows targets (priority)
    ["x86_64-pc-windows-gnu"]="cross|windows|gnu"
    ["x86_64-pc-windows-msvc"]="cross|windows|msvc"
    
    # Android targets (priority)
    ["aarch64-linux-android"]="cross|android|"
    ["armv7-linux-androideabi"]="cross|android|"
    ["x86_64-linux-android"]="cross|android|"
    
    # Special
    ["host"]="native|auto|"
)

# Get target info
get_target_info() {
    local target="$1"
    echo "${TARGET_CONFIG[$target]:-cross|unknown|}"
}

# Install target if needed
install_rust_target() {
    local target="$1"
    if [[ "$target" != "host" ]]; then
        log_info "Installing Rust target: $target"
        rustup target add "$target" || return 1
    fi
}

# Check dependencies for target
check_dependencies() {
    local target="$1"
    local info=$(get_target_info "$target")
    local type=$(echo "$info" | cut -d'|' -f1)
    local platform=$(echo "$info" | cut -d'|' -f2)
    local libc=$(echo "$info" | cut -d'|' -f3)

    case "$platform" in
        android)
            if ! command -v adb &> /dev/null; then
                log_warn "adb not found, Android deployment will not work"
            fi
            if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
                log_error "ANDROID_NDK_HOME not set. Run: rustup target add $target && export ANDROID_NDK_HOME=/path/to/ndk"
                return 1
            fi
            ;;
        windows)
            if [[ "$type" == "cross" ]] && ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
                log_error "MinGW-w64 not installed. Install: apt-get install gcc-mingw-w64-x86-64 (or equivalent)"
                return 1
            fi
            ;;
        linux)
            if [[ "$libc" == "musl" ]] && ! command -v musl-gcc &> /dev/null; then
                log_warn "musl-gcc not found, musl targets may fail"
            fi
            ;;
    esac
    return 0
}

# Build for target
build_target() {
    local target="$1"
    local cargo_args=()
    
    cargo_args+=(--workspace)
    
    if [[ "$ALL_FEATURES" == "true" ]]; then
        cargo_args+=(--all-features)
    else
        cargo_args+=(--no-default-features)
    fi
    
    if [[ "$RELEASE" == "true" ]]; then
        cargo_args+=(--release)
    fi
    
    if [[ "$VERBOSE" == "true" ]]; then
        cargo_args+=(--verbose)
    fi
    
    if [[ "$target" != "host" ]]; then
        cargo_args+=(--target "$target")
    fi
    
    log_info "Building for $target with args: ${cargo_args[*]}"
    
    # Set environment variables for cross-compilation
    export CARGO_TARGET_DIR="${PROJECT_ROOT}/target"
    
    case "$target" in
        *-linux-android)
            export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-/opt/android/ndk}"
            if [[ -z "$ANDROID_NDK_HOME" || ! -d "$ANDROID_NDK_HOME" ]]; then
                log_error "ANDROID_NDK_HOME not set or invalid"
                return 1
            fi
            ;;
        *-pc-windows-gnu)
            export CC=x86_64-w64-mingw32-gcc
            export CXX=x86_64-w64-mingw32-g++
            export AR=x86_64-w64-mingw32-ar
            ;;
    esac
    
    cargo build "${cargo_args[@]}"
}

# Run tests for target
test_target() {
    local target="$1"
    local cargo_args=()
    
    cargo_args+=(--workspace)
    
    if [[ "$ALL_FEATURES" == "true" ]]; then
        cargo_args+=(--all-features)
    fi
    
    if [[ "$RELEASE" == "true" ]]; then
        cargo_args+=(--release)
    fi
    
    if [[ "$VERBOSE" == "true" ]]; then
        cargo_args+=(--verbose)
    fi
    
    if [[ "$target" != "host" ]]; then
        cargo_args+=(--target "$target")
    fi
    
    cargo test "${cargo_args[@]}"
}

# Run clippy
run_clippy() {
    local target="$1"
    local cargo_args=()
    
    cargo_args+=(clippy --workspace --all-targets)
    
    if [[ "$ALL_FEATURES" == "true" ]]; then
        cargo_args+=(--all-features)
    else
        cargo_args+=(--no-default-features)
    fi
    
    if [[ "$target" != "host" ]]; then
        cargo_args+=(--target "$target")
    fi
    
    cargo "${cargo_args[@]}" -- -D warnings
}

# Check formatting
check_fmt() {
    cargo fmt --all -- --check
}

# Generate docs
generate_docs() {
    local target="$1"
    local cargo_args=()
    
    cargo_args+=(doc --workspace --no-deps)
    
    if [[ "$ALL_FEATURES" == "true" ]]; then
        cargo_args+=(--all-features)
    fi
    
    if [[ "$target" != "host" ]]; then
        cargo_args+=(--target "$target")
    fi
    
    cargo "${cargo_args[@]}"
}

# Main build function
main() {
    log_info "Starting cross-platform build for target: $TARGET"
    log_info "Host OS: $HOST_OS"
    log_info "Release: $RELEASE, All features: $ALL_FEATURES"
    
    # Check if target is supported
    if [[ -z "${TARGET_CONFIG[$TARGET]:-}" ]]; then
        log_error "Unsupported target: $TARGET"
        log_info "Supported targets:"
        for t in "${!TARGET_CONFIG[@]}"; do
            echo "  $t"
        done | sort
        exit 1
    fi
    
    # Check dependencies
    check_dependencies "$TARGET" || exit 1
    
    # Install Rust target
    install_rust_target "$TARGET" || exit 1
    
    # Check formatting
    if [[ "$CHECK_FMT" == "true" ]]; then
        log_info "Checking formatting..."
        check_fmt || { log_error "Formatting check failed. Run: cargo fmt --all"; exit 1; }
    fi
    
    # Run clippy
    if [[ "$RUN_CLIPPY" == "true" ]]; then
        log_info "Running clippy..."
        run_clippy "$TARGET" || exit 1
    fi
    
    # Build
    log_info "Building..."
    build_target "$TARGET" || exit 1
    
    # Run tests
    if [[ "$RUN_TESTS" == "true" ]]; then
        log_info "Running tests..."
        test_target "$TARGET" || exit 1
    fi
    
    # Generate docs (only for host or if explicitly requested)
    if [[ "$TARGET" == "host" ]]; then
        log_info "Generating documentation..."
        generate_docs "$TARGET" || exit 1
    fi
    
    log_success "Build completed successfully for $TARGET"
    
    # Show output location
    local out_dir="target"
    if [[ "$TARGET" != "host" ]]; then
        out_dir="target/$TARGET"
    fi
    if [[ "$RELEASE" == "true" ]]; then
        out_dir="$out_dir/release"
    else
        out_dir="$out_dir/debug"
    fi
    log_info "Artifacts in: $out_dir"
    
    # List binaries
    if [[ -d "$out_dir" ]]; then
        find "$out_dir" -maxdepth 1 -type f -executable -name "demo" 2>/dev/null | while read -r bin; do
            log_info "Binary: $bin"
            file "$bin"
        done
    fi
}

main "$@"