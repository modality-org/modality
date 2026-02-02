# Mining Race Condition and Infinite Loop Bug - Summary

## Investigation Complete ✅

Successfully investigated the testnet2 mining failure and identified **two related bugs**:

### 1. Race Condition (Original Issue)
**Symptom:** Mining fails with "Invalid block: Mined block was rejected by fork choice rules"

**Cause:** When multiple miners simultaneously find valid blocks at the same height, the fork choice "first-seen" rule causes one miner's block to be rejected.

**Severity:** Medium - Normal behavior in multi-miner networks, should be recoverable

### 2. Infinite Loop Bug (Critical Discovery)  
**Symptom:** After a race condition, node gets stuck indefinitely trying to mine the same block

**Cause:** Bug in mining loop that treats "block skipped" the same as "block mined"

## Testnet2 Evidence

```
Node: testnet2 (ssh testnet2)
Chain Tip: Block 32875
Stuck since: 2025-11-20 03:07 UTC (>1 hour)
Log Pattern: Infinite loop at blocks 32876/32877
```

### Infinite Loop Sequence
```
1. [03:07:32] Mining block 32877...
2. [03:07:46] ⚠️  Failed to mine block 32877 (rejected by fork choice rules)
3. [03:07:47] ⛏️  Correcting mining index from 32877 to 32876
4. [03:07:50] WARN Block 32876 already exists in chain, skipping mining
5. [03:07:50] INFO ✅ Successfully mined and gossipped block 32876  ← BUG!
6. [03:07:51] Mining block 32877...
7. GOTO 2 (infinite loop)
```

## Root Cause Analysis

### Code Location
- **File:** `rust/modal-node/src/actions/miner.rs`
- **Functions:** `mine_and_gossip_block` + main mining loop

### The Bug

**Function:** `mine_and_gossip_block` (lines 1378-1382)
```rust
// Check if we're trying to mine a block that already exists
if index < chain.height() + 1 && index < chain.blocks.len() as u64 {
    log::warn!("Block {} already exists in chain (height: {}), skipping mining", index, chain.height());
    return Ok(());  // ← Problem: Returns Ok() when skipping!
}
```

**Caller:** Main mining loop (lines 437-440)
```rust
match mine_and_gossip_block(...).await {
    Ok(()) => {
        log::info!("✅ Successfully mined and gossipped block {}", current_index);
        current_index += 1;  // ← Always increments, even when block was skipped!
    }
}
```

### Why It's a Problem

The function returns `Ok(())` in two completely different scenarios:
1. ✅ **Block was actually mined** → Should increment index
2. ❌ **Block was skipped (already exists)** → Should NOT increment index

The caller cannot distinguish between these cases, so it always:
- Logs "Successfully mined"
- Increments the index

This creates an infinite loop when a block exists via gossip.

## Test Suite Created

Created `examples/network/miner-gossip-race/` test suite:

### Test Strategy
1. **Force race condition** - Shared genesis + simultaneous mining start + 300ms mining delay
2. **Detect infinite loop** - Look for "already exists...skipping" + "Successfully mined" pattern
3. **Verify recovery** - Check if miners eventually synchronize

### Current Status
- ✅ Test infrastructure complete
- ✅ Can run cleanly without port conflicts (removed status_port)
- ⚠️  Race condition is intermittent (timing-dependent even with 300ms delay)
- ⚠️  Haven't caught the infinite loop in automated test yet

### Test Files
- `test.sh` - Automated integration test (17 tests)
- `01-run-miner1.sh` - Manual testing helper
- `02-run-miner2.sh` - Manual testing helper  
- `00-clean.sh` - Cleanup script
- `README.md` - Documentation

## Solution Options

