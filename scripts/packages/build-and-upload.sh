#!/usr/bin/env bash

# Build and Upload Modal Packages
# This script builds the modal packages and uploads them to AWS S3

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
VERSION="${TIMESTAMP}-${GIT_COMMIT}"

# Validate allowed branches
ALLOWED_BRANCHES=("mainnet" "testnet")
if [[ ! " ${ALLOWED_BRANCHES[@]} " =~ " ${GIT_BRANCH} " ]]; then
    echo -e "\033[0;31m[ERROR]\033[0m Branch '$GIT_BRANCH' is not allowed. Allowed branches: ${ALLOWED_BRANCHES[*]}"
    echo -e "\033[0;31m[ERROR]\033[0m Please switch to one of the allowed branches before running this script."
    exit 1
fi

# Default values
S3_BUCKET="get.modal.money-content"
S3_PREFIX=""
AWS_REGION="us-east-1"
SKIP_BUILD=false
SKIP_UPLOAD=false
SKIP_JS=true
SKIP_CARGO_REGISTRY=true
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
    $0 [OPTIONS]

OPTIONS:
    --bucket BUCKET         S3 bucket name for uploads (default: get.modal.money)
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
    $0                                    # Uses default bucket: get.modal.money
    $0 --bucket my-bucket                 # Use custom bucket
    $0 --prefix modal-packages/           # Add prefix to path
    $0 --version custom-version --region us-west-2
    $0 --skip-build                       # Only upload existing build

S3 PATH STRUCTURE:
    Uploads will be organized as: s3://BUCKET/PREFIX/BRANCH/VERSION/
    Default: s3://get.modal.money/BRANCH/VERSION/
    Example: s3://get.modal.money/testnet/20251018_143022-a1b2c3d/

CARGO REGISTRY:
    The script also publishes to a Cargo sparse registry for easy installation:
    cargo install --index sparse+https://get.modal.money/BRANCH/VERSION/cargo-registry/index/ modal
    Registry URL: https://get.modal.money

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
            SKIP_BUILD=false
            shift
            ;;
        --skip-upload)
            SKIP_UPLOAD=false
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
        log_info "Cargo registry: https://get.modal.money"
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
    
    # Install cross if not available
    if ! command -v cross &> /dev/null; then
        log_info "Installing cross for cross-compilation..."
        cargo install cross
    fi
    
    # Define target platforms
    TARGETS=(
        "x86_64-unknown-linux-gnu"      # Linux x86_64
        "aarch64-apple-darwin"          # macOS Apple Silicon
    )
    
    # Build for each target platform
    log_info "Building Rust CLI for multiple platforms..."
    cd "$PROJECT_ROOT/rust"
    
    for target in "${TARGETS[@]}"; do
        log_info "Building for $target..."
        
        # Use cross for Linux and Windows, regular cargo for macOS (if on macOS)
        if [[ "$target" == *"darwin"* ]] && [[ "$(uname -s)" == "Darwin" ]]; then
            # On macOS, use native cargo for macOS targets
            rustup target add "$target" 2>/dev/null || true
            MODAL_GIT_BRANCH="$GIT_BRANCH" MODAL_GIT_COMMIT="$GIT_COMMIT" cargo build --release --target "$target"
        else
            # Use cross for other platforms with static OpenSSL
            MODAL_GIT_BRANCH="$GIT_BRANCH" MODAL_GIT_COMMIT="$GIT_COMMIT" cross build --release --target "$target"
        fi
        
        # Determine platform name and binary extension
        case "$target" in
            x86_64-unknown-linux-gnu)
                platform="linux-x86_64"
                binary_name="modal"
                ;;
            aarch64-apple-darwin)
                platform="darwin-aarch64"
                binary_name="modal"
                ;;
        esac
        
        # Copy binary to platform-specific directory
        mkdir -p "$BUILD_DIR/binaries/$platform"
        cp "$PROJECT_ROOT/rust/target/$target/release/$binary_name" "$BUILD_DIR/binaries/$platform/"
        
        # Strip binary to reduce size (except Windows)
        if [[ "$platform" != "windows-"* ]]; then
            strip "$BUILD_DIR/binaries/$platform/$binary_name" 2>/dev/null || true
        fi
        
        log_success "Built for $platform"
    done
    
    log_success "Rust CLI built successfully for all platforms"
    
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
    
    # Create install script for users
    log_info "Creating install script..."
    cat > "$BUILD_DIR/install.sh" << 'INSTALL_SCRIPT_EOF'
#!/usr/bin/env bash
# Modality Installation Script
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { printf "${BLUE}[INFO]${NC} %s\n" "$1"; }
log_success() { printf "${GREEN}[SUCCESS]${NC} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${NC} %s\n" "$1"; }

