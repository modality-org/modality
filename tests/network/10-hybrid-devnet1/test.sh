#!/usr/bin/env bash
# Bounded solo hybrid smoke: mining nominations activate Shoal at epoch 2.
set -euo pipefail
cd "$(dirname "$0")"
source ../test-lib.sh
command -v modal >/dev/null || rebuild
rm -rf ./tmp
test_init "10-hybrid-devnet1"

NODE_PID=$(test_start_process "RUST_LOG=info ./01-run-hybrid-node.sh" "node1")
NODE_LOG="$LOG_DIR/${CURRENT_TEST}_node1.log"
assert_success "test_wait_for_port 10111 30" "Hybrid node listens"
assert_success "test_wait_for_log '$NODE_LOG' 'Successfully mined and gossipped block' 120" "Hybrid node mines"
assert_success "test_wait_for_log '$NODE_LOG' 'EPOCH 2 STARTED' 120" "Mining reaches epoch 2"
assert_success "test_wait_for_log '$NODE_LOG' 'This node IS a sequencer for epoch 2' 60" "Epoch-0 nomination activates sequencer"
assert_success "test_wait_for_log '$NODE_LOG' 'Starting Shoal consensus loop' 60" "Shoal consensus starts"
assert_success "kill -0 $NODE_PID" "Hybrid process remains running"
test_finalize
