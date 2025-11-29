#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)

# Load test library
source ../test-lib.sh

echo "========================================"
echo "Running 08-js-sdk test"
echo "========================================"
echo ""

# Clean up any existing processes and data
cleanup() {
    echo "Cleaning up..."
    
    # Stop node if running - try modal node kill first
    if [ -d "./tmp/node1" ] && command -v modal &> /dev/null; then
        modal node kill --dir ./tmp/node1 2>/dev/null || true
    elif [ -f "./tmp/node1.pid" ]; then
        NODE_PID=$(cat ./tmp/node1.pid)
        if ps -p ${NODE_PID} > /dev/null 2>&1; then
            kill ${NODE_PID} 2>/dev/null || true
            sleep 1
            kill -9 ${NODE_PID} 2>/dev/null || true
        fi
        rm -f ./tmp/node1.pid
    fi
    
    # Clean up temp files
    rm -rf ./tmp
}

# Set up cleanup trap
trap cleanup EXIT

# Clean start
cleanup

echo "Test 1: Starting devnet1 node..."
./01-start-devnet1.sh
if [ $? -ne 0 ]; then
    echo "✗ Failed to start devnet1 node"
    exit 1
fi
echo "✓ devnet1 node started"
echo ""

# Wait a bit longer for node to be fully ready
echo "Waiting for node to be fully initialized..."
sleep 5

# Check if node is actually running
if [ ! -f "./tmp/node1.pid" ]; then
    echo "✗ Node PID file not found"
    exit 1
fi

NODE_PID=$(cat ./tmp/node1.pid)
if ! ps -p ${NODE_PID} > /dev/null 2>&1; then
    echo "✗ Node process is not running (PID: ${NODE_PID})"
    cat ./tmp/node1-output.log || true
    exit 1
fi

echo "✓ Node confirmed running (PID: ${NODE_PID})"
echo ""

# Verify node is listening on WebSocket
echo "Verifying node is listening on port 10101..."
if command -v lsof >/dev/null 2>&1; then
    lsof -i :10101 || echo "  (lsof shows no process, but node may still be starting...)"
elif command -v netstat >/dev/null 2>&1; then
    netstat -an | grep 10101 || echo "  (netstat shows no listener, but node may still be starting...)"
fi
echo ""

echo "Test 2: Connecting with JavaScript SDK..."
node 03-connect-sdk.js
if [ $? -ne 0 ]; then
    echo "✗ JavaScript SDK connection failed"
    echo ""
    echo "Node logs:"
    cat ./tmp/node1-output.log || echo "No logs available"
    exit 1
fi
echo "✓ JavaScript SDK connection successful"
echo ""

echo "Test 3: Stopping devnet1 node..."
./02-stop-devnet1.sh
if [ $? -ne 0 ]; then
    echo "✗ Failed to stop devnet1 node"
    exit 1
fi
echo "✓ devnet1 node stopped"
echo ""

echo "========================================"
echo "✓ All tests passed!"
echo "========================================"