# Detect platform
detect_platform() {
    local os="$(uname -s)"
    local arch="$(uname -m)"
    
    case "$os" in
        Linux*)
            case "$arch" in
                x86_64) echo "linux-x86_64" ;;
                aarch64|arm64) echo "linux-aarch64" ;;
                *) log_error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        Darwin*)
            case "$arch" in
                x86_64) echo "darwin-x86_64" ;;
                arm64) echo "darwin-aarch64" ;;
                *) log_error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "windows-x86_64"
            ;;
        *)
            log_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

# Installation
PLATFORM=$(detect_platform)
BASE_URL="${MODALITY_INSTALL_URL:-http://get.modal.money/BRANCH/VERSION}"
INSTALL_DIR="${MODALITY_INSTALL_DIR:-$HOME/.modality/bin}"
BINARY_NAME="modal"

if [ "$PLATFORM" = "windows-x86_64" ]; then
    BINARY_NAME="modal.exe"
fi

log_info "Detected platform: $PLATFORM"
log_info "Installing to: $INSTALL_DIR"

# Create install directory
mkdir -p "$INSTALL_DIR"

# Download binary
BINARY_URL="$BASE_URL/binaries/$PLATFORM/$BINARY_NAME"
log_info "Downloading from: $BINARY_URL"

if command -v curl > /dev/null 2>&1; then
    curl -fsSL "$BINARY_URL" -o "$INSTALL_DIR/$BINARY_NAME"
elif command -v wget > /dev/null 2>&1; then
    wget -q "$BINARY_URL" -O "$INSTALL_DIR/$BINARY_NAME"
else
    log_error "Neither curl nor wget found. Please install one of them."
    exit 1
fi

# Make executable
chmod +x "$INSTALL_DIR/$BINARY_NAME"

log_success "Modality installed successfully!"
log_info "Binary location: $INSTALL_DIR/$BINARY_NAME"

# Check if in PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        # Already in PATH
        ;;
    *)
        printf "\n"
        log_info "To use modal, add it to your PATH:"
        log_info "  export PATH=\"\$PATH:$INSTALL_DIR\""
        printf "\n"
        log_info "Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
        ;;
esac

