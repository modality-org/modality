#!/bin/bash
set -e

echo "=== Uploading Program to Contract ==="

PROGRAM_DIR="./tmp/simple_program"
CONTRACT_DIR="./tmp/test_contract"

# Create contract if it doesn't exist
if [ ! -d "$CONTRACT_DIR" ]; then
    echo "Creating test contract..."
    modal contract create --dir "$CONTRACT_DIR"
fi

# Upload program
echo "Uploading program to contract..."
modal program upload \
    "$PROGRAM_DIR/pkg/simple_program_bg.wasm" \
    --dir "$CONTRACT_DIR" \
    --name simple_program \
    --gas-limit 1000000

echo ""
echo "✓ Program uploaded to /__programs__/simple_program.wasm"
echo ""
echo "Next: Run ./04-invoke-program.sh to invoke the program"

