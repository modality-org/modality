#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 7: Stop Sequencer"
echo "================================================"
echo ""

# Try using modal node kill first if the node directory exists
if [ -d "tmp/node1" ] && command -v modal &> /dev/null; then
    echo "Stopping sequencer using modal node kill..."
    modal node kill --dir tmp/node1 2>/dev/null && echo "✅ Sequencer stopped" && exit 0 || echo "Unable to kill node1..."
fi

echo ""

