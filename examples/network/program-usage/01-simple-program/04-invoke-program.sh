#!/bin/bash
set -e

echo "=== Invoking Program ==="

CONTRACT_DIR="./tmp/test_contract"

if [ ! -d "$CONTRACT_DIR" ]; then
    echo "Error: Contract not found. Run ./03-upload-program.sh first."
    exit 1
fi

# Invoke program with arguments
echo "Creating invoke commit..."
modal contract commit \
    --dir "$CONTRACT_DIR" \
    --method invoke \
    --path "/__programs__/simple_program.wasm" \
    --value '{"args": {"message": "Hello from program", "count": 42}}'

echo ""
echo "✓ Program invoked successfully"
echo ""
echo "The program will:"
echo "  1. Post message to /data/message"
echo "  2. Post count to /data/count"
echo "  3. Post timestamp to /data/executed_at"
echo ""
echo "Note: Program execution happens on validators during consensus"
echo "      In a local-only setup, push to a running validator to see results"

