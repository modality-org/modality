#!/usr/bin/env bash
# Test cross-compilation for a single platform

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build/test"

log_info "Testing cross-compilation..."
log_info "Project root: $PROJECT_ROOT"
log_info "Build directory: $BUILD_DIR"

# Clean and create build directory
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Test Linux x86_64 build (fastest test)
TARGET="x86_64-unknown-linux-gnu"
log_info "Building for $TARGET..."

cd "$PROJECT_ROOT/rust"
cross build --release --target "$TARGET"

if [ $? -eq 0 ]; then
    log_success "Build successful for $TARGET"
    
    # Copy binary
    mkdir -p "$BUILD_DIR/linux-x86_64"
    cp "$PROJECT_ROOT/rust/target/$TARGET/release/modality" "$BUILD_DIR/linux-x86_64/"
    
    # Check binary size
    SIZE=$(du -h "$BUILD_DIR/linux-x86_64/modality" | cut -f1)
    log_info "Binary size: $SIZE"
    
    # Strip binary
    strip "$BUILD_DIR/linux-x86_64/modality"
    STRIPPED_SIZE=$(du -h "$BUILD_DIR/linux-x86_64/modality" | cut -f1)
    log_info "Stripped size: $STRIPPED_SIZE"
    
    log_success "Test build completed successfully!"
    log_info "Binary location: $BUILD_DIR/linux-x86_64/modality"
else
    log_error "Build failed for $TARGET"
    exit 1
fi

