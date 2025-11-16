#!/usr/bin/env bash
set -e

echo "Verifying parameter values..."

# Check if parameters were stored (they should be after round 0 is processed)
if [ -f ./tmp/test-network-params/name.txt ]; then
    NAME=$(cat ./tmp/test-network-params/name.txt)
    echo "  Network name: $NAME"
    
    if echo "$NAME" | grep -q "devnet1"; then
        echo "  ✓ Name contains 'devnet1'"
    else
        echo "  ✗ Name does not contain 'devnet1'"
        exit 1
    fi
fi

if [ -f ./tmp/test-network-params/difficulty.txt ]; then
    DIFFICULTY=$(cat ./tmp/test-network-params/difficulty.txt)
    echo "  Difficulty: $DIFFICULTY"
    
    if echo "$DIFFICULTY" | grep -q "1"; then
        echo "  ✓ Difficulty is 1"
    else
        echo "  ✗ Difficulty is not 1"
        exit 1
    fi
fi

if [ -f ./tmp/test-network-params/validator0.txt ]; then
    VALIDATOR=$(cat ./tmp/test-network-params/validator0.txt)
    echo "  Validator 0: $VALIDATOR"
    
    if echo "$VALIDATOR" | grep -q "12D3KooW"; then
        echo "  ✓ Validator ID looks valid"
    else
        echo "  ✗ Validator ID doesn't look valid"
        exit 1
    fi
fi

echo ""
echo "✓ All parameter values verified"

