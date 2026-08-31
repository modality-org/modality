#!/usr/bin/env bash

# Build the Modal CLI for the local host only (debug, no cross-compilation).
# For a fast developer rebuild/test cycle. Use build.sh for multi-platform packages.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
    cat << EOF
Build Modal CLI for the local system (debug)

USAGE:
    $0 [OPTIONS]

Builds only the native debug \`modal\` binary. Skips Linux cross-compilation,
WASM, and JavaScript packages. Output is rust/target/debug/modal, also copied
into build/binaries/<platform>/ for a stable local path.

OPTIONS:
    -h, --help             Show this help message

EXAMPLES:
    $0
    export PATH="\$PWD/rust/target/debug:\$PATH"
    modal --version

To package this binary without compiling again:
    ./scripts/packages/build.sh --from-dev --skip-linux --skip-wasm

EOF
}

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                arm64|aarch64) echo "darwin-aarch64" ;;
                x86_64) echo "darwin-x86_64" ;;
                *)
                    log_error "Unsupported macOS architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64) echo "linux-x86_64" ;;
                aarch64|arm64) echo "linux-aarch64" ;;
                *)
                    log_error "Unsupported Linux architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        *)
            log_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

platform="$(detect_platform)"
binary_src="$PROJECT_ROOT/rust/target/debug/modal"
binary_dest="$BUILD_DIR/binaries/$platform/modal"

log_info "Starting local Modal CLI build (debug)"
log_info "Project root: $PROJECT_ROOT"
log_info "Platform: $platform"
log_info "Git branch: $GIT_BRANCH"
log_info "Git commit: $GIT_COMMIT"

cd "$PROJECT_ROOT/rust"
MODAL_GIT_BRANCH="$GIT_BRANCH" MODAL_GIT_COMMIT="$GIT_COMMIT" cargo build -p modal

mkdir -p "$(dirname "$binary_dest")"
cp "$binary_src" "$binary_dest"

log_success "Local debug build complete"
log_info "  Binary: $binary_src"
log_info "  Copied: $binary_dest"
log_info "  Run with:"
log_info "    export PATH=\"$PROJECT_ROOT/rust/target/debug:\$PATH\""
log_info "    modal --version"
