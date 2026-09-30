#!/bin/bash
set -e

echo "=== Building Simple Program ==="

PROGRAM_DIR="./tmp/simple_program"

if [ ! -d "$PROGRAM_DIR" ]; then
    echo "Error: Program directory not found. Run ./01-create-program.sh first."
    exit 1
fi

cd "$PROGRAM_DIR"

echo "Building WASM program..."
./build.sh

echo ""
echo "✓ Program built successfully"
echo "  Output: target/wasm32-unknown-unknown/release/simple_program.wasm"
echo ""
echo "Next: Run ./03-upload-program.sh to upload to a contract"

