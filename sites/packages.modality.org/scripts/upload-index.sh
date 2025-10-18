#!/usr/bin/env bash

# Upload Modality Installation Page to S3
# This script uploads the index.html file to the registry.modality.org S3 bucket

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STATIC_DIR="$(dirname "$SCRIPT_DIR")/static"
INDEX_FILE="$STATIC_DIR/index.html"

# Default values
S3_BUCKET="packages.modality.org"
AWS_REGION="us-east-1"
DRY_RUN=false

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
Upload Modality Installation Page to S3

USAGE:
    $0 [OPTIONS]

OPTIONS:
    --bucket BUCKET         S3 bucket name (default: registry.modality.org)
    --region REGION         AWS region (default: us-east-1)
    --dry-run               Show what would be uploaded without actually uploading
    -h, --help              Show this help message

ENVIRONMENT VARIABLES:
    AWS_PROFILE            AWS profile to use

EXAMPLES:
    $0                      # Upload to default bucket
    $0 --dry-run           # Show what would be uploaded
    $0 --bucket my-bucket  # Upload to custom bucket

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --bucket)
            S3_BUCKET="$2"
            shift 2
            ;;
        --region)
            AWS_REGION="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
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

# Validate that the index.html file exists
if [[ ! -f "$INDEX_FILE" ]]; then
    log_error "Index file not found: $INDEX_FILE"
    exit 1
fi

# Check if AWS CLI is installed
if ! command -v aws &> /dev/null; then
    log_error "AWS CLI is not installed. Please install it first."
    exit 1
fi

# Check AWS credentials
if ! aws sts get-caller-identity &> /dev/null; then
    log_error "AWS credentials not configured. Please run 'aws configure' or set AWS_PROFILE."
    exit 1
fi

# Show configuration
log_info "Uploading Modality installation page to S3"
log_info "Script directory: $SCRIPT_DIR"
log_info "Static directory: $STATIC_DIR"
log_info "Index file: $INDEX_FILE"
log_info "S3 bucket: $S3_BUCKET"
log_info "AWS region: $AWS_REGION"

if [[ "$DRY_RUN" == true ]]; then
    log_warning "DRY RUN MODE - No files will be uploaded"
fi

# Upload function
upload_to_s3() {
    log_info "Uploading index.html to S3..."
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "Would upload: $INDEX_FILE -> s3://$S3_BUCKET/index.html"
        log_info "Command: aws s3 cp \"$INDEX_FILE\" \"s3://$S3_BUCKET/index.html\" --region \"$AWS_REGION\" --content-type \"text/html\""
    else
        # Upload the index.html file with proper content type
        aws s3 cp "$INDEX_FILE" "s3://$S3_BUCKET/index.html" \
            --region "$AWS_REGION" \
            --content-type "text/html" \
            --cache-control "public, max-age=3600"
        
        log_success "Index page uploaded successfully!"
        log_info "URL: https://$S3_BUCKET/"
        log_info "S3 path: s3://$S3_BUCKET/index.html"
    fi
}

# Main execution
upload_to_s3

if [[ "$DRY_RUN" == false ]]; then
    log_info "Upload completed successfully!"
    log_info "You can now visit: https://$S3_BUCKET/"
fi
