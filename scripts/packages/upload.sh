#!/usr/bin/env bash

# Upload Modal Packages
# This script uploads the built modal packages to AWS S3

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

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
SKIP_CARGO_REGISTRY=true
UPLOAD_LATEST=true

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
Upload Modality Packages to S3

USAGE:
    $0 [OPTIONS]

OPTIONS:
    --bucket BUCKET         S3 bucket name for uploads (default: get.modal.money-content)
    --prefix PREFIX         S3 prefix for uploads (default: empty)
    --region REGION         AWS region (default: us-east-1)
    --build-dir DIR         Build directory (default: PROJECT_ROOT/build)
    --version VERSION       Package version (default: from manifest.json)
    --enable-cargo-registry Enable Cargo registry publishing (disabled by default)
    --skip-latest           Skip updating the 'latest' symlink
    -h, --help              Show this help message

ENVIRONMENT VARIABLES:
    AWS_PROFILE            AWS profile to use

EXAMPLES:
    $0                                    # Upload to default bucket
    $0 --bucket my-bucket                 # Use custom bucket
    $0 --prefix modal-packages/           # Add prefix to path
    $0 --region us-west-2                 # Use different region
    $0 --enable-cargo-registry            # Enable Cargo registry upload

S3 PATH STRUCTURE:
    Uploads will be organized as: s3://BUCKET/PREFIX/BRANCH/VERSION/
    Default: s3://get.modal.money-content/BRANCH/VERSION/
    Example: s3://get.modal.money-content/testnet/20251118_143022-a1b2c3d/

CARGO REGISTRY:
    The script also publishes to a Cargo sparse registry for easy installation:
    cargo install --index sparse+https://get.modal.money/BRANCH/VERSION/cargo-registry/index/ modal

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
        --region)
            AWS_REGION="$2"
            shift 2
            ;;
        --build-dir)
            BUILD_DIR="$2"
            shift 2
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --enable-cargo-registry)
            SKIP_CARGO_REGISTRY=false
            shift
            ;;
        --skip-latest)
            UPLOAD_LATEST=false
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

# Verify build directory exists
if [[ ! -d "$BUILD_DIR" ]]; then
    log_error "Build directory does not exist: $BUILD_DIR"
    log_error "Please run build.sh first or specify correct --build-dir"
    exit 1
fi

# Load version from manifest if not specified
if [[ -z "$VERSION" ]]; then
    if [[ -f "$BUILD_DIR/manifest.json" ]]; then
        VERSION=$(grep -o '"version": "[^"]*"' "$BUILD_DIR/manifest.json" | cut -d'"' -f4)
        log_info "Loaded version from manifest: $VERSION"
    else
        log_error "No manifest.json found in build directory and no --version specified"
        exit 1
    fi
fi

# Show configuration
log_info "Starting Modality package upload process"
log_info "Build directory: $BUILD_DIR"
log_info "Git branch: $GIT_BRANCH"
log_info "Git commit: $GIT_COMMIT"
log_info "Version: $VERSION"
log_info "S3 bucket: $S3_BUCKET"
log_info "S3 prefix: $S3_PREFIX"
log_info "S3 path: $S3_PREFIX$GIT_BRANCH/$VERSION/"
log_info "AWS region: $AWS_REGION"

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
    
    # Create the structured path: BRANCH/VERSION/
    S3_PATH="$S3_PREFIX$GIT_BRANCH/$VERSION/"
    
    # Upload build directory
    log_info "Uploading to s3://$S3_BUCKET/$S3_PATH"
    aws s3 sync "$BUILD_DIR" "s3://$S3_BUCKET/$S3_PATH" \
        --region "$AWS_REGION" \
        --exclude "*.DS_Store" \
        --exclude "*.git*" \
        --exclude "node_modules/*"
    
    # Upload latest symlink for the branch
    if [[ "$UPLOAD_LATEST" == true ]]; then
        log_info "Updating 'latest' symlink for branch $GIT_BRANCH"
        aws s3 cp "s3://$S3_BUCKET/$S3_PATH" "s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/latest/" \
            --recursive \
            --region "$AWS_REGION"
    fi
    
    log_success "Packages uploaded successfully!"
    log_info "S3 URLs:"
    log_info "  Version: s3://$S3_BUCKET/$S3_PATH"
    if [[ "$UPLOAD_LATEST" == true ]]; then
        log_info "  Latest:  s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/latest/"
    fi
    log_info "  Branch:  $GIT_BRANCH"
    log_info "  Commit:  $GIT_COMMIT"
    
    # Public URLs (assuming the bucket is configured for static website hosting)
    log_info ""
    log_info "Public URLs:"
    log_info "  Version: https://get.modal.money/$GIT_BRANCH/$VERSION/"
    if [[ "$UPLOAD_LATEST" == true ]]; then
        log_info "  Latest:  https://get.modal.money/$GIT_BRANCH/latest/"
    fi
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
    
    # Check if build-registry.sh exists
    if [[ ! -f "$PROJECT_ROOT/sites/get.modal.money/build-registry.sh" ]]; then
        log_warning "build-registry.sh not found, skipping Cargo registry publishing"
        return 0
    fi
    
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
    if [[ "$UPLOAD_LATEST" == true ]]; then
        log_info "Updating latest symlink with new registry structure..."
        aws s3 sync "s3://$S3_BUCKET/$CARGO_REGISTRY_PATH" "s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/latest/cargo-registry/" \
            --region "$AWS_REGION" \
            --delete
    fi
    
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
    
    log_info ""
    log_info "To use this registry, add to ~/.cargo/config.toml:"
    log_info "  [registries.modal]"
    log_info "  index = \"$REGISTRY_INDEX_URL\""
    log_info ""
    log_info "Then install with:"
    log_info "  cargo install --index sparse+https://get.modal.money/$GIT_BRANCH/$VERSION/cargo-registry/index/ modal"
}

# Main execution
main() {
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Upload to S3
    upload_to_s3
    
    # Invalidate CloudFront cache
    invalidate_cloudfront_cache
    
    # Publish to Cargo registry if not skipped
    if [[ "$SKIP_CARGO_REGISTRY" == false ]]; then
        publish_to_cargo_registry
    else
        log_warning "Skipping Cargo registry publishing"
    fi
    
    log_success "Upload process completed successfully!"
    
    # Show upload summary
    log_info ""
    log_info "Upload summary:"
    log_info "  Build directory: $BUILD_DIR"
    log_info "  Total size: $(du -sh "$BUILD_DIR" | cut -f1)"
    log_info "  Git branch: $GIT_BRANCH"
    log_info "  Git commit: $GIT_COMMIT"
    log_info "  Version: $VERSION"
    log_info "  S3 location: s3://$S3_BUCKET/$S3_PREFIX$GIT_BRANCH/$VERSION/"
}

# Run main function
main "$@"

