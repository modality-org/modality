#!/bin/bash
set -e

echo "=== Creating Simple Program ==="

# Create program in tmp directory
PROGRAM_DIR="./tmp/simple_program"

if [ -d "$PROGRAM_DIR" ]; then
    echo "Removing existing program directory..."
    rm -rf "$PROGRAM_DIR"
fi

echo "Creating program project..."
modal program create --dir "$PROGRAM_DIR" --name simple_program

echo ""
echo "✓ Program project created at $PROGRAM_DIR"
echo ""
echo "Next: Edit the program logic in $PROGRAM_DIR/src/lib.rs"
echo "      Then run ./02-build-program.sh"

