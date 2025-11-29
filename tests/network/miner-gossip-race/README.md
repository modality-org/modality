# Example: Miner Gossip Race Condition

This example demonstrates and tests the race condition that occurs when multiple miners compete for the same block.

## Overview

This test reproduces a real-world mining issue where:
1. Two miners start mining the same block index
2. One miner finds a valid block and gossips it to the network
3. The other miner finishes mining while the gossiped block is propagating
4. The second miner's block is rejected by fork choice rules ("first-seen" rule)
5. Mining gets stuck in a retry loop

## The Problem

When a miner completes mining a block, it tries to save it through fork choice validation. If another node's block at the same height was already received via gossip, the fork choice rule rejects the locally mined block with:

```
Failed to mine block X (Invalid block: Mined block was rejected by fork choice rules)
```

The current implementation uses a "first-seen" rule for single-block forks, which means:
- The first block received at a given height is kept
- All subsequent blocks at that height are rejected as orphans
- This is correct for gossiped blocks from other nodes
- But problematic when the local miner's own block is rejected

## Test Scenario

This test sets up:
1. **Miner 1** - Connected to network, mining with low difficulty
2. **Miner 2** - Connected to Miner 1, also mining with low difficulty
3. Both miners start from the same genesis block
4. Both attempt to mine block 1 simultaneously
5. One miner succeeds first, gossips to the other
6. The slower miner finishes mining but has their block rejected

## Expected Behavior (Current)

The logs will show:
```
[Miner 2] ⛏️  Mining block at index 1...
[Miner 1] ✅ Successfully mined and gossipped block 1
[Miner 2] Received miner block gossip (block 1 from Miner 1)
[Miner 2] ⚠️  Failed to mine block 1 (Invalid block: Mined block was rejected by fork choice rules)
[Miner 2] ⛏️  Correcting mining index from 1 to 1 after error
[Miner 2] Block 1 already exists in chain, skipping mining
[Miner 2] ✅ Successfully mined and gossipped block 1
[Miner 2] ⛏️  Mining block at index 2...
```

The miner recovers but wastes computational effort mining a block that gets rejected.

## Expected Impact with Mining Slowdown

**Without slowdown (mining_delay_ms = 0 or null):**
- Difficulty 1: mines in ~0.1-2 seconds
- Race condition window: ~100-2000ms
- Race probability: ~20-30%

**With 100ms slowdown:**
- Difficulty 1: mines in ~10-20 seconds  
- Race condition window: ~10000-20000ms
- Race probability: ~80-90%

**With 300ms slowdown (current test setting):**
- Difficulty 1: mines in ~30-60 seconds
- Race condition window: ~30000-60000ms
- Race probability: **~99%** 🎯

The longer mining time combined with simultaneous start from shared genesis makes collision almost guaranteed!

## Desired Behavior (Future Fix)

The miner should:
1. Check if a block already exists at the target height before starting expensive PoW
2. If a gossiped block arrives during mining, abandon the current mining attempt
3. Immediately advance to mining the next block

## Prerequisites

Build the `modal` CLI:
```bash
cd ../../../rust
cargo build --package modal
export PATH="$(pwd)/target/debug:$PATH"
```

## Running the Example

### Automated Test

Run the integration test:
```bash
./test.sh
```

This will:
1. Set up both miners
2. Start them sequentially (miner1, then miner2)
3. Monitor logs for the race condition
4. Verify both miners eventually sync up
5. Check for wasted mining effort

**Note:** The race condition is timing-dependent and may not occur every run. See "Forcing the Race Condition" below.

### Force the Race Condition (Recommended)

To make the race condition occur reliably (~80-90% probability):

```bash
./04-force-race-condition.sh
```

This script:
1. Pre-mines a shared genesis block
2. Gives both miners the same starting point
3. Starts both miners simultaneously
4. Both try to mine block 1 at the same time
5. Race condition almost guaranteed!

See `FORCING_RACE_CONDITION.md` for more techniques.

### Manual Test

1. Start Miner 1:
   ```bash
   ./01-run-miner1.sh
   ```

2. In a new terminal, start Miner 2:
   ```bash
   ./02-run-miner2.sh
   ```

3. Watch the logs for the race condition - look for:
   - "Failed to mine block X (Invalid block: Mined block was rejected by fork choice rules)"
   - "Correcting mining index from X to Y after error"

### Automated Test

Run the integration test:
```bash
./test.sh
```

This will:
1. Set up both miners
2. Start them simultaneously
3. Monitor logs for the race condition
4. Verify both miners eventually sync up
5. Check for wasted mining effort

## Key Log Patterns

**Race condition detected:**
```
⚠️  Failed to mine block X (Invalid block: Mined block was rejected by fork choice rules)
```

**Miner recovery:**
```
⛏️  Correcting mining index from X to Y after error
```

**Wasted effort:**
```
Block X already exists in chain, skipping mining
```

## Related Code

- **Fork choice logic**: `rust/modal-observer/src/chain_observer.rs` - `process_gossiped_block()`
- **First-seen rule**: `rust/modal-observer/src/chain_observer.rs` - `should_accept_single_block()`
- **Mining loop**: `rust/modal-node/src/actions/miner.rs` - Mining loop error handling
- **Gossip handler**: `rust/modal-node/src/gossip/miner/block.rs` - Incoming block processing

## Visual Diagram

```
Time →
─────────────────────────────────────────────────────────────────

Miner 1:  [Start Mining Block 1] ────────> [Found Nonce!] ──> [Save Block] ──> [Gossip Block] ──> [Mine Block 2]
                                                                     ✓                   │
                                                                                         │
                                                                                         │ Network gossip
                                                                                         │
                                                                                         ▼
Miner 2:  [Start Mining Block 1] ─────────────────────> [Found Nonce!] ──────> [Receive Gossip] ──> [Try Save] ──X REJECTED!
                                                                                      │                    │
                                                                                      │                    │
                                                                                      ▼                    ▼
                                                                           [Block 1 already exists]  [Fork choice: first-seen]
                                                                                                           │
                                                                                                           ▼
                                                                                                  [Retry/Correct/Skip]
                                                                                                           │
                                                                                                           ▼
                                                                                                    [Mine Block 2]

Problem: Miner 2 wasted computational effort mining Block 1, only to have it rejected.
```

## Potential Fixes

### Fix 1: Pre-Mining Check
Before starting PoW, check if block exists:
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
Allow mining to be interrupted when a block arrives via gossip:
```rust
// In mine_block_with_persistence
while nonce < u64::MAX {
    if self.should_abort_mining(index).await? {
        return Err(MiningError::Aborted);
    }
    // ... continue mining ...
}
```

### Fix 3: Smarter Error Handling
When fork choice rejects a mined block, immediately advance:
```rust
Err(e) if e.to_string().contains("rejected by fork choice rules") => {
    log::info!("Block {} mined by another node, advancing", current_index);
    current_index += 1; // Don't retry
}
```

## Clean Up

Remove test nodes and storage:
```bash
./00-clean.sh
```

## Troubleshooting

**Both miners mining indefinitely without detecting race:**
- The race condition is timing-dependent
- Lower the initial difficulty to increase the likelihood
- Check network connectivity between miners

**No fork choice rejection:**
- Verify both miners are connected (check peer count in logs)
- Check that gossip is working (look for "Received miner block gossip")
- Increase mining speed by lowering difficulty further

## Testing Tips

- Use `RUST_LOG=debug` for detailed logging
- Monitor both miner logs simultaneously
- The race condition is more likely with:
  - Very low difficulty (fast mining)
  - Good network connectivity
  - Multiple miners on the same block

