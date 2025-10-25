#!/bin/bash

# Build Cargo Registry for get.modal.money
# This script creates a proper sparse registry with all workspace packages

set -e

# Get the project root (assuming this script is in sites/get.modal.money/)
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

# Function to get sparse registry path for a package name
get_index_path() {
    local name="$1"
    local len=${#name}
    
    if [ $len -le 3 ]; then
        echo "$len/$name"
    else
        echo "${name:0:2}/${name:2:2}/$name"
    fi
}

# Function to extract dependencies from Cargo.toml
extract_dependencies() {
    local cargo_toml="$1"
    local package_dir="$2"
    local deps_json="[]"
    
    # Parse [dependencies] section
    # This is a simplified parser that handles path dependencies with versions
    local in_deps=0
    local in_dev_deps=0
    local in_build_deps=0
    
    while IFS= read -r line; do
        # Check section headers
        if [[ "$line" =~ ^\[dependencies\] ]]; then
            in_deps=1
            in_dev_deps=0
            in_build_deps=0
            continue
        elif [[ "$line" =~ ^\[dev-dependencies\] ]]; then
            in_deps=0
            in_dev_deps=1
            in_build_deps=0
            continue
        elif [[ "$line" =~ ^\[build-dependencies\] ]]; then
            in_deps=0
            in_dev_deps=0
            in_build_deps=1
            continue
        elif [[ "$line" =~ ^\[.*\] ]]; then
            in_deps=0
            in_dev_deps=0
            in_build_deps=0
            continue
        fi
        
        # Only process normal dependencies (not dev or build)
        if [ $in_deps -eq 1 ]; then
            # Handle inline table format: dep = { path = "...", version = "..." }
            if [[ "$line" =~ ^([a-zA-Z0-9_-]+)[[:space:]]*=[[:space:]]*\{.*path[[:space:]]*=[[:space:]]*\"([^\"]+)\".*version[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
                local dep_name="${BASH_REMATCH[1]}"
                local version="${BASH_REMATCH[3]}"
                local optional="false"
                local default_features="true"
                local features="[]"
                
                # Check if optional
                if [[ "$line" =~ optional[[:space:]]*=[[:space:]]*true ]]; then
                    optional="true"
                fi
                
                # Check if default-features = false
                if [[ "$line" =~ default-features[[:space:]]*=[[:space:]]*false ]]; then
                    default_features="false"
                fi
                
                # Extract features if present
                if [[ "$line" =~ features[[:space:]]*=[[:space:]]*\[([^\]]*)\] ]]; then
                    local feat_str="${BASH_REMATCH[1]}"
                    # Convert to JSON array
                    features="["
                    local first=1
                    for feat in $(echo "$feat_str" | tr ',' '\n'); do
                        feat=$(echo "$feat" | sed 's/^[[:space:]]*"//;s/"[[:space:]]*$//')
                        if [ $first -eq 1 ]; then
                            features="${features}\"${feat}\""
                            first=0
                        else
                            features="${features},\"${feat}\""
                        fi
                    done
                    features="${features}]"
                fi
                
                # Add to deps array
                local dep_obj="{\"name\":\"${dep_name}\",\"req\":\"${version}\",\"features\":${features},\"optional\":${optional},\"default_features\":${default_features},\"target\":null,\"kind\":\"normal\"}"
                if [ "$deps_json" = "[]" ]; then
                    deps_json="[${dep_obj}]"
                else
                    deps_json="${deps_json%]},${dep_obj}]"
                fi
            # Handle reverse format: version = "...", path = "..."
            elif [[ "$line" =~ ^([a-zA-Z0-9_-]+)[[:space:]]*=[[:space:]]*\{.*version[[:space:]]*=[[:space:]]*\"([^\"]+)\".*path[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
                local dep_name="${BASH_REMATCH[1]}"
                local version="${BASH_REMATCH[2]}"
                local optional="false"
                local default_features="true"
                local features="[]"
                
                # Check if optional
                if [[ "$line" =~ optional[[:space:]]*=[[:space:]]*true ]]; then
                    optional="true"
                fi
                
                # Check if default-features = false
                if [[ "$line" =~ default-features[[:space:]]*=[[:space:]]*false ]]; then
                    default_features="false"
                fi
                
                # Extract features if present
                if [[ "$line" =~ features[[:space:]]*=[[:space:]]*\[([^\]]*)\] ]]; then
                    local feat_str="${BASH_REMATCH[1]}"
                    # Convert to JSON array
                    features="["
                    local first=1
                    for feat in $(echo "$feat_str" | tr ',' '\n'); do
                        feat=$(echo "$feat" | sed 's/^[[:space:]]*"//;s/"[[:space:]]*$//')
                        if [ $first -eq 1 ]; then
                            features="${features}\"${feat}\""
                            first=0
                        else
                            features="${features},\"${feat}\""
                        fi
                    done
                    features="${features}]"
                fi
                
                # Add to deps array
                local dep_obj="{\"name\":\"${dep_name}\",\"req\":\"${version}\",\"features\":${features},\"optional\":${optional},\"default_features\":${default_features},\"target\":null,\"kind\":\"normal\"}"
                if [ "$deps_json" = "[]" ]; then
                    deps_json="[${dep_obj}]"
                else
                    deps_json="${deps_json%]},${dep_obj}]"
                fi
            fi
        fi
    done < "$cargo_toml"
    
    echo "$deps_json"
}

# Function to extract features from Cargo.toml
extract_features() {
    local cargo_toml="$1"
    local features_json="{}"
    
    local in_features=0
    local feature_content=""
    
    while IFS= read -r line; do
        if [[ "$line" =~ ^\[features\] ]]; then
            in_features=1
            continue
        elif [[ "$line" =~ ^\[.*\] ]] && [ $in_features -eq 1 ]; then
            break
        fi
        
        if [ $in_features -eq 1 ]; then
            feature_content="${feature_content}${line}"$'\n'
        fi
    done < "$cargo_toml"
    
    # Parse features into JSON format
    if [ -n "$feature_content" ]; then
        features_json="{"
        local first=1
        while IFS= read -r line; do
            # Skip empty lines and comments
            [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
            
            # Parse feature line: feature_name = ["dep1", "dep2"]
            if [[ "$line" =~ ^([a-zA-Z0-9_-]+)[[:space:]]*=[[:space:]]*\[([^\]]*)\] ]]; then
                local feat_name="${BASH_REMATCH[1]}"
                local feat_deps="${BASH_REMATCH[2]}"
                
                # Build feature deps array
                local feat_array="["
                local first_dep=1
                for dep in $(echo "$feat_deps" | tr ',' '\n'); do
                    dep=$(echo "$dep" | sed 's/^[[:space:]]*"//;s/"[[:space:]]*$//')
                    if [ -n "$dep" ]; then
                        if [ $first_dep -eq 1 ]; then
                            feat_array="${feat_array}\"${dep}\""
                            first_dep=0
                        else
                            feat_array="${feat_array},\"${dep}\""
                        fi
                    fi
                done
                feat_array="${feat_array}]"
                
                if [ $first -eq 1 ]; then
                    features_json="${features_json}\"${feat_name}\":${feat_array}"
                    first=0
                else
                    features_json="${features_json},\"${feat_name}\":${feat_array}"
                fi
            fi
        done <<< "$feature_content"
        features_json="${features_json}}"
    fi
    
    echo "$features_json"
}

# Clean and create registry directory
log_info "Preparing registry directory..."
rm -rf "$REGISTRY_DIR"
mkdir -p "$REGISTRY_DIR"

# Extract workspace members
log_info "Extracting workspace members..."
cd "$PROJECT_ROOT/rust"

# Define packages in dependency order (least dependent first)
# This ensures dependencies are packaged before dependents
WORKSPACE_MEMBERS=(
    "modality-utils"
    "modal-datastore"
    "modality-network-consensus"
    "modality-network-mining"
    "modality-network-devnet"
    "modality-network-node"
    "modality-lang"
    "modality"
)

log_info "Processing ${#WORKSPACE_MEMBERS[@]} workspace members in dependency order"

# Arrays to store package information (using parallel indexed arrays for compatibility)
PACKAGE_NAMES=()
PACKAGE_VERSIONS=()
PACKAGE_CHECKSUMS=()

# Process each workspace member
for member in "${WORKSPACE_MEMBERS[@]}"; do
    log_info "Processing package: $member"
    
    # Navigate to package directory
    cd "$PROJECT_ROOT/rust/$member"
    
    # Extract package info from Cargo.toml
    PACKAGE_NAME=$(grep '^name' Cargo.toml | head -1 | cut -d'"' -f2)
    PACKAGE_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
    
    log_info "  Name: $PACKAGE_NAME"
    log_info "  Version: $PACKAGE_VERSION"
    
    # Run cargo package
    log_info "  Running cargo package..."
    # Use --no-verify to skip dependency resolution from crates.io
    # The packaged .crate will have proper metadata for the registry
    if cargo package --allow-dirty --no-verify > /dev/null 2>&1; then
        # Copy .crate file to registry
        CRATE_FILE="${PACKAGE_NAME}-${PACKAGE_VERSION}.crate"
        cp "$PROJECT_ROOT/rust/target/package/$CRATE_FILE" "$REGISTRY_DIR/"
        
        log_info "  Copied $CRATE_FILE to registry"
    else
        log_warning "  Failed to package $PACKAGE_NAME"
        # Try to see the error
        ERROR_MSG=$(cargo package --allow-dirty --no-verify 2>&1 | tail -5)
        
        # Special handling for packages that fail due to missing workspace deps on crates.io
        # We'll manually create a .crate by running cargo prepare-for-publish
        if [[ "$ERROR_MSG" =~ ("no matching package named"|"failed to select a version") ]]; then
            log_info "  Attempting manual package creation..."
            
            # Try building just the package to ensure it compiles
            if cargo build -p "$PACKAGE_NAME" > /dev/null 2>&1; then
                # Create a temporary directory for manual packaging
                TEMP_PKG_DIR="/tmp/${PACKAGE_NAME}-${PACKAGE_VERSION}"
                rm -rf "$TEMP_PKG_DIR"
                mkdir -p "$TEMP_PKG_DIR"
                
                # Copy source files
                cp -r "$PROJECT_ROOT/rust/$member"/* "$TEMP_PKG_DIR/" 2>/dev/null || true
                
                # Normalize Cargo.toml (remove path specs from dependencies)
                if [ -f "$TEMP_PKG_DIR/Cargo.toml" ]; then
                    # Create normalized Cargo.toml
                    python3 -c "
import re, sys
with open('$TEMP_PKG_DIR/Cargo.toml', 'r') as f:
    content = f.read()
# Remove path specifications from dependencies, keeping version and features
content = re.sub(r'(\w+)\s*=\s*\{\s*path\s*=\s*\"[^\"]+\"\s*,\s*version\s*=\s*\"([^\"]+)\"([^}]*)\}', r'\1 = { version = \"\2\"\3}', content)
content = re.sub(r'(\w+)\s*=\s*\{\s*version\s*=\s*\"([^\"]+)\"\s*,\s*path\s*=\s*\"[^\"]+\"([^}]*)\}', r'\1 = { version = \"\2\"\3}', content)
with open('$TEMP_PKG_DIR/Cargo.toml', 'w') as f:
    f.write(content)
" 2>/dev/null
                    
                    # Create .crate archive
                    CRATE_FILE="${PACKAGE_NAME}-${PACKAGE_VERSION}.crate"
                    cd /tmp
                    # Exclude macOS metadata files
                    COPYFILE_DISABLE=1 tar -czf "$REGISTRY_DIR/$CRATE_FILE" --exclude='._*' --exclude='.DS_Store' "${PACKAGE_NAME}-${PACKAGE_VERSION}"
                    cd "$PROJECT_ROOT/rust/$member"
                    
                    # Clean up temp dir
                    rm -rf "$TEMP_PKG_DIR"
                    
                    log_success "  Manually created $CRATE_FILE"
                else
                    log_warning "  Could not manually package, skipping..."
                    continue
                fi
            else
                log_warning "  Package doesn't build, skipping..."
                continue
            fi
        else
            echo "$ERROR_MSG" | while read line; do
                log_warning "    $line"
            done
            continue
        fi
    fi
    
    # Calculate checksum
    if command -v sha256sum &> /dev/null; then
        CHECKSUM=$(sha256sum "$REGISTRY_DIR/$CRATE_FILE" | cut -d' ' -f1)
    elif command -v shasum &> /dev/null; then
        CHECKSUM=$(shasum -a 256 "$REGISTRY_DIR/$CRATE_FILE" | cut -d' ' -f1)
    else
        log_error "Neither sha256sum nor shasum found"
        exit 1
    fi
    
    log_info "  Checksum: $CHECKSUM"
    
    # Store package information (append to parallel arrays)
    PACKAGE_NAMES+=("$PACKAGE_NAME")
    PACKAGE_VERSIONS+=("$PACKAGE_VERSION")
    PACKAGE_CHECKSUMS+=("$CHECKSUM")
    
    # Extract dependencies
    log_info "  Extracting dependencies..."
    DEPS_JSON=$(extract_dependencies "$PROJECT_ROOT/rust/$member/Cargo.toml" "$PROJECT_ROOT/rust/$member")
    
    # Extract features
    log_info "  Extracting features..."
    FEATURES_JSON=$(extract_features "$PROJECT_ROOT/rust/$member/Cargo.toml")
    
    # Create sparse registry index structure
    INDEX_PATH=$(get_index_path "$PACKAGE_NAME")
    mkdir -p "$REGISTRY_DIR/index/$(dirname "$INDEX_PATH")"
    
    # Create package metadata JSON
    PACKAGE_JSON="{\"name\":\"$PACKAGE_NAME\",\"vers\":\"$PACKAGE_VERSION\",\"deps\":$DEPS_JSON,\"cksum\":\"$CHECKSUM\",\"features\":$FEATURES_JSON,\"yanked\":false,\"links\":null}"
    
    # Append to index file (one JSON per line for sparse registry)
    echo "$PACKAGE_JSON" >> "$REGISTRY_DIR/index/$INDEX_PATH"
    
    log_success "  Package $PACKAGE_NAME processed successfully"
done

# Create config.json for sparse registry
log_info "Creating registry config.json..."
cat > "$REGISTRY_DIR/index/config.json" << EOF
{
    "dl": "http://get.modal.money/testnet/latest/cargo-registry/{crate}-{version}.crate",
    "api": "http://get.modal.money/testnet/latest/cargo-registry/"
}
EOF

# Create index.html files for browsing
log_info "Creating index.html files for browsing..."

# Main registry index.html
cat > "$REGISTRY_DIR/index.html" << 'EOF_MAIN'
<!DOCTYPE html>
<html>
<head>
    <title>Modality Package Registry</title>
    <style>
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
            margin: 40px;
            background: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }
        h1 { 
            color: #333;
            border-bottom: 2px solid #007bff;
            padding-bottom: 10px;
        }
        h2 {
            color: #555;
            margin-top: 30px;
        }
        .package { 
            background: #f8f9fa;
            padding: 15px;
            margin: 15px 0;
            border-radius: 5px;
            border-left: 4px solid #007bff;
        }
        .package h3 {
            margin-top: 0;
            color: #007bff;
        }
        .version { 
            color: #666;
            font-size: 14px;
        }
        .checksum {
            font-size: 11px;
            color: #999;
            font-family: monospace;
            word-break: break-all;
        }
        code { 
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Monaco', 'Courier New', monospace;
            font-size: 13px;
        }
        pre {
            background: #2d2d2d;
            color: #f8f8f2;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
        }
        .install-section {
            background: #e7f3ff;
            padding: 20px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Modality Package Registry</h1>
        <p>This is a Cargo sparse registry for Modality workspace packages.</p>
        
        <h2>Available Packages</h2>
EOF_MAIN

# Add each package to the HTML
for i in "${!PACKAGE_NAMES[@]}"; do
    PACKAGE_NAME="${PACKAGE_NAMES[$i]}"
    PACKAGE_VERSION="${PACKAGE_VERSIONS[$i]}"
    CHECKSUM="${PACKAGE_CHECKSUMS[$i]}"
    
    if [ -n "$PACKAGE_NAME" ]; then
        cat >> "$REGISTRY_DIR/index.html" << EOF
        <div class="package">
            <h3>$PACKAGE_NAME</h3>
            <p class="version">Version: $PACKAGE_VERSION</p>
            <p class="checksum">Checksum: $CHECKSUM</p>
        </div>
EOF
    fi
done

# Complete the main HTML
cat >> "$REGISTRY_DIR/index.html" << 'EOF_MAIN2'
        
        <h2>Quick Installation (Recommended)</h2>
        <div class="install-section">
            <h3>Option 1: Pre-built Binary (Fast & Easy)</h3>
            <p>Install the pre-compiled binary with a single command:</p>
            <pre>curl -fsSL http://get.modal.money/testnet/latest/install.sh | sh</pre>
            <p>Supports: Linux x86_64 • macOS Apple Silicon</p>
            
            <h3>Option 2: Direct Download</h3>
            <p>Download binaries directly:</p>
            <ul>
                <li><a href="http://get.modal.money/testnet/latest/binaries/linux-x86_64/modality">Linux x86_64</a></li>
                <li><a href="http://get.modal.money/testnet/latest/binaries/darwin-aarch64/modality">macOS Apple Silicon</a></li>
            </ul>
        </div>
        
        <h2>Build from Source (Cargo)</h2>
        <div class="install-section">
            <p>To build and install from this registry:</p>
            <pre>cargo install --index sparse+http://get.modal.money/testnet/latest/cargo-registry/index/ modality</pre>
        </div>
        
        <h2>Registry Configuration</h2>
        <p>Add to <code>~/.cargo/config.toml</code>:</p>
        <pre>[registries.modality]
index = "sparse+http://get.modal.money/testnet/latest/cargo-registry/index/"</pre>
        <p>Then install with:</p>
        <pre>cargo install --registry modality modality</pre>
        
        <h2>About</h2>
        <p>This registry contains all Modality workspace crates with proper dependency resolution. When you install <code>modality</code>, all internal dependencies will be automatically fetched from this registry.</p>
    </div>
</body>
</html>
EOF_MAIN2

# Create package index directory HTML
mkdir -p "$REGISTRY_DIR/index/mo/da"
cat > "$REGISTRY_DIR/index/mo/da/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>Modality Package Index - mo/da</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        h1 { color: #333; }
        .package { 
            background: #f5f5f5;
            padding: 10px;
            margin: 10px 0;
            border-radius: 5px;
        }
        code { 
            background: #eee;
            padding: 2px 4px;
            border-radius: 3px;
        }
    </style>
</head>
<body>
    <h1>Package Index: mo/da</h1>
    <p>Packages in the Modality workspace</p>
    
    <h2>Packages</h2>
EOF

# Add links to each package
for i in "${!PACKAGE_NAMES[@]}"; do
    PACKAGE_NAME="${PACKAGE_NAMES[$i]}"
    if [ -n "$PACKAGE_NAME" ]; then
        echo "    <div class=\"package\"><a href=\"$PACKAGE_NAME\">$PACKAGE_NAME</a></div>" >> "$REGISTRY_DIR/index/mo/da/index.html"
    fi
done

cat >> "$REGISTRY_DIR/index/mo/da/index.html" << EOF
</body>
</html>
EOF

log_success "Registry built successfully!"
log_info "Registry location: $REGISTRY_DIR"

# Show summary
log_info "Package summary:"
for i in "${!PACKAGE_NAMES[@]}"; do
    PACKAGE_NAME="${PACKAGE_NAMES[$i]}"
    PACKAGE_VERSION="${PACKAGE_VERSIONS[$i]}"
    if [ -n "$PACKAGE_NAME" ]; then
        log_info "  - $PACKAGE_NAME v$PACKAGE_VERSION"
    fi
done

# Show registry structure
log_info "Registry structure:"
find "$REGISTRY_DIR" -type f | sort | sed "s|$REGISTRY_DIR/||" | sed 's/^/  /'

log_info "Ready for upload to S3!"