# Test installation
if command -v modal > /dev/null 2>&1 || [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
    log_success "Installation verified!"
else
    log_error "Installation failed. Binary not found or not executable."
    exit 1
fi
INSTALL_SCRIPT_EOF
    chmod +x "$BUILD_DIR/install.sh"
    
    # Replace placeholders in install script
    sed -i.bak "s|BRANCH|$GIT_BRANCH|g" "$BUILD_DIR/install.sh"
    sed -i.bak "s|VERSION|$VERSION|g" "$BUILD_DIR/install.sh"
    rm "$BUILD_DIR/install.sh.bak"
    
    # Create index.html files for S3 browsing
    log_info "Creating main index.html for S3 browsing..."
    cat > "$BUILD_DIR/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Modality Packages - $VERSION</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }
        h1 { color: #333; }
        h2 { color: #555; margin-top: 30px; }
        .package { background: #f5f5f5; padding: 10px; margin: 10px 0; border-radius: 5px; }
        code { background: #eee; padding: 2px 4px; border-radius: 3px; font-family: monospace; }
        pre { background: #f5f5f5; padding: 15px; border-radius: 5px; overflow-x: auto; }
        .platform { display: inline-block; background: #007bff; color: white; padding: 3px 8px; border-radius: 3px; margin: 2px; font-size: 0.9em; }
    </style>
</head>
<body>
    <h1>Modality Packages</h1>
    <p>Version: <code>$VERSION</code></p>
    <p>Branch: <code>$GIT_BRANCH</code></p>
    <p>Commit: <code>$GIT_COMMIT</code></p>
    <p>Built: <code>$(date)</code></p>
    
    <h2>Quick Installation</h2>
    <pre>curl -fsSL http://get.modal.money/$GIT_BRANCH/$VERSION/install.sh | sh</pre>
    
    <h2>Available Binaries</h2>
    <div class="package">
        <h3>Pre-built Binaries</h3>
        <p>Download directly for your platform:</p>
        <ul>
            <li><span class="platform">Linux x86_64</span> <a href="binaries/linux-x86_64/modal">Download</a></li>
            <li><span class="platform">macOS Apple Silicon</span> <a href="binaries/darwin-aarch64/modal">Download</a></li>
        </ul>
    </div>
    
    <h2>Other Packages</h2>
    <ul>
        <li><a href="binaries/">All Binaries</a></li>
        <li><a href="wasm/">WebAssembly Packages</a></li>
        <li><a href="cargo-registry/">Cargo Registry</a></li>
        <li><a href="manifest.json">Package Manifest</a></li>
        <li><a href="install.sh">Install Script</a></li>
    </ul>
    
    <h2>Installation Methods</h2>
    
    <h3>Method 1: Install Script (Recommended)</h3>
    <pre>curl -fsSL http://get.modal.money/$GIT_BRANCH/$VERSION/install.sh | sh</pre>
    
    <h3>Method 2: Manual Download</h3>
    <p>Download the binary for your platform above and add it to your PATH.</p>
    
    <h3>Method 3: Build from Source (Cargo Registry)</h3>
    <pre>cargo install --index sparse+https://get.modal.money/$GIT_BRANCH/$VERSION/cargo-registry/index/ modal</pre>
    
    <h3>Registry Configuration</h3>
    <p>Or add to <code>~/.cargo/config.toml</code>:</p>
    <pre>[registries.modal]
index = "sparse+https://get.modal.money/$GIT_BRANCH/$VERSION/cargo-registry/index/"</pre>
    <p>Then install with:</p>
    <pre>cargo install --registry modal modal</pre>
</body>
</html>
EOF
    
    # Create index.html for the binaries directory
    cat > "$BUILD_DIR/binaries/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Modality Binaries</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .platform { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
        .platform h3 { margin-top: 0; color: #007bff; }
    </style>
</head>
<body>
    <h1>Modality Pre-built Binaries</h1>
    <p>Version: <code>$VERSION</code></p>
    
    <div class="platform">
        <h3>Linux x86_64</h3>
        <ul>
            <li><a href="linux-x86_64/modal">modal</a></li>
        </ul>
    </div>
    
    <div class="platform">
        <h3>macOS Apple Silicon (ARM64)</h3>
        <ul>
            <li><a href="darwin-aarch64/modal">modal</a></li>
        </ul>
    </div>
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

# Invalidate CloudFront cache function
invalidate_cloudfront_cache() {
    log_info "Invalidating CloudFront cache..."
    
    # Get the CloudFront distribution ID from CDK exports, or use default
    # The CDK stack exports this as "GetModalMoneyDistributionId"
    DISTRIBUTION_ID=$(aws cloudformation describe-stacks \
        --region "$AWS_REGION" \
        --query "Stacks[?StackName=='GetModalMoneyStack'].Outputs[?ExportName=='GetModalMoneyDistributionId'].OutputValue" \
        --output text 2>/dev/null)
    
    # Use default distribution ID if not found in CloudFormation
    if [[ -z "$DISTRIBUTION_ID" || "$DISTRIBUTION_ID" == "None" ]]; then
        log_info "CloudFront distribution ID not found in CloudFormation exports"
        DISTRIBUTION_ID="EAB0G50HTKF8I"
        log_info "Using default distribution ID: $DISTRIBUTION_ID"
    fi
    
    log_info "Found CloudFront distribution: $DISTRIBUTION_ID"
    
    # Invalidate paths for the uploaded version and latest symlink
    INVALIDATION_PATHS="/$GIT_BRANCH/$VERSION/* /$GIT_BRANCH/latest/*"
    
    log_info "Creating invalidation for paths: $INVALIDATION_PATHS"
    
    INVALIDATION_ID=$(aws cloudfront create-invalidation \
        --distribution-id "$DISTRIBUTION_ID" \
        --paths $INVALIDATION_PATHS \
        --region "$AWS_REGION" \
        --query 'Invalidation.Id' \
        --output text 2>&1)
    
    if [[ $? -eq 0 && -n "$INVALIDATION_ID" && "$INVALIDATION_ID" != "None" ]]; then
        log_success "CloudFront cache invalidation created: $INVALIDATION_ID"
        log_info "Cache invalidation is in progress. It may take a few minutes to complete."
    else
        log_warning "Failed to create CloudFront invalidation: $INVALIDATION_ID"
        log_warning "Cache will expire naturally based on CloudFront TTL settings."
    fi
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
    
    # Invalidate CloudFront cache
    invalidate_cloudfront_cache
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
    "$PROJECT_ROOT/sites/get.modal.money/build-registry.sh"
    
    # Upload registry to S3
    log_info "Uploading registry to S3..."
    aws s3 sync "$PROJECT_ROOT/sites/get.modal.money/registry/" "s3://$S3_BUCKET/$CARGO_REGISTRY_PATH" \
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
    REGISTRY_CONFIG_URL="https://get.modal.money"
    REGISTRY_INDEX_URL="$REGISTRY_CONFIG_URL/$GIT_BRANCH/$VERSION/cargo-registry/index"
    
    log_success "Cargo registry published successfully!"
    log_info "Registry URLs:"
    log_info "  Registry config: $REGISTRY_CONFIG_URL"
    log_info "  Index URL: $REGISTRY_INDEX_URL"
    
    # Create registry configuration file
    cat > "$BUILD_DIR/cargo-registry-config.toml" << EOF
[registries.modal]
index = "$REGISTRY_INDEX_URL"
EOF
    
    log_info "To use this registry, add to ~/.cargo/config.toml:"
    log_info "  [registries.modal]"
    log_info "  index = \"$REGISTRY_INDEX_URL\""
    log_info ""
    log_info "Then install with:"
    log_info "  cargo install --index sparse+http://get.modal.money/$GIT_BRANCH/$VERSION/cargo-registry/index/ modal"
    
    # Invalidate CloudFront cache for registry paths
    invalidate_cloudfront_cache
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