### Option 1: Return Enum (Recommended)
```rust
enum MiningOutcome {
    Mined,
    Skipped,
}

fn mine_and_gossip_block(...) -> Result<MiningOutcome, Error> {
    if block_exists {
        return Ok(MiningOutcome::Skipped);
    }
    // ... mine block ...
    Ok(MiningOutcome::Mined)
}

// Caller:
match mine_and_gossip_block(...).await {
    Ok(MiningOutcome::Mined) => {
        log::info!("✅ Successfully mined block {}", current_index);
        current_index += 1;
    }
    Ok(MiningOutcome::Skipped) => {
        log::info!("⏭️  Block {} already exists, skipping", current_index);
        current_index += 1; // Still increment, just don't claim we mined it
    }
    Err(e) => {
        // Error handling
    }
}
```

### Option 2: Remove Skip Logic
Remove the "already exists" check entirely and let fork choice handle it:
```rust
// Delete lines 1378-1383
// Let the mining attempt happen, fork choice will reject if needed
```

**Pros:** Simpler
**Cons:** Wastes CPU mining a block that will be rejected

### Option 3: Verify Chain Tip Before Mining
```rust
// Before mining, always check current chain state
let tip = get_chain_tip_from_datastore();
if current_index <= tip {
    log::info!("Chain advanced to {}, updating mining index", tip);
    current_index = tip + 1;
    continue; // Don't increment on this iteration
}
```

## Impact Assessment

**Severity:** 🔴 Critical
- Node becomes completely stuck after first race condition
- Requires manual restart to recover
- Wastes mining power in infinite loop
- Affects all multi-miner networks (testnet, mainnet)

**Frequency:** High in multi-miner environments
- Any time two miners find blocks simultaneously
- More likely with lower difficulty (testnet)
- Inevitable in production with multiple miners

**Workaround:** Restart the node
- Node will resync and find correct chain tip
- But will get stuck again on next race condition

## Next Steps

### Immediate Actions
1. ✅ Document bug and evidence
2. ✅ Create test suite
3. ⬜ **Implement fix (Option 1 recommended)**
4. ⬜ Test fix locally
5. ⬜ Verify fix resolves testnet2 issue

### Testing Strategy
1. Run local test multiple times to catch race condition
2. Manually test with `01-run-miner1.sh` + `02-run-miner2.sh`
3. Deploy to testnet2 and monitor
4. Verify node can recover from race conditions

### Long-term
1. Add metrics for race condition frequency
2. Consider optimization: faster gossip propagation
3. Monitor orphan block rates in production

## Files Modified

### Test Suite
- `examples/network/miner-gossip-race/test.sh` - Added infinite loop detection (Test 9)
- `examples/network/miner-gossip-race/01-run-miner1.sh` - Removed status_port, added mining_delay_ms
- `examples/network/miner-gossip-race/02-run-miner2.sh` - Removed status_port, added mining_delay_ms
- `examples/network/miner-gossip-race/00-clean.sh` - Updated cleanup

### Documentation
- `docs/progress/INFINITE_LOOP_BUG_INVESTIGATION.md` - Detailed analysis
- `docs/progress/MINING_SLOWDOWN_IMPLEMENTATION.md` - mining_delay_ms parameter
- `docs/progress/99_PERCENT_RACE_CONDITION.md` - 300ms delay rationale

### Code (for mining slowdown only, bug fix still needed)
- `rust/modal-common/src/hash_tax.rs` - Added mining_delay_ms
- `rust/modal-miner/src/miner.rs` - Propagated mining_delay_ms
- `rust/modal-miner/src/chain.rs` - Propagated mining_delay_ms
- `rust/modal-node/src/config.rs` - Added mining_delay_ms config
- `rust/modal-node/src/node.rs` - Extracted mining_delay_ms from config
- `rust/modal-node/src/actions/miner.rs` - Passed mining_delay_ms to mining functions

## Recommendations

**Priority 1:** Fix the infinite loop bug (Option 1)
- This is blocking testnet operations
- Simple fix with clear solution

**Priority 2:** Improve test reliability
- Currently race condition is intermittent
- May need even longer delays or different approach

**Priority 3:** Add monitoring
- Track race condition frequency
- Alert when node gets stuck
- Automatic restart capability?

---

**Status:** Investigation complete, ready for bug fix implementation
**Blocker:** Testnet2 node is stuck and needs manual intervention
**ETA:** Bug fix can be implemented in <1 hour, testing will take longer

