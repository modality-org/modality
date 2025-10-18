#!/usr/bin/env bash

# Build and Upload Modality Packages
# This script builds the modality packages and uploads them to AWS S3

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
S3_BUCKET=""
S3_PREFIX=""
AWS_REGION="us-east-1"
SKIP_BUILD=false
SKIP_UPLOAD=false
SKIP_JS=false
SKIP_CARGO_REGISTRY=false
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
Build and Upload Modality Packages

USAGE:
    $0 --bucket BUCKET [OPTIONS]

REQUIRED:
    --bucket BUCKET         S3 bucket name for uploads

OPTIONS:
    --prefix PREFIX         S3 prefix for uploads (default: empty)
    --version VERSION       Package version (default: TIMESTAMP-GITCOMMIT)
    --region REGION         AWS region (default: us-east-1)
    --skip-build           Skip building, only upload existing build
    --skip-upload          Skip uploading, only build
    --skip-js              Skip JavaScript packages build
    --skip-cargo-registry  Skip Cargo registry publishing
    --clean                Clean build directory before building
    -h, --help             Show this help message

ENVIRONMENT VARIABLES:
    AWS_PROFILE            AWS profile to use

EXAMPLES:
    $0 --bucket my-bucket
    $0 --bucket my-bucket --prefix modality-packages/
    $0 --bucket my-bucket --version custom-version --region us-west-2
    $0 --bucket my-bucket --skip-build  # Only upload existing build

S3 PATH STRUCTURE:
    Uploads will be organized as: s3://BUCKET/PREFIX/BRANCH/VERSION/
    Example: s3://my-bucket/modality-packages/main/20251018_143022-a1b2c3d/

CARGO REGISTRY:
    The script also publishes to a Cargo sparse registry for easy installation:
    cargo install --index sparse+https://packages.modality.org/BRANCH/VERSION/cargo-registry/index/ modality
    Registry URL: https://packages.modality.org

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --bucket)
            S3_BUCKET="$2"
            shift 2
            ;;
        --prefix)
            S3_PREFIX="$2"
            shift 2
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --region)
            AWS_REGION="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --skip-upload)
            SKIP_UPLOAD=true
            shift
            ;;
        --skip-js)
            SKIP_JS=true
            shift
            ;;
        --skip-cargo-registry)
            SKIP_CARGO_REGISTRY=true
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

# Validate required parameters
if [[ -z "$S3_BUCKET" ]]; then
    log_error "S3 bucket is required. Use --bucket BUCKET"
    show_help
    exit 1
fi

# Show configuration
if [[ "$SKIP_BUILD" == false ]]; then
    log_info "Starting Modality package build and upload process"
    log_info "Project root: $PROJECT_ROOT"
    log_info "Build directory: $BUILD_DIR"
    log_info "Git branch: $GIT_BRANCH"
    log_info "Git commit: $GIT_COMMIT"
    log_info "Version: $VERSION"
    log_info "S3 bucket: $S3_BUCKET"
    log_info "S3 prefix: $S3_PREFIX"
    log_info "S3 path: $S3_PREFIX$GIT_BRANCH/$VERSION/"
    log_info "AWS region: $AWS_REGION"
    if [[ "$SKIP_CARGO_REGISTRY" == false ]]; then
        log_info "Cargo registry: https://packages.modality.org"
    fi
fi

# Clean build directory if requested
if [[ "$CLEAN_BUILD" == true ]]; then
    log_info "Cleaning build directory..."
    rm -rf "$BUILD_DIR"
fi

# Create build directory
mkdir -p "$BUILD_DIR"

