#!/usr/bin/env bash

# Build and Upload Modal Packages
# This script is a convenience wrapper that calls build.sh and upload.sh

set -e  # Exit on any error

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_SCRIPT="$SCRIPT_DIR/build.sh"
UPLOAD_SCRIPT="$SCRIPT_DIR/upload.sh"

# Validate allowed branches
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
ALLOWED_BRANCHES=("mainnet" "testnet")
if [[ ! " ${ALLOWED_BRANCHES[@]} " =~ " ${GIT_BRANCH} " ]]; then
    echo -e "\033[0;31m[ERROR]\033[0m Branch '$GIT_BRANCH' is not allowed. Allowed branches: ${ALLOWED_BRANCHES[*]}"
    echo -e "\033[0;31m[ERROR]\033[0m Please switch to one of the allowed branches before running this script."
    exit 1
fi

# Default values
SKIP_BUILD=false
SKIP_UPLOAD=false
ENABLE_CARGO_REGISTRY=false

# Arrays to hold arguments for each script
BUILD_ARGS=()
UPLOAD_ARGS=()

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

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
Build and Upload Modality Packages

This is a convenience wrapper that calls build.sh and upload.sh in sequence.

USAGE:
    $0 [OPTIONS]

OPTIONS:
    Build Options:
      --version VERSION       Package version (default: TIMESTAMP-GITCOMMIT)
      --clean                 Clean build directory before building
      --skip-js               Skip JavaScript packages build
      --skip-build            Skip build step, only upload

    Upload Options:
      --bucket BUCKET         S3 bucket name (default: get.modal.money-content)
      --prefix PREFIX         S3 prefix for uploads
      --region REGION         AWS region (default: us-east-1)
      --enable-cargo-registry Enable Cargo registry publishing
      --skip-latest           Skip updating the 'latest' symlink
      --skip-upload           Skip upload step, only build

    General:
      -h, --help              Show this help message

ENVIRONMENT VARIABLES:
    AWS_PROFILE            AWS profile to use

EXAMPLES:
    $0                                     # Build and upload with defaults
    $0 --clean                             # Clean build and upload
    $0 --skip-build                        # Only upload existing build
    $0 --skip-upload                       # Only build, don't upload
    $0 --bucket my-bucket --region us-west-2
    $0 --enable-cargo-registry             # Include Cargo registry in upload
    $0 --version "1.0.0" --skip-js         # Custom version, skip JS packages

S3 PATH STRUCTURE:
    s3://BUCKET/PREFIX/BRANCH/VERSION/
    Example: s3://get.modal.money-content/testnet/20251121_215641-1428985/

NOTES:
    - This script calls build.sh and upload.sh sequentially
    - For more control, call those scripts directly
    - Only 'mainnet' and 'testnet' branches are allowed

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        # Build options
        --version)
            BUILD_ARGS+=("--version" "$2")
            UPLOAD_ARGS+=("--version" "$2")
            shift 2
            ;;
        --clean)
            BUILD_ARGS+=("--clean")
            shift
            ;;
        --skip-js)
            BUILD_ARGS+=("--skip-js")
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        
        # Upload options
        --bucket)
            UPLOAD_ARGS+=("--bucket" "$2")
            shift 2
            ;;
        --prefix)
            UPLOAD_ARGS+=("--prefix" "$2")
            shift 2
            ;;
        --region)
            UPLOAD_ARGS+=("--region" "$2")
            shift 2
            ;;
        --enable-cargo-registry)
            UPLOAD_ARGS+=("--enable-cargo-registry")
            shift
            ;;
        --skip-latest)
            UPLOAD_ARGS+=("--skip-latest")
            shift
            ;;
        --skip-upload)
            SKIP_UPLOAD=true
            shift
            ;;
        
        # Help
        -h|--help)
            show_help
            exit 0
            ;;
        
        # Unknown
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Main execution
log_info "Build and Upload Process Starting"
log_info "Branch: $GIT_BRANCH"

# Step 1: Build
if [[ "$SKIP_BUILD" == false ]]; then
    log_info "Step 1/2: Building packages..."
    if ! bash "$BUILD_SCRIPT" "${BUILD_ARGS[@]}"; then
        log_error "Build failed"
        exit 1
    fi
    log_success "Build completed"
else
    log_info "Skipping build step"
fi

# Step 2: Upload
if [[ "$SKIP_UPLOAD" == false ]]; then
    log_info "Step 2/2: Uploading packages..."
    if ! bash "$UPLOAD_SCRIPT" "${UPLOAD_ARGS[@]}"; then
        log_error "Upload failed"
        exit 1
    fi
    log_success "Upload completed"
else
    log_info "Skipping upload step"
fi

log_success "Build and upload process completed successfully!"
