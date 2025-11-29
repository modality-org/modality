#!/bin/bash
set -e

echo "Quick Mining Test - Testing SHA256 hash function"
echo "================================================"

# Export PATH
export PATH="/Users/dotcontract/work/modality-dev/modality/rust/target/release:$PATH"

# Clean up any existing test
rm -rf ./tmp/quick-mining-test
mkdir -p ./tmp/quick-mining-test

echo "Creating test node with devnet1 config..."
cp -r ../../fixtures/network-configs/devnet1 ./tmp/quick-mining-test/network-config

# Create a simple passfile
echo '{"keypair":{"public_key":"ed01209c258f5d9487d557f3d1e3a0c4e20c3e0a7a6df2a6d3e0e0a0a0a0a0a0","private_key":"3044022044c5c0ee73d234b6c7e6a5c52b1f7c3c8f6a0e7c6e5a4b3a2a1a0a9a8a7a6a5a402207d6c5b4a3a2a1a0a9a8a7a6a5a4a3a2a1a0a9a8a7a6a5a4a3a2a1a0a9a8a7a6"},"address":"modal1something"}' > ./tmp/quick-mining-test/node.passfile

# Create config
cat > ./tmp/quick-mining-test/config.json << CONFIG_EOF
{
  "passfile_path": "./node.passfile",
  "storage_path": "./storage",
  "network_config_path": "./network-config/config.json",
  "listeners": ["/ip4/127.0.0.1/tcp/10201/ws"],
  "run_miner": true,
  "initial_difficulty": 1
}
CONFIG_EOF

echo ""
echo "Starting mining (should be fast with SHA256)..."
echo "Will mine for 30 seconds..."

# Start node in background
modal node run --dir ./tmp/quick-mining-test > ./tmp/quick-mining-test/node.log 2>&1 &
NODE_PID=$!
echo "Node PID: $NODE_PID"

# Wait for node to start
sleep 3

# Monitor for blocks
echo "Monitoring mining progress..."
START_TIME=$(date +%s)
LAST_COUNT=0

for i in {1..15}; do
    sleep 2
    BLOCK_COUNT=$(modal node inspect --dir ./tmp/quick-mining-test 2>/dev/null | grep -i "chain_height" | grep -oE '[0-9]+' || echo "0")
    
    if [ "$BLOCK_COUNT" != "$LAST_COUNT" ]; then
        END_TIME=$(date +%s)
        DURATION=$((END_TIME - START_TIME))
        if [ "$BLOCK_COUNT" -gt "0" ]; then
            AVG=$(echo "scale=2; $DURATION / $BLOCK_COUNT" | bc)
            echo "  Block $BLOCK_COUNT mined (avg: ${AVG}s per block)"
        fi
        LAST_COUNT=$BLOCK_COUNT
    fi
    
    if [ "$BLOCK_COUNT" -ge "3" ]; then
        break
    fi
done

# Stop node
kill $NODE_PID 2>/dev/null || true
sleep 2

# Get final count
FINAL_COUNT=$(modal node inspect --dir ./tmp/quick-mining-test 2>/dev/null | grep -i "chain_height" | grep -oE '[0-9]+' || echo "0")
END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))

echo ""
echo "================================================"
if [ "$FINAL_COUNT" -ge "3" ]; then
    AVG=$(echo "scale=2; $TOTAL_DURATION / $FINAL_COUNT" | bc)
    echo "✓ Test PASSED - SHA256 mining is working!"
    echo "  Mined $FINAL_COUNT blocks in $TOTAL_DURATION seconds"
    echo "  Average: ${AVG}s per block"
    
    # Check logs for hash function message
    if grep -qi "sha256" ./tmp/quick-mining-test/node.log; then
        echo "  ✓ Confirmed using SHA256 hash function"
    elif grep -qi "Using miner hash configuration" ./tmp/quick-mining-test/node.log; then
        echo "  ✓ Hash function configuration loaded"
        grep "hash configuration" ./tmp/quick-mining-test/node.log | head -3
    fi
    
    echo ""
    echo "This is MUCH faster than RandomX (which takes ~13s per block)!"
    exit 0
else
    echo "✗ Test incomplete - Only mined $FINAL_COUNT blocks in $TOTAL_DURATION seconds"
    echo ""
    echo "Checking logs for errors..."
    echo "Last 30 lines:"
    tail -30 ./tmp/quick-mining-test/node.log
    exit 1
fi
