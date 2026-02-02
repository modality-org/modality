# Miner Gossip Race Condition Test - Summary

## Overview

Created a comprehensive test example in `examples/network/miner-gossip-race/` that reproduces and demonstrates the mining race condition where a locally mined block gets rejected due to fork choice "first-seen" rules.

## Problem Being Tested

When multiple miners compete for the same block:
1. Miner A starts mining block N
2. Miner B also starts mining block N
3. Miner A finishes first and gossips their block
4. Miner B receives the gossiped block while still mining
5. Miner B finishes mining and tries to save their block
6. **Fork choice rejects Miner B's block** with: "Invalid block: Mined block was rejected by fork choice rules"
7. Miner B wastes computational effort and gets stuck in a retry loop

## Files Created

### `/examples/network/miner-gossip-race/README.md`
Comprehensive documentation explaining:
- The race condition problem
- Current vs. desired behavior
- Test scenario setup
- How to run the test manually and automatically
- Log patterns to look for
- Potential fixes (3 different approaches)
- Troubleshooting guide

### `/examples/network/miner-gossip-race/00-clean.sh`
Cleanup script that:
- Kills any running miner processes
- Removes temporary files and storage
- Prepares for a fresh test run

### `/examples/network/miner-gossip-race/01-run-miner1.sh`
Miner 1 setup script that:
- Creates a miner node with low difficulty (difficulty=1)
- Configures it to mine on port 10401
- Runs standalone (no bootstrappers)
- Starts mining blocks

### `/examples/network/miner-gossip-race/02-run-miner2.sh`
Miner 2 setup script that:
- Creates a second miner node with low difficulty (difficulty=1)
- Configures it to bootstrap from Miner 1
- Runs on port 10402
- Competes with Miner 1 for blocks (race condition trigger)

### `/examples/network/miner-gossip-race/03-manual-test-instructions.sh`
Helper script that prints instructions for running the test manually in two terminals to observe the race condition in real-time.

### `/examples/network/miner-gossip-race/test.sh`
Automated integration test that:
1. Creates and configures both miners
2. Starts them sequentially (Miner 1 first, then Miner 2)
3. Waits for miners to connect and compete
4. **Monitors logs for race condition patterns:**
   - "rejected by fork choice rules"
   - "Correcting mining index"
   - "already exists in chain, skipping mining"
5. Verifies both miners recover and continue mining
6. Checks chain synchronization between miners
7. Reports statistics on race condition occurrences
8. Uses the test-lib.sh framework for consistent testing

## How to Run

### Automated Test
```bash
cd examples/network/miner-gossip-race
./test.sh
```

### Manual Test (observe in real-time)
```bash
# Terminal 1
./01-run-miner1.sh

# Terminal 2 (after miner1 starts)
./02-run-miner2.sh
```

Watch for the error messages in Terminal 2:
- `⚠️  Failed to mine block X (Invalid block: Mined block was rejected by fork choice rules)`
- `⛏️  Correcting mining index from X to Y after error`
- `Block X already exists in chain, skipping mining`

## Expected Test Results

The test is designed to demonstrate and measure the race condition:

1. **Race Condition Detection** - Test checks if the fork choice rejection appears in logs
2. **Mining Recovery** - Verifies that miners recover from the rejection
3. **Chain Synchronization** - Confirms both miners end up with synchronized chains
4. **Statistics Reporting** - Reports how many times the race condition occurred:
   - Fork choice rejections count
   - Mining corrections count
   - Block skips (wasted effort) count

## Integration with Test Suite

The test can be run:
- Standalone via `./test.sh`
- As part of the full test suite via `../test-numbered-examples.sh` (once promoted to numbered example)
- Via individual manual scripts for debugging

## Potential Fixes Documented

The README documents three potential fixes:

### Fix 1: Pre-Mining Check
Check if block exists before starting expensive PoW:
```rust
let exists = {
    let ds = datastore.lock().await;
    MinerBlock::find_canonical_by_index(&ds, index).await?.is_some()
};
if exists {
    log::info!("Block {} already exists, skipping mining", index);
    return Ok(());
}
```

### Fix 2: Interruptible Mining
Allow mining to be interrupted when gossiped block arrives:
```rust
while nonce < u64::MAX {
    if self.should_abort_mining(index).await? {
        return Err(MiningError::Aborted);
    }
    // ... continue mining ...
}
```

### Fix 3: Smarter Error Handling
When fork choice rejects, immediately advance instead of retrying:
```rust
Err(e) if e.to_string().contains("rejected by fork choice rules") => {
    log::info!("Block {} mined by another node, advancing", current_index);
    current_index += 1; // Don't retry
}
```

## Key Features

1. **Timing-Dependent** - Uses very low difficulty (1) to increase mining speed and likelihood of race condition
2. **Network Realistic** - Uses actual gossip protocol between two connected nodes
3. **Observable** - Provides clear log patterns to identify the race condition
4. **Comprehensive** - Tests both failure detection and recovery
5. **Statistical** - Measures frequency of the race condition
6. **Educational** - README explains the problem, current behavior, and potential solutions

## Code References

The test helps understand these components:
- `rust/modal-observer/src/chain_observer.rs` - Fork choice logic
- `rust/modal-node/src/actions/miner.rs` - Mining loop and error handling
- `rust/modal-node/src/gossip/miner/block.rs` - Gossip block handling
- `rust/modal-miner/src/fork_choice.rs` - Fork choice integration

## Next Steps

This test can be used to:
1. Verify the race condition exists (documentation)
2. Test proposed fixes
3. Ensure fixes don't break existing behavior
4. Measure performance impact of fixes
5. Serve as regression test after fix is implemented

