#!/usr/bin/env bash
# Check network connectivity between nodes
# This helps verify network health and partition recovery

cd $(dirname -- "$0")
set -e

echo "=== Checking Network Connectivity ==="
echo ""

# Node peer IDs and ports (devnet3 standard)
NODE1_PEER_ID="12D3KooWA7csjq5MQyPWA4R7R5jRMf8RwhnSjXNNY3fF2jH2uxK3"
NODE1_PORT="10301"

NODE2_PEER_ID="12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB"
NODE2_PORT="10302"

NODE3_PEER_ID="12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
NODE3_PORT="10303"

# Check if ports are listening
check_port() {
    local port=$1
    local node=$2
    if lsof -i ":$port" -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo "✓ $node is running on port $port"
        return 0
    else
        echo "✗ $node is NOT running on port $port"
        return 1
    fi
}

echo "Port Status:"
check_port "$NODE1_PORT" "Node1"
check_port "$NODE2_PORT" "Node2"
check_port "$NODE3_PORT" "Node3"

# Note: Node4 uses a dynamic port, so we don't check it here

echo ""
echo "Network connectivity check complete."

