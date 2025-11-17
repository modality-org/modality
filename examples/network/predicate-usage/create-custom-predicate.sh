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

# Step 1: Create a custom predicate using modal CLI
echo "Step 1: Creating a custom predicate project..."
echo ""

# Remove existing directory if it exists
rm -rf "$SCRIPT_DIR/tmp/custom-predicate"

# Use modal predicate create to scaffold the project
modal predicate create --dir "$SCRIPT_DIR/tmp/custom-predicate" --name is_within_percent

echo ""
echo "✓ Created predicate project with modal predicate create"
echo ""

# Step 1b: Implement custom logic
echo "Step 1b: Implementing custom predicate logic..."
echo ""

# Replace the default lib.rs with our custom implementation
cat > "$SCRIPT_DIR/tmp/custom-predicate/src/lib.rs" << 'EOF'
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Input structure for predicates
#[derive(Debug, Deserialize)]
struct PredicateInput {
    data: Value,
    context: PredicateContext,
}

/// Context passed to predicates during evaluation
#[derive(Debug, Deserialize)]
struct PredicateContext {
    contract_id: String,
    block_height: u64,
    timestamp: u64,
}

/// Result of predicate evaluation
#[derive(Debug, Serialize)]
struct PredicateResult {
    valid: bool,
    gas_used: u64,
    errors: Vec<String>,
}

impl PredicateResult {
    fn success(gas_used: u64) -> Self {
        Self {
            valid: true,
            gas_used,
            errors: Vec::new(),
        }
    }

    fn failure(gas_used: u64, errors: Vec<String>) -> Self {
        Self {
            valid: false,
            gas_used,
            errors,
        }
    }

    fn error(gas_used: u64, error: String) -> Self {
        Self {
            valid: false,
            gas_used,
            errors: vec![error],
        }
    }
}

/// Custom predicate: check if a value is within a percentage range of a target
/// 
/// Example: is_within_percent({"value": 105, "target": 100, "percent": 10})
/// Returns true because 105 is within 10% of 100 (90-110)
#[wasm_bindgen]
pub fn evaluate(input_json: &str) -> String {
    let gas_used = 25;

    let input: PredicateInput = match serde_json::from_str(input_json) {
        Ok(i) => i,
        Err(e) => {
            let result = PredicateResult::error(gas_used, format!("Invalid input: {}", e));
            return serde_json::to_string(&result).unwrap();
        }
    };

    // Extract parameters
    let value = match input.data.get("value").and_then(|v| v.as_f64()) {
        Some(v) => v,
        None => {
            let result = PredicateResult::error(gas_used, "Missing or invalid 'value' parameter".to_string());
            return serde_json::to_string(&result).unwrap();
        }
    };

    let target = match input.data.get("target").and_then(|v| v.as_f64()) {
        Some(t) => t,
        None => {
            let result = PredicateResult::error(gas_used, "Missing or invalid 'target' parameter".to_string());
            return serde_json::to_string(&result).unwrap();
        }
    };

    let percent = match input.data.get("percent").and_then(|v| v.as_f64()) {
        Some(p) => p,
        None => {
            let result = PredicateResult::error(gas_used, "Missing or invalid 'percent' parameter".to_string());
            return serde_json::to_string(&result).unwrap();
        }
    };

    // Calculate range
    let tolerance = target * (percent / 100.0);
    let min = target - tolerance;
    let max = target + tolerance;

    // Check if value is within range
    let valid = value >= min && value <= max;

    if valid {
        let result = PredicateResult::success(gas_used + 50);
        serde_json::to_string(&result).unwrap()
    } else {
        let result = PredicateResult::failure(
            gas_used + 50,
            vec![format!(
                "Value {} is not within {}% of {} (range: {}-{})",
                value, percent, target, min, max
            )]
        );
        serde_json::to_string(&result).unwrap()
    }
}
EOF

echo "✓ Implemented custom predicate: is_within_percent"
echo ""
echo "Predicate logic:"
echo "  - Checks if a value is within a percentage of a target"
echo "  - Example: 105 is within 10% of 100 (range 90-110)"
echo ""

# Step 2: Build the predicate
echo "Step 2: Building WASM module..."
echo ""

cd "$SCRIPT_DIR/tmp/custom-predicate"

# Use the generated build script
if [ -f "./build.sh" ]; then
    echo "Using generated build.sh script..."
    ./build.sh
    echo ""
else
    echo "⚠️  build.sh not found, building manually..."
    if command -v wasm-pack &> /dev/null; then
        echo "Building with wasm-pack..."
        wasm-pack build --target web --release
    else
        echo "Building with cargo..."
        cargo build --target wasm32-unknown-unknown --release
    fi
    echo ""
fi

# Detect WASM file location (build.sh normalizes to dist/)
if [ -f "dist/is_within_percent.wasm" ]; then
    WASM_FILE="dist/is_within_percent.wasm"
    echo "✓ Built WASM module: $WASM_FILE"
else
    echo "⚠️  WASM file not found. Build may have failed."
    WASM_FILE=""
fi

if [ -n "$WASM_FILE" ]; then
    echo "  Size: $(wc -c < "$WASM_FILE") bytes"
    echo ""
fi

# Step 3: Upload to contract
echo "Step 3: Uploading to contract..."
echo ""
echo "To upload your custom predicate:"
echo ""
echo "  # Convert WASM to base64"
echo "  WASM_BASE64=\$(base64 < tmp/custom-predicate/$WASM_FILE)"
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
echo "  1. ✓ Created predicate project with 'modal predicate create'"
echo "  2. ✓ Implemented custom is_within_percent logic"
echo "  3. ✓ Built to WASM using generated build.sh"
echo "  4. ✓ Showed how to upload to contract"
echo "  5. ✓ Demonstrated usage in modal model"
echo "  6. ✓ Provided test cases and best practices"
echo ""
echo "Files created in tmp/custom-predicate/:"
echo "  - Cargo.toml (generated)"
echo "  - package.json (generated)"
echo "  - build.sh (generated)"
echo "  - README.md (generated)"
echo "  - src/lib.rs (custom implementation)"
echo "  - tests/lib.rs (generated)"
if [ -n "$WASM_FILE" ]; then
    echo "  - $WASM_FILE (built)"
fi
echo ""
echo "Also created:"
echo "  - tmp/custom-contract.modality (example usage)"
echo ""
echo "Your custom predicate is ready to use!"
echo ""
echo "To create your own predicate:"
echo "  modal predicate create --dir my-predicate --name my_predicate"
echo ""

