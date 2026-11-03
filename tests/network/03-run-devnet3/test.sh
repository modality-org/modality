#!/usr/bin/env bash
# Integration test for 03-run-devnet3 example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Build modal CLI if needed
command -v modal &> /dev/null || rebuild

# Clean up any previous test nodes
rm -rf ./tmp

# Initialize test (after cleanup so logs directory is created fresh)
test_init "03-run-devnet3"

# Test 1: Start node1
echo ""
echo "Test 1: Starting node1..."
NODE1_PID=$(test_start_process "cd $(pwd) && ./01-run-node1.sh" "node1")
assert_success "test_wait_for_port 10301" "Node1 should start on port 10301"
sleep 2

# Test 2: Start node2
echo ""
echo "Test 2: Starting node2..."
NODE2_PID=$(test_start_process "cd $(pwd) && ./02-run-node2.sh" "node2")
assert_success "test_wait_for_port 10302" "Node2 should start on port 10302"
sleep 2

# Test 3: Start node3
echo ""
echo "Test 3: Starting node3..."
NODE3_PID=$(test_start_process "cd $(pwd) && ./03-run-node3.sh" "node3")
assert_success "test_wait_for_port 10303" "Node3 should start on port 10303"
sleep 3

# Test 4: Verify all nodes are still running
echo ""
echo "Test 4: Verifying all nodes are running..."
assert_success "kill -0 $NODE1_PID" "Node1 should still be running"
assert_success "kill -0 $NODE2_PID" "Node2 should still be running"
assert_success "kill -0 $NODE3_PID" "Node3 should still be running"

# Test 5: Prove both advertised bootstrap addresses answer a request. The
# ping CLI uses an ephemeral client, so this checks reachability, not a
# persistent connection between the sequencers.
echo ""
echo "Test 5: Pinging both advertised bootstrap addresses..."
ping_with_retry() {
    local target="$1"
    local attempt
    for attempt in 1 2 3 4 5; do
        if modal node ping --dir ./tmp/node1 --target "$target" --times 1; then
            return 0
        fi
        sleep 2
    done
    return 1
}
assert_success \
    "ping_with_retry /ip4/127.0.0.1/tcp/10302/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB" \
    "Node2 advertised address should answer ping"
assert_success \
    "ping_with_retry /ip4/127.0.0.1/tcp/10303/ws/p2p/12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se" \
    "Node3 advertised address should answer ping"

# Finalize test
test_finalize
exit $?

