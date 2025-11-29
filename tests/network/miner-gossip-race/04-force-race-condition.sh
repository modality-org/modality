#!/usr/bin/env bash
# Force race condition by starting both miners simultaneously from shared genesis
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

echo "=== Forcing Race Condition Test ==="
echo ""

# Clean up first
./00-clean.sh

echo ""
echo "Step 1: Creating shared genesis block..."

# Create miner1 and mine a genesis block
modal node create --dir "${SCRIPT_DIR}/tmp/miner1"

# Configure miner1
cat > "${SCRIPT_DIR}/tmp/miner1/config.json" << 'EOF'
{
  "passfile_path": "./node.passfile",
  "storage_path": "./storage",
  "listeners": ["/ip4/0.0.0.0/tcp/10401/ws"],
  "bootstrappers": [],
  "run_miner": true,
  "initial_difficulty": 1,
  "status_port": 8401
}
EOF

# Start miner1 briefly to create genesis
echo "Mining genesis block..."
timeout 30 modal node run-miner --dir ./tmp/miner1 2>&1 | grep -m1 "Successfully mined and gossipped block 0" || true
sleep 2

# Kill miner1 specifically (we need just miner1, not all nodes in dir)
pkill -f "modal node run-miner.*tmp/miner1" || true
sleep 2

# Verify genesis was created
if [ ! -d "./tmp/miner1/storage" ]; then
    echo "❌ Failed to create genesis block"
    exit 1
fi

echo "✓ Genesis block created"

echo ""
echo "Step 2: Copying genesis to miner2..."

# Create miner2 and copy the same storage
modal node create --dir "${SCRIPT_DIR}/tmp/miner2"

# Get miner1's peer ID for bootstrapping
MINER1_PEER_ID=$(jq -r '.id' ./tmp/miner1/config.json)

# Configure miner2
cat > "${SCRIPT_DIR}/tmp/miner2/config.json" << EOF
{
  "passfile_path": "./node.passfile",
  "storage_path": "./storage",
  "listeners": ["/ip4/0.0.0.0/tcp/10402/ws"],
  "bootstrappers": [
    "/ip4/127.0.0.1/tcp/10401/ws/p2p/${MINER1_PEER_ID}"
  ],
  "run_miner": true,
  "initial_difficulty": 1,
  "status_port": 8402
}
EOF

# Copy the genesis storage from miner1 to miner2 (same starting point!)
cp -r ./tmp/miner1/storage ./tmp/miner2/

echo "✓ Both miners now have identical genesis block"

echo ""
echo "Step 3: Starting both miners SIMULTANEOUSLY..."
echo "⚠️  This should trigger the race condition for block 1"

# Start both miners at the same time
modal node run-miner --dir ./tmp/miner1 2>&1 | tee /tmp/miner1-force.log &
MINER1_PID=$!

modal node run-miner --dir ./tmp/miner2 2>&1 | tee /tmp/miner2-force.log &
MINER2_PID=$!

echo ""
echo "✓ Both miners started (PIDs: $MINER1_PID, $MINER2_PID)"
echo ""
echo "Watching for race condition..."
echo "Press Ctrl+C to stop"
echo ""

# Monitor both logs for the race condition
sleep 5

# Function to check for race condition
check_race_condition() {
    local log=$1
    local miner=$2
    
    if grep -q "rejected by fork choice rules" "$log" 2>/dev/null; then
        echo "🎯 RACE CONDITION DETECTED in $miner!"
        echo ""
        echo "Log excerpt:"
        grep -A 2 -B 2 "rejected by fork choice rules" "$log"
        echo ""
        return 0
    fi
    return 1
}

# Monitor for 60 seconds
for i in {1..60}; do
    if check_race_condition /tmp/miner1-force.log "Miner 1"; then
        echo "✅ Successfully forced race condition!"
        break
    fi
    if check_race_condition /tmp/miner2-force.log "Miner 2"; then
        echo "✅ Successfully forced race condition!"
        break
    fi
    
    # Show progress
    if [ $((i % 5)) -eq 0 ]; then
        echo "[$i/60] Still monitoring..."
        
        # Show what blocks each miner has
        MINER1_BLOCKS=$(grep -c "Successfully mined and gossipped block" /tmp/miner1-force.log 2>/dev/null || echo "0")
        MINER2_BLOCKS=$(grep -c "Successfully mined and gossipped block" /tmp/miner2-force.log 2>/dev/null || echo "0")
        echo "  Miner1: $MINER1_BLOCKS blocks, Miner2: $MINER2_BLOCKS blocks"
    fi
    
    sleep 1
done

echo ""
echo "Test complete. Cleaning up..."
kill $MINER1_PID $MINER2_PID 2>/dev/null || true

echo ""
echo "Full logs available at:"
echo "  Miner1: /tmp/miner1-force.log"
echo "  Miner2: /tmp/miner2-force.log"

