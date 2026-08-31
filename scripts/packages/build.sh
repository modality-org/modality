#!/usr/bin/env bash

# Build Modal Packages
# This script builds the modal packages

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
VERSION="${TIMESTAMP}-${GIT_COMMIT}"

# Default values
SKIP_JS=true
SKIP_WASM=false
SKIP_LINUX=false
FROM_DEV=false
CLEAN_BUILD=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
Build Modality Packages

USAGE:
    $0 [OPTIONS]

OPTIONS:
    --version VERSION       Package version (default: TIMESTAMP-GITCOMMIT)
    --from-dev             Reuse rust/target/debug/modal from build-dev.sh for this machine
    --skip-linux           Skip Linux cross-compile (keep an existing linux binary if present)
    --skip-wasm            Skip WASM package builds
    --skip-js              Skip JavaScript packages build
    --clean                Clean build directory before building
    -h, --help             Show this help message

EXAMPLES:
    $0                      # Build all packages
    $0 --clean              # Clean build and rebuild
    $0 --from-dev           # After build-dev.sh: skip recompiling the host CLI
    $0 --from-dev --skip-linux --skip-wasm
                            # Package the local debug CLI only (no compile)

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --from-dev)
            FROM_DEV=true
            shift
            ;;
        --skip-linux)
            SKIP_LINUX=true
            shift
            ;;
        --skip-wasm)
            SKIP_WASM=true
            shift
            ;;
        --skip-js)
            SKIP_JS=true
            shift
            ;;
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
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

# Show configuration
log_info "Starting Modality package build"
log_info "Project root: $PROJECT_ROOT"
log_info "Build directory: $BUILD_DIR"
log_info "Git branch: $GIT_BRANCH"
log_info "Git commit: $GIT_COMMIT"
log_info "Version: $VERSION"
if [[ "$FROM_DEV" == true ]]; then
    log_info "Host CLI: reuse debug binary from build-dev.sh"
fi

# Clean build directory if requested
if [[ "$CLEAN_BUILD" == true ]]; then
    log_info "Cleaning build directory..."
    rm -rf "$BUILD_DIR"
fi

# Create build directory
mkdir -p "$BUILD_DIR"

host_rust_target() {
    case "$(uname -s)-$(uname -m)" in
        Darwin-arm64|Darwin-aarch64) echo "aarch64-apple-darwin" ;;
        Darwin-x86_64) echo "x86_64-apple-darwin" ;;
        Linux-x86_64) echo "x86_64-unknown-linux-gnu" ;;
        Linux-aarch64|Linux-arm64) echo "aarch64-unknown-linux-gnu" ;;
        *) echo "" ;;
    esac
}

platform_for_target() {
    case "$1" in
        x86_64-unknown-linux-gnu) echo "linux-x86_64" ;;
        aarch64-unknown-linux-gnu) echo "linux-aarch64" ;;
        aarch64-apple-darwin) echo "darwin-aarch64" ;;
        x86_64-apple-darwin) echo "darwin-x86_64" ;;
        *) echo "" ;;
    esac
}

ensure_cross() {
    if ! command -v cross &> /dev/null; then
        log_info "Installing cross for cross-compilation..."
        cargo install cross
    fi
}

