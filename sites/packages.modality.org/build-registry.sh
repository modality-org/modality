#!/bin/bash

# Build Cargo Registry for packages.modality.org
# This script creates a proper sparse registry using cargo package

set -e

# Get the project root (assuming this script is in sites/packages.modality.org/)
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REGISTRY_DIR="$(cd "$(dirname "$0")" && pwd)/registry"

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

# Extract package info from Cargo.toml
log_info "Extracting package information..."
PACKAGE_NAME=$(grep '^name' "$PROJECT_ROOT/rust/modality/Cargo.toml" | cut -d'"' -f2)
PACKAGE_VERSION=$(grep '^version' "$PROJECT_ROOT/rust/modality/Cargo.toml" | head -1 | cut -d'"' -f2)

log_info "Package: $PACKAGE_NAME v$PACKAGE_VERSION"

# Clean and create registry directory
log_info "Preparing registry directory..."
rm -rf "$REGISTRY_DIR"
mkdir -p "$REGISTRY_DIR"

# Use cargo package to create proper source crate
log_info "Creating package with cargo package..."
cd "$PROJECT_ROOT/rust/modality"
cargo package --allow-dirty --no-verify
cd "$PROJECT_ROOT"

# Copy the generated .crate file to registry
CRATE_FILE="${PACKAGE_NAME}-${PACKAGE_VERSION}.crate"
cp "rust/target/package/$CRATE_FILE" "$REGISTRY_DIR/"

log_info "Copied $CRATE_FILE to registry"

# Calculate checksum
log_info "Calculating checksum..."
CHECKSUM=$(sha256sum "$REGISTRY_DIR/$CRATE_FILE" | cut -d' ' -f1)
log_info "Checksum: $CHECKSUM"

# Create sparse registry index structure
log_info "Creating sparse registry index..."
mkdir -p "$REGISTRY_DIR/index/mo/da"

# Create package metadata JSON
PACKAGE_JSON="{\"name\":\"$PACKAGE_NAME\",\"vers\":\"$PACKAGE_VERSION\",\"deps\":[],\"cksum\":\"$CHECKSUM\",\"features\":{},\"yanked\":false,\"links\":null}"
echo "$PACKAGE_JSON" > "$REGISTRY_DIR/index/mo/da/$PACKAGE_NAME"

# Create config.json for sparse registry
log_info "Creating registry config.json..."
cat > "$REGISTRY_DIR/index/config.json" << EOF
{
    "dl": "http://packages.modality.org/testnet/latest/cargo-registry/{crate}-{version}.crate",
    "api": "http://packages.modality.org/testnet/latest/cargo-registry/"
}
EOF

# Create index.html files for browsing
log_info "Creating index.html files for browsing..."

# Main registry index.html
cat > "$REGISTRY_DIR/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>Modality Package Registry</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        h1 { color: #333; }
        .package { background: #f5f5f5; padding: 10px; margin: 10px 0; border-radius: 5px; }
        .version { color: #666; }
        code { background: #eee; padding: 2px 4px; border-radius: 3px; }
    </style>
</head>
<body>
    <h1>Modality Package Registry</h1>
    <p>This is a Cargo sparse registry for Modality packages.</p>
    
    <h2>Available Packages</h2>
    <div class="package">
        <h3>$PACKAGE_NAME</h3>
        <p class="version">Version: $PACKAGE_VERSION</p>
        <p>Checksum: <code>$CHECKSUM</code></p>
    </div>
    
    <h2>Installation</h2>
    <p>To install packages from this registry:</p>
    <pre>cargo install --index sparse+http://packages.modality.org/index/ $PACKAGE_NAME</pre>
    
    <h2>Registry Configuration</h2>
    <p>Add to <code>~/.cargo/config.toml</code>:</p>
    <pre>[registries.modality]
index = "sparse+http://packages.modality.org/index/"</pre>
    <p>Then install with:</p>
    <pre>cargo install --registry modality $PACKAGE_NAME</pre>
</body>
</html>
EOF

# Package index.html
cat > "$REGISTRY_DIR/index/mo/da/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>Modality Package Index</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        h1 { color: #333; }
        .package { background: #f5f5f5; padding: 10px; margin: 10px 0; border-radius: 5px; }
        code { background: #eee; padding: 2px 4px; border-radius: 3px; }
    </style>
</head>
<body>
    <h1>Package Index: mo/da</h1>
    <p>Packages starting with "mo"</p>
    
    <div class="package">
        <h3><a href="modality">modality</a></h3>
        <p>Modality CLI tool</p>
    </div>
</body>
</html>
EOF

log_success "Registry built successfully!"
log_info "Registry location: $REGISTRY_DIR"
log_info "Package: $PACKAGE_NAME v$PACKAGE_VERSION"
log_info "Checksum: $CHECKSUM"

# Show registry structure
log_info "Registry structure:"
find "$REGISTRY_DIR" -type f | sort | sed "s|$REGISTRY_DIR/||" | sed 's/^/  /'

log_info "Ready for upload to S3!"