# Build packages function
build_packages() {
    log_info "Building Modality packages..."
    
    # Build Rust CLI
    log_info "Building Rust CLI..."
    cd "$PROJECT_ROOT/rust"
    cargo build --release
    
    # Copy Rust CLI binary and strip it to reduce size
    mkdir -p "$BUILD_DIR/bin"
    cp "$PROJECT_ROOT/rust/target/release/modality" "$BUILD_DIR/bin/"
    strip "$BUILD_DIR/bin/modality"
    log_success "Rust CLI built successfully"
    
    # Build WASM package
    log_info "Building WASM package..."
    cd "$PROJECT_ROOT/rust/modality-lang"
    
    # Install wasm-pack if not available
    if ! command -v wasm-pack &> /dev/null; then
        log_info "Installing wasm-pack..."
        cargo install wasm-pack
    fi
    
    # Build for different targets
    npm run build
    npm run build-node
    npm run build-bundler
    
    # Copy WASM builds
    mkdir -p "$BUILD_DIR/wasm"
    cp -r dist "$BUILD_DIR/wasm/web"
    cp -r dist-node "$BUILD_DIR/wasm/node"
    cp -r dist-bundler "$BUILD_DIR/wasm/bundler"
    log_success "WASM packages built successfully"
    
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
        "rust_cli": {
            "name": "modality",
            "path": "bin/modality",
            "platform": "linux-x86_64"
        },
        "wasm_web": {
            "name": "modality-lang-wasm",
            "path": "wasm/web/",
            "target": "web"
        },
        "wasm_node": {
            "name": "modality-lang-wasm",
            "path": "wasm/node/",
            "target": "nodejs"
        },
        "wasm_bundler": {
            "name": "modality-lang-wasm",
            "path": "wasm/bundler/",
            "target": "bundler"
        }
    }
}
EOF
    
    # Create index.html files for S3 browsing
    log_info "Creating main index.html for S3 browsing..."
    cat > "$BUILD_DIR/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Modality Packages - $VERSION</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        h1 { color: #333; }
        .package { background: #f5f5f5; padding: 10px; margin: 10px 0; border-radius: 5px; }
        code { background: #eee; padding: 2px 4px; border-radius: 3px; }
    </style>
</head>
<body>
    <h1>Modality Packages</h1>
    <p>Version: <code>$VERSION</code></p>
    <p>Branch: <code>$GIT_BRANCH</code></p>
    <p>Commit: <code>$GIT_COMMIT</code></p>
    <p>Built: <code>$(date)</code></p>
    
    <h2>Available Packages</h2>
    <ul>
        <li><a href="bin/">Rust CLI Binary</a></li>
        <li><a href="wasm/">WebAssembly Packages</a></li>
        <li><a href="cargo-registry/">Cargo Registry</a></li>
        <li><a href="manifest.json">Package Manifest</a></li>
    </ul>
    
    <h2>Installation</h2>
    <h3>Rust CLI</h3>
    <pre>cargo install --index sparse+https://packages.modality.org/$GIT_BRANCH/$VERSION/cargo-registry/index/ modality</pre>
    
    <h3>Registry Configuration</h3>
    <p>Or add to <code>~/.cargo/config.toml</code>:</p>
    <pre>[registries.modality]
index = "sparse+https://packages.modality.org/$GIT_BRANCH/$VERSION/cargo-registry/index/"</pre>
    <p>Then install with:</p>
    <pre>cargo install --registry modality modality</pre>
</body>
</html>
EOF
    
    # Create index.html for the bin directory
    cat > "$BUILD_DIR/bin/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Rust CLI Binary</title>
</head>
<body>
    <h1>Rust CLI Binary</h1>
    <p>Platform: linux-x86_64</p>
    <ul>
        <li><a href="modality">modality</a> - Main CLI binary</li>
    </ul>
</body>
</html>
EOF
    
    # Create index.html for the wasm directory
    cat > "$BUILD_DIR/wasm/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>WebAssembly Packages</title>
</head>
<body>
    <h1>WebAssembly Packages</h1>
    <p>Modality language parser compiled to WebAssembly</p>
    <ul>
        <li><a href="web/">Web Target</a> - For browsers</li>
        <li><a href="node/">Node.js Target</a> - For Node.js</li>
        <li><a href="bundler/">Bundler Target</a> - For webpack/rollup</li>
    </ul>
</body>
</html>
EOF
    
    log_success "All packages built successfully!"
}

# Upload to S3 function
upload_to_s3() {
    log_info "Uploading packages to S3..."
    
    # Handle S3 prefix - only add trailing slash if prefix is not empty and not just "/"
    if [[ -n "$S3_PREFIX" && "$S3_PREFIX" != "/" && ! "$S3_PREFIX" =~ /$ ]]; then
        S3_PREFIX="${S3_PREFIX}/"
    fi
    
    # Create the structured path: rust/$GIT_BRANCH/$VERSION/
    S3_PATH="$S3_PREFIX$GIT_BRANCH/$VERSION/"
    
    # Upload build directory
    aws s3 sync "$BUILD_DIR" "s3://$S3_BUCKET/$S3_PATH" \
        --region "$AWS_REGION" \
        --exclude "*.DS_Store" \
        --exclude "*.git*" \
        --exclude "node_modules/*"
    
    # Upload latest symlink for the branch
    aws s3 cp "s3://$S3_BUCKET/$S3_PATH" "s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/latest/" \
        --recursive \
        --region "$AWS_REGION"
    
    log_success "Packages uploaded successfully!"
    log_info "S3 URLs:"
    log_info "  Version: s3://$S3_BUCKET/$S3_PATH"
    log_info "  Latest:  s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/latest/"
    log_info "  Branch:  $GIT_BRANCH"
    log_info "  Commit:  $GIT_COMMIT"
}

# Cargo registry publishing function
publish_to_cargo_registry() {
    log_info "Publishing to Cargo registry..."
    
    # Handle S3 prefix - only add trailing slash if prefix is not empty and not just "/"
    if [[ -n "$S3_PREFIX" && "$S3_PREFIX" != "/" && ! "$S3_PREFIX" =~ /$ ]]; then
        S3_PREFIX="${S3_PREFIX}/"
    fi
    
    # Create the structured path for Cargo registry
    CARGO_REGISTRY_PATH="$S3_PREFIX$GIT_BRANCH/$VERSION/cargo-registry/"
    
    # Build registry using dedicated script
    log_info "Building registry with dedicated script..."
    "$PROJECT_ROOT/sites/packages.modality.org/build-registry.sh"
    
    # Upload registry to S3
    log_info "Uploading registry to S3..."
    aws s3 sync "$PROJECT_ROOT/sites/packages.modality.org/registry/" "s3://$S3_BUCKET/$CARGO_REGISTRY_PATH" \
        --region "$AWS_REGION" \
        --exclude "*.DS_Store" \
        --exclude "*.git*" \
        --delete
    
    # Update the latest symlink with the new registry structure
    log_info "Updating latest symlink with new registry structure..."
    aws s3 sync "s3://$S3_BUCKET/$CARGO_REGISTRY_PATH" "s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/latest/cargo-registry/" \
        --region "$AWS_REGION" \
        --delete
    
    # Create registry config
    REGISTRY_CONFIG_URL="https://packages.modality.org"
    REGISTRY_INDEX_URL="$REGISTRY_CONFIG_URL/$GIT_BRANCH/$VERSION/cargo-registry/index"
    
    log_success "Cargo registry published successfully!"
    log_info "Registry URLs:"
    log_info "  Registry config: $REGISTRY_CONFIG_URL"
    log_info "  Index URL: $REGISTRY_INDEX_URL"
    
    # Create registry configuration file
    cat > "$BUILD_DIR/cargo-registry-config.toml" << EOF
[registries.modality]
index = "$REGISTRY_INDEX_URL"
EOF
    
    log_info "To use this registry, add to ~/.cargo/config.toml:"
    log_info "  [registries.modality]"
    log_info "  index = \"$REGISTRY_INDEX_URL\""
    log_info ""
    log_info "Then install with:"
    log_info "  cargo install --index sparse+http://packages.modality.org/$GIT_BRANCH/$VERSION/cargo-registry/index/ modality"
}

# Main execution
main() {
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Build packages if not skipped
    if [[ "$SKIP_BUILD" == false ]]; then
        build_packages
    else
        log_warning "Skipping build step"
        if [[ ! -d "$BUILD_DIR" ]]; then
            log_error "Build directory does not exist. Cannot skip build."
            exit 1
        fi
    fi
    
    # Upload to S3 if not skipped
    if [[ "$SKIP_UPLOAD" == false ]]; then
        upload_to_s3
    else
        log_warning "Skipping upload step"
    fi
    
    # Publish to Cargo registry if not skipped
    if [[ "$SKIP_CARGO_REGISTRY" == false ]]; then
        publish_to_cargo_registry
    else
        log_warning "Skipping Cargo registry publishing"
    fi
    
    log_success "Build and upload process completed successfully!"
    
    # Show build summary
    log_info "Build summary:"
    log_info "  Build directory: $BUILD_DIR"
    log_info "  Total size: $(du -sh "$BUILD_DIR" | cut -f1)"
    log_info "  Git branch: $GIT_BRANCH"
    log_info "  Git commit: $GIT_COMMIT"
    log_info "  Version: $VERSION"
    log_info "  S3 location: s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/$VERSION/"
}

# Run main function
main "$@"