# Build packages function
build_packages() {
    log_info "Building Modality packages..."
    local host_target platform dest src
    host_target="$(host_rust_target)"
    
    # Define target platforms
    TARGETS=(
        "x86_64-unknown-linux-gnu"      # Linux x86_64
        "aarch64-apple-darwin"          # macOS Apple Silicon
    )
    
    # Build for each target platform
    log_info "Building Rust CLI for multiple platforms..."
    cd "$PROJECT_ROOT/rust"
    
    for target in "${TARGETS[@]}"; do
        platform="$(platform_for_target "$target")"
        dest="$BUILD_DIR/binaries/$platform/modal"
        mkdir -p "$(dirname "$dest")"
        
        if [[ "$SKIP_LINUX" == true && "$target" == *"linux"* ]]; then
            if [[ -f "$dest" ]]; then
                log_warning "Skipping $platform compile (keeping $dest)"
                continue
            fi
            log_error "No existing $dest. Run without --skip-linux, or a full build.sh first."
            exit 1
        fi
        
        if [[ "$FROM_DEV" == true && "$target" == "$host_target" ]]; then
            src="$PROJECT_ROOT/rust/target/debug/modal"
            if [[ ! -f "$src" ]]; then
                log_error "No debug CLI at $src"
                log_error "Run ./scripts/packages/build-dev.sh first, or omit --from-dev."
                exit 1
            fi
            log_info "Reusing debug CLI for $platform (from build-dev.sh)"
            cp "$src" "$dest"
            log_success "Packaged $platform without recompiling"
            continue
        fi
        
        log_info "Building for $target..."
        
        if [[ "$target" == "$host_target" ]]; then
            # Host build: no --target, so artifacts live in target/release/ and
            # subsequent package builds incrementally reuse that profile.
            MODAL_GIT_BRANCH="$GIT_BRANCH" MODAL_GIT_COMMIT="$GIT_COMMIT" \
                cargo build --release -p modal
            src="$PROJECT_ROOT/rust/target/release/modal"
        elif [[ "$target" == *"darwin"* ]] && [[ "$(uname -s)" == "Darwin" ]]; then
            rustup target add "$target" 2>/dev/null || true
            MODAL_GIT_BRANCH="$GIT_BRANCH" MODAL_GIT_COMMIT="$GIT_COMMIT" \
                cargo build --release -p modal --target "$target"
            src="$PROJECT_ROOT/rust/target/$target/release/modal"
        else
            ensure_cross
            MODAL_GIT_BRANCH="$GIT_BRANCH" \
            MODAL_GIT_COMMIT="$GIT_COMMIT" \
            AWS_LC_SYS_CMAKE_BUILDER="1" \
                cross build --release -p modal --target "$target"
            src="$PROJECT_ROOT/rust/target/$target/release/modal"
        fi
        
        cp "$src" "$dest"
        
        if [[ "$platform" != "windows-"* ]]; then
            strip "$dest" 2>/dev/null || true
        fi
        
        log_success "Built for $platform"
    done
    
    log_success "Rust CLI built successfully for all platforms"
    
    if [[ "$SKIP_WASM" == true ]]; then
        log_warning "Skipping WASM packages build"
    else
    # Build WASM packages
    log_info "Building WASM packages..."
    
    # Install wasm-pack if not available
    if ! command -v wasm-pack &> /dev/null; then
        log_info "Installing wasm-pack..."
        cargo install wasm-pack
    fi
    
    # Build modality-lang WASM package
    log_info "Building modality-lang WASM..."
    cd "$PROJECT_ROOT/rust/modality-lang"
    npm run build
    npm run build-node
    npm run build-bundler
    
    # Copy modality-lang WASM builds
    mkdir -p "$BUILD_DIR/wasm/modality-lang"
    cp -r dist "$BUILD_DIR/wasm/modality-lang/web"
    cp -r dist-node "$BUILD_DIR/wasm/modality-lang/node"
    cp -r dist-bundler "$BUILD_DIR/wasm/modality-lang/bundler"
    log_success "modality-lang WASM built"
    
    # Build modal-wasm-validation WASM package
    log_info "Building modal-wasm-validation WASM..."
    cd "$PROJECT_ROOT/rust/modal-wasm-validation"
    npm run build
    npm run build-node
    npm run build-bundler
    
    # Copy modal-wasm-validation WASM builds
    mkdir -p "$BUILD_DIR/wasm/modal-wasm-validation"
    cp -r dist "$BUILD_DIR/wasm/modal-wasm-validation/web"
    cp -r dist-node "$BUILD_DIR/wasm/modal-wasm-validation/node"
    cp -r dist-bundler "$BUILD_DIR/wasm/modal-wasm-validation/bundler"
    log_success "modal-wasm-validation WASM built"
    
    log_success "All WASM packages built successfully"
    fi
    
    # Build JavaScript packages (if not skipped)
    if [[ "$SKIP_JS" == false ]]; then
    log_info "Building JavaScript packages..."
    cd "$PROJECT_ROOT/js"
    
    # Install dependencies
    if command -v pnpm &> /dev/null; then
            log_info "Installing dependencies with pnpm..."
        pnpm install
    elif command -v npm &> /dev/null; then
            log_info "Installing dependencies with npm..."
        npm install
    else
        log_error "Neither pnpm nor npm found. Please install one of them."
        exit 1
    fi
    
    # Install rimraf globally if not available
    if ! command -v rimraf &> /dev/null; then
        log_info "Installing rimraf globally..."
        npm install -g rimraf
    fi
    
    # Build CLI package
    log_info "Building CLI package..."
    cd "$PROJECT_ROOT/js/packages/cli"
    if npm run build; then
        log_success "CLI package built successfully"
    else
        log_warning "CLI package build failed, but continuing..."
    fi
    
    log_success "JavaScript packages built successfully"
    else
        log_warning "Skipping JavaScript packages build"
    fi
    
    # Create package manifest
    log_info "Creating package manifest..."
    cat > "$BUILD_DIR/manifest.json" << EOF
{
    "version": "$VERSION",
    "timestamp": "$TIMESTAMP",
    "git_branch": "$GIT_BRANCH",
    "git_commit": "$GIT_COMMIT",
    "packages": {
        "binaries": {
            "linux-x86_64": {
                "name": "modal",
                "path": "binaries/linux-x86_64/modal",
                "platform": "linux",
                "arch": "x86_64"
            },
            "darwin-aarch64": {
                "name": "modal",
                "path": "binaries/darwin-aarch64/modal",
                "platform": "darwin",
                "arch": "aarch64"
            }
        },
        "wasm": {
            "modality-lang": {
                "web": "wasm/modality-lang/web/",
                "node": "wasm/modality-lang/node/",
                "bundler": "wasm/modality-lang/bundler/"
            },
            "modal-wasm-validation": {
                "web": "wasm/modal-wasm-validation/web/",
                "node": "wasm/modal-wasm-validation/node/",
                "bundler": "wasm/modal-wasm-validation/bundler/"
            }
        }
    }
}
EOF
    
    log_success "All packages built successfully!"
}

# Main execution
main() {
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Build packages
    build_packages
    
    log_success "Build process completed successfully!"
    
    # Show build summary
    log_info "Build summary:"
    log_info "  Build directory: $BUILD_DIR"
    log_info "  Total size: $(du -sh "$BUILD_DIR" | cut -f1)"
    log_info "  Git branch: $GIT_BRANCH"
    log_info "  Git commit: $GIT_COMMIT"
    log_info "  Version: $VERSION"
}

# Run main function
main "$@"
