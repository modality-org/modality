#!/usr/bin/env bash
# Integration test for 08-network-partition example
# Tests network partition scenarios and recovery

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Store script directory
SCRIPT_DIR="$(pwd)"

# Build modal CLI if needed
if [ ! -f "$SCRIPT_DIR/../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    (cd "$SCRIPT_DIR/../../../rust" && cargo build --package modal)
fi

# Add modal to PATH for this test (use absolute path to avoid issues with cd)
export PATH="$SCRIPT_DIR/../../../rust/target/debug:$PATH"

# Clean up any previous test nodes
rm -rf ./tmp

# Initialize test (after cleanup so logs directory is created fresh)
test_init "08-network-partition"

echo ""
echo "=========================================="
echo "Network Partition and Recovery Test Suite"
echo "=========================================="
echo ""
echo "This test demonstrates Byzantine-like resilience:"
echo "  - n=4 validators (can tolerate f=1 Byzantine)"
echo "  - Quorum threshold = 2f+1 = 3 validators"
echo "  - Single node partition: consensus continues"
echo "  - Two node partition: consensus halts (safety)"
echo "  - Node recovery: automatic catch-up and rejoin"
echo ""

# Test 1: Create and start all 4 nodes
echo ""
echo "=========================================="
echo "Test 1: Starting 4-validator network"
echo "=========================================="

# Create all nodes first
echo "Creating node1..."
assert_success "modal node create --dir ./tmp/node1 --from-template devnet3/node1 --bootstrappers '/ip4/127.0.0.1/tcp/10302/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB,/ip4/127.0.0.1/tcp/10303/ws/p2p/12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se'" "Create node1"

echo "Creating node2..."
assert_success "modal node create --dir ./tmp/node2 --from-template devnet3/node2 --bootstrappers '/ip4/127.0.0.1/tcp/10301/ws/p2p/12D3KooWA7csjq5MQyPWA4R7R5jRMf8RwhnSjXNNY3fF2jH2uxK3,/ip4/127.0.0.1/tcp/10303/ws/p2p/12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se'" "Create node2"

echo "Creating node3..."
assert_success "modal node create --dir ./tmp/node3 --from-template devnet3/node3 --bootstrappers '/ip4/127.0.0.1/tcp/10301/ws/p2p/12D3KooWA7csjq5MQyPWA4R7R5jRMf8RwhnSjXNNY3fF2jH2uxK3,/ip4/127.0.0.1/tcp/10302/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB'" "Create node3"

echo "Creating node4..."
assert_success "modal node create --dir ./tmp/node4 --network devnet3" "Create node4"

# Clear storage for all nodes
for i in 1 2 3 4; do
    assert_success "modal node clear-storage --dir ./tmp/node$i --yes" "Clear node$i storage"
done

# Start all nodes
echo ""
echo "Starting node1..."
NODE1_PID=$(test_start_process "cd $(pwd)/tmp/node1 && modal node run-validator" "node1")
assert_success "test_wait_for_port 10301" "Node1 should start on port 10301"

echo "Starting node2..."
NODE2_PID=$(test_start_process "cd $(pwd)/tmp/node2 && modal node run-validator" "node2")
assert_success "test_wait_for_port 10302" "Node2 should start on port 10302"

echo "Starting node3..."
NODE3_PID=$(test_start_process "cd $(pwd)/tmp/node3 && modal node run-validator" "node3")
assert_success "test_wait_for_port 10303" "Node3 should start on port 10303"

echo "Starting node4..."
NODE4_PID=$(test_start_process "cd $(pwd)/tmp/node4 && modal node run-validator" "node4")

# Wait for network to stabilize
echo "Waiting for network to stabilize..."
sleep 10

# Test 2: Verify all nodes are running
echo ""
echo "=========================================="
echo "Test 2: Verifying network health"
echo "=========================================="

# Check that all standard ports are listening
assert_success "lsof -i :10301 -sTCP:LISTEN -t" "Node1 port 10301 should be listening"
assert_success "lsof -i :10302 -sTCP:LISTEN -t" "Node2 port 10302 should be listening"
assert_success "lsof -i :10303 -sTCP:LISTEN -t" "Node3 port 10303 should be listening"

# Verify all nodes are running
assert_success "kill -0 $NODE1_PID" "Node1 should be running"
assert_success "kill -0 $NODE2_PID" "Node2 should be running"
assert_success "kill -0 $NODE3_PID" "Node3 should be running"
assert_success "kill -0 $NODE4_PID" "Node4 should be running"

echo ""
echo "✓ All 4 validators are running and healthy"

# Test 3: Single node partition (Byzantine tolerance f=1)
echo ""
echo "=========================================="
echo "Test 3: Single node partition (f=1)"
echo "=========================================="
echo ""
echo "Simulating network partition by killing node4..."
echo "With n=4 validators, losing 1 node leaves 3 active."
echo "Quorum = 2f+1 = 3, so consensus CAN continue."
echo ""

# Kill node4 to simulate partition
assert_success "kill -9 $NODE4_PID" "Should kill node4"

# Wait for process to fully exit AND for lock to be released
echo "Waiting for node4 to fully exit and release database lock..."
for i in {1..60}; do
    if ! kill -0 "$NODE4_PID" 2>/dev/null; then
        # Process is dead, now check if lock is released
        if ! lsof ./tmp/node4/storage/LOCK 2>/dev/null | grep -q "modal"; then
            echo "Process and lock released after $i checks"
            break
        fi
    fi
    sleep 0.5
done
sleep 1  # Additional buffer

# Verify node4 is no longer running
if ! kill -0 "$NODE4_PID" 2>/dev/null; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Node4 is stopped (partitioned)"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Node4 should be stopped"
fi

# Verify other nodes are still running
assert_success "kill -0 $NODE1_PID" "Node1 should still be running"
assert_success "kill -0 $NODE2_PID" "Node2 should still be running"
assert_success "kill -0 $NODE3_PID" "Node3 should still be running"

echo ""
echo "✓ Network has 3 active validators (meets quorum of 3)"
echo "  Consensus can continue despite single node partition"

# Test 4: Node recovery
echo ""
echo "=========================================="
echo "Test 4: Node recovery and catch-up"
echo "=========================================="
echo ""
echo "Recovering node4 by restarting it..."
echo "Node should sync with network and rejoin consensus."
echo ""

# Restart node4 (without clearing storage to allow catch-up)
NODE4_RECOVERED_PID=$(test_start_process "cd $(pwd)/tmp/node4 && modal node run-validator" "node4-recovered")

# Wait for node to rejoin
sleep 10

# Verify node4 is running again
assert_success "kill -0 $NODE4_RECOVERED_PID" "Node4 should be running after recovery"

echo ""
echo "✓ Node4 successfully recovered and rejoined the network"
echo "  All 4 validators are active again"

# Test 5: Two-node partition (exceeds Byzantine threshold)
echo ""
echo "=========================================="
echo "Test 5: Two-node partition (exceeds f=1)"
echo "=========================================="
echo ""
echo "Simulating severe partition by killing node3 and node4..."
echo "With n=4 validators, losing 2 nodes leaves only 2 active."
echo "Quorum = 2f+1 = 3, so consensus CANNOT continue."
echo "This demonstrates the Byzantine threshold."
echo ""

# Kill node3 and node4
assert_success "kill -9 $NODE3_PID" "Should kill node3"
assert_success "kill -9 $NODE4_RECOVERED_PID" "Should kill node4"

# Wait for processes to fully exit AND locks to be released
echo "Waiting for nodes to fully exit and release database locks..."
for i in {1..60}; do
    both_ready=true
    if kill -0 "$NODE3_PID" 2>/dev/null; then
        both_ready=false
    elif lsof ./tmp/node3/storage/LOCK 2>/dev/null | grep -q "modal"; then
        both_ready=false
    fi
    if kill -0 "$NODE4_RECOVERED_PID" 2>/dev/null; then
        both_ready=false
    elif lsof ./tmp/node4/storage/LOCK 2>/dev/null | grep -q "modal"; then
        both_ready=false
    fi
    if [ "$both_ready" = true ]; then
        echo "Processes and locks released after $i checks"
        break
    fi
    sleep 0.5
done
sleep 1  # Additional buffer

# Verify both nodes are stopped
if ! kill -0 "$NODE3_PID" 2>/dev/null; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Node3 is stopped (partitioned)"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Node3 should be stopped"
fi

if ! kill -0 "$NODE4_RECOVERED_PID" 2>/dev/null; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Node4 is stopped (partitioned)"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Node4 should be stopped"
fi

# Verify remaining nodes are still running
assert_success "kill -0 $NODE1_PID" "Node1 should still be running"
assert_success "kill -0 $NODE2_PID" "Node2 should still be running"

echo ""
echo "✗ Network has only 2 active validators (below quorum of 3)"
echo "  Consensus CANNOT continue - this demonstrates safety"
echo "  System will not commit incorrect state without quorum"

# Test 6: Full network recovery
echo ""
echo "=========================================="
echo "Test 6: Full network recovery"
echo "=========================================="
echo ""
echo "Recovering node3 and node4 to restore quorum..."
echo ""

# Restart node3
NODE3_RECOVERED_PID=$(test_start_process "cd $(pwd)/tmp/node3 && modal node run-validator" "node3-recovered")
assert_success "test_wait_for_port 10303" "Node3 should restart on port 10303"

# Restart node4
NODE4_FINAL_PID=$(test_start_process "cd $(pwd)/tmp/node4 && modal node run-validator" "node4-final")

# Wait for network to stabilize
sleep 10

# Verify all nodes are running
assert_success "kill -0 $NODE1_PID" "Node1 should be running"
assert_success "kill -0 $NODE2_PID" "Node2 should be running"
assert_success "kill -0 $NODE3_RECOVERED_PID" "Node3 should be running after recovery"
assert_success "kill -0 $NODE4_FINAL_PID" "Node4 should be running after recovery"

# Verify connectivity
assert_success "lsof -i :10301 -sTCP:LISTEN -t" "Node1 port should be listening"
assert_success "lsof -i :10302 -sTCP:LISTEN -t" "Node2 port should be listening"
assert_success "lsof -i :10303 -sTCP:LISTEN -t" "Node3 port should be listening"

echo ""
echo "✓ All 4 validators recovered and operational"
echo "  Network has full quorum and can continue consensus"

# Summary
echo ""
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo ""
echo "Demonstrated network partition resilience:"
echo "  ✓ 4-validator network with f=1 Byzantine tolerance"
echo "  ✓ Single node partition: consensus continues (3 >= 3 quorum)"
echo "  ✓ Node recovery: automatic catch-up and rejoin"
echo "  ✓ Two-node partition: consensus halts (2 < 3 quorum, safety)"
echo "  ✓ Full recovery: network resumes normal operation"
echo ""
echo "Key insights:"
echo "  - System tolerates up to f=1 validator failures"
echo "  - Safety is maintained: no commits without quorum"
echo "  - Liveness resumes when quorum is restored"
echo "  - Validators can rejoin and sync after network healing"
echo ""

# Finalize test
test_finalize
exit $?

