#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 7: Stop Validator"
echo "================================================"
echo ""

# Try using modal node kill first if the node directory exists
if [ -d "tmp/validator-node" ] && command -v modal &> /dev/null; then
    echo "Stopping validator using modal node kill..."
    modal node kill --dir tmp/validator-node 2>/dev/null && echo "✅ Validator stopped" && exit 0 || echo "Trying alternative methods..."
fi

# Fallback to PID file method
if [ -f "tmp/validator.pid" ]; then
    VALIDATOR_PID=$(cat tmp/validator.pid)
    echo "Stopping validator (PID: $VALIDATOR_PID)..."
    kill $VALIDATOR_PID 2>/dev/null || true
    rm tmp/validator.pid
    echo "✅ Validator stopped"
else
    echo "No validator PID file found, checking port 10101..."
    PID=$(lsof -i :10101 -sTCP:LISTEN -t 2>/dev/null || echo "")
    if [ -n "$PID" ]; then
        echo "Stopping validator on port 10101 (PID: $PID)..."
        kill $PID 2>/dev/null || true
        echo "✅ Validator stopped"
    else
        echo "No validator running on port 10101"
    fi
fi

echo ""

