#!/bin/bash
# Example: Creating a Custom WASM Predicate
# This demonstrates how to create and upload your own predicate

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../.."

echo "=========================================="
echo "Custom Predicate Example"
echo "=========================================="
echo ""

# Step 1: Create a custom predicate in Rust
echo "Step 1: Creating a custom predicate..."
echo ""

# Create tmp directory for the custom predicate
mkdir -p "$SCRIPT_DIR/tmp/custom-predicate/src"

cat > "$SCRIPT_DIR/tmp/custom-predicate/Cargo.toml" << 'EOF'
[package]
name = "custom-predicate"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = "z"
lto = true
EOF

cat > "$SCRIPT_DIR/tmp/custom-predicate/src/lib.rs" << 'EOF'
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Input {
    data: Value,
    context: Context,
}

#[derive(Debug, Deserialize)]
struct Context {
    contract_id: String,
    block_height: u64,
    timestamp: u64,
}

#[derive(Debug, Serialize)]
struct Result {
    valid: bool,
    gas_used: u64,
    errors: Vec<String>,
}

/// Custom predicate: check if a value is within a percentage range of a target
/// 
/// Example: is_within_percent({"value": 105, "target": 100, "percent": 10})
/// Returns true because 105 is within 10% of 100 (90-110)
#[wasm_bindgen]
pub fn evaluate(input_json: &str) -> String {
    let gas_used = 25;

    let input: Input = match serde_json::from_str(input_json) {
        Ok(i) => i,
        Err(e) => {
            let result = Result {
                valid: false,
                gas_used,
                errors: vec![format!("Invalid input: {}", e)],
            };
            return serde_json::to_string(&result).unwrap();
        }
    };

    // Extract parameters
    let value = match input.data.get("value").and_then(|v| v.as_f64()) {
        Some(v) => v,
        None => {
            let result = Result {
                valid: false,
                gas_used,
                errors: vec!["Missing or invalid 'value' parameter".to_string()],
            };
            return serde_json::to_string(&result).unwrap();
        }
    };

    let target = match input.data.get("target").and_then(|v| v.as_f64()) {
        Some(t) => t,
        None => {
            let result = Result {
                valid: false,
                gas_used,
                errors: vec!["Missing or invalid 'target' parameter".to_string()],
            };
            return serde_json::to_string(&result).unwrap();
        }
    };

    let percent = match input.data.get("percent").and_then(|v| v.as_f64()) {
        Some(p) => p,
        None => {
            let result = Result {
                valid: false,
                gas_used,
                errors: vec!["Missing or invalid 'percent' parameter".to_string()],
            };
            return serde_json::to_string(&result).unwrap();
        }
    };

    // Calculate range
    let tolerance = target * (percent / 100.0);
    let min = target - tolerance;
    let max = target + tolerance;

    // Check if value is within range
    let valid = value >= min && value <= max;

    let result = Result {
        valid,
        gas_used,
        errors: if valid {
            vec![]
        } else {
            vec![format!(
                "Value {} is not within {}% of {} (range: {}-{})",
                value, percent, target, min, max
            )]
        },
    };

    serde_json::to_string(&result).unwrap()
}
EOF

echo "✓ Created custom predicate: is_within_percent"
echo ""
echo "Predicate logic:"
echo "  - Checks if a value is within a percentage of a target"
echo "  - Example: 105 is within 10% of 100 (range 90-110)"
echo ""

# Step 2: Build the predicate
echo "Step 2: Building WASM module..."
echo ""

cd "$SCRIPT_DIR/tmp/custom-predicate"

if command -v wasm-pack &> /dev/null; then
    echo "Building with wasm-pack..."
    wasm-pack build --target web --release
    
    WASM_FILE="pkg/custom_predicate_bg.wasm"
    
    if [ -f "$WASM_FILE" ]; then
        echo "✓ Built WASM module: $WASM_FILE"
        echo "  Size: $(wc -c < "$WASM_FILE") bytes"
        echo ""
    else
        echo "⚠️  WASM file not found. Build may have failed."
        echo ""
    fi
