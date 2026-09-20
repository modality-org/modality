#!/bin/bash
# Build standard predicates to WASM modules
# Each predicate is compiled separately to its own WASM file

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/.."
WASM_DIR="$PROJECT_ROOT/build/wasm/predicates"

echo "Building standard predicates to WASM..."

# Create output directory
mkdir -p "$WASM_DIR"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed"
    echo "Install it with: cargo install wasm-pack"
    exit 1
fi

cd "$PROJECT_ROOT/rust/modal-wasm-validation"

# Build the entire package for wasm32
echo "Building modal-wasm-validation for wasm32-unknown-unknown..."
cargo build --target wasm32-unknown-unknown --release

# The compiled WASM is in target/wasm32-unknown-unknown/release/
WASM_FILE="$PROJECT_ROOT/rust/target/wasm32-unknown-unknown/release/modal_wasm_validation.wasm"

if [ ! -f "$WASM_FILE" ]; then
    echo "Error: WASM file not found at $WASM_FILE"
    exit 1
fi

# For now, we'll use the same WASM file for all predicates
# In a more advanced setup, we could compile each predicate separately
# by using different entry points or separate crates

echo "Copying WASM modules to $WASM_DIR..."
cp "$WASM_FILE" "$WASM_DIR/signed_by.wasm"
cp "$WASM_FILE" "$WASM_DIR/amount_in_range.wasm"
cp "$WASM_FILE" "$WASM_DIR/has_property.wasm"
cp "$WASM_FILE" "$WASM_DIR/timestamp_valid.wasm"
cp "$WASM_FILE" "$WASM_DIR/post_to_path.wasm"
# Text predicates
cp "$WASM_FILE" "$WASM_DIR/text_equals.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_equals_ignore_case.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_contains.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_starts_with.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_ends_with.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_is_empty.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_not_empty.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_length_eq.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_length_gt.wasm"
cp "$WASM_FILE" "$WASM_DIR/text_length_lt.wasm"

echo "✓ Built predicate WASM modules:"
ls -lh "$WASM_DIR"/*.wasm

echo ""
echo "NOTE: Currently all predicates use the same WASM module."
echo "To call a specific predicate, use its evaluate_* function."
echo "Example: evaluate_signed_by, evaluate_amount_in_range, etc."

