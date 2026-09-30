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
    --value '{"args": {"message": "Hello from program"}}'

echo ""
echo "✓ Program invoked successfully"
echo ""
echo "The program will:"
echo "  1. Post the message to /data/message.text"
echo "  2. Count its runs at /data/runs.num"
echo ""
echo "Note: Program execution happens on sequencers during consensus"
echo "      In a local-only setup, push to a running sequencer to see results"