else
    echo "⚠️  wasm-pack not installed. To build:"
    echo "     cargo install wasm-pack"
    echo "     cd $SCRIPT_DIR/tmp/custom-predicate"
    echo "     wasm-pack build --target web --release"
    echo ""
fi

# Step 3: Upload to contract
echo "Step 3: Uploading to contract..."
echo ""
echo "To upload your custom predicate:"
echo ""
echo "  # Convert WASM to base64"
echo "  WASM_BASE64=\$(base64 < tmp/custom-predicate/pkg/custom_predicate_bg.wasm)"
echo ""
echo "  # Create commit with POST action"
echo "  modal contract commit mycontract \\"
echo "    --post /_code/is_within_percent.wasm \"\$WASM_BASE64\""
echo ""

# Step 4: Use in contract
echo "Step 4: Using in your contract..."
echo ""

cat > "$SCRIPT_DIR/tmp/custom-contract.modality" << 'EOF'
model price_monitor:
  part checker:
    init -> checking: +start
    checking -> ok: +is_within_percent({"value": 105, "target": 100, "percent": 10})
    checking -> alert: -is_within_percent({"value": 150, "target": 100, "percent": 10})
    ok -> checking: +recheck
    alert -> checking: +reset

formula acceptable_price:
  <+is_within_percent> true
EOF

echo "Example contract using custom predicate:"
echo "----------------------------------------"
cat "$SCRIPT_DIR/tmp/custom-contract.modality"
echo "----------------------------------------"
echo ""

# Step 5: Test the predicate
echo "Step 5: Testing the predicate..."
echo ""
echo "Test cases:"
echo ""
echo "  1. is_within_percent({value: 105, target: 100, percent: 10})"
echo "     Expected: +is_within_percent (valid: 105 in range 90-110)"
echo ""
echo "  2. is_within_percent({value: 150, target: 100, percent: 10})"
echo "     Expected: -is_within_percent (invalid: 150 not in range 90-110)"
echo ""
echo "  3. is_within_percent({value: 90, target: 100, percent: 10})"
echo "     Expected: +is_within_percent (valid: 90 in range 90-110)"
echo ""

# Step 6: Best practices
echo "Step 6: Best practices for custom predicates:"
echo ""
echo "✓ Keep predicates simple and focused"
echo "✓ Use descriptive names (snake_case)"
echo "✓ Validate all inputs"
echo "✓ Return clear error messages"
echo "✓ Keep gas usage low (< 1M for simple checks)"
echo "✓ Test thoroughly before deployment"
echo "✓ Document expected input format"
echo "✓ Make predicates deterministic (no randomness)"
echo ""

# Step 7: Security considerations
echo "Step 7: Security considerations:"
echo ""
echo "⚠️  Predicates run in a sandboxed environment"
echo "⚠️  No access to filesystem or network"
echo "⚠️  Gas limits prevent infinite loops"
echo "⚠️  All inputs are untrusted - validate everything"
echo "⚠️  Hash verification ensures integrity"
echo "⚠️  Cross-contract predicates are isolated"
echo ""

echo "=========================================="
echo "Custom Predicate Example Complete!"
echo "=========================================="
echo ""
echo "Summary:"
echo "  1. ✓ Created Rust predicate with wasm-bindgen in tmp/"
echo "  2. ✓ Built to WASM with wasm-pack"
echo "  3. ✓ Showed how to upload to contract"
echo "  4. ✓ Demonstrated usage in modal model"
echo "  5. ✓ Provided test cases"
echo "  6. ✓ Documented best practices"
echo ""
echo "Files created in tmp/:"
echo "  - tmp/custom-predicate/src/lib.rs"
echo "  - tmp/custom-predicate/Cargo.toml"
echo "  - tmp/custom-predicate/pkg/custom_predicate_bg.wasm (if built)"
echo "  - tmp/custom-contract.modality"
echo ""
echo "Your custom predicate is ready to use!"
echo ""

