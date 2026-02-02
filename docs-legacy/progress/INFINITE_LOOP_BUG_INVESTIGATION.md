# Infinite Loop Bug Investigation

## Problem Description

On the live testnet2 node, after experiencing a race condition, the miner enters an **infinite loop** where it continuously tries to mine the same block, gets rejected, and never progresses.

## Testnet2 Evidence

### Current State
```
Chain Tip: Block 32875 (hash: 001068c373abd86b)
Total Blocks: 61197 (Canonical: 32877, Orphaned: 28320)
```

### Log Pattern (Repeating for >1 hour)
```
[04:09:07] Mining block 32877 with nominated peer
[04:09:53] ⚠️  Failed to mine block 32877 (Invalid block: Mined block was rejected by fork choice rules)
[04:09:53] ⛏️  Correcting mining index from 32877 to 32876 after error
[04:09:54] ⛏️  Mining block at index 32876...
[04:09:56] ✅ Successfully mined and gossipped block 32876  ← BUG: Claims success but didn't actually mine!
[04:09:57] ⛏️  Mining block at index 32877...
[loop repeats]
```

### Key Evidence Lines
```
[03:07:31] WARN  Block 32876 already exists in chain (height: 32876), skipping mining
[03:07:31] INFO  ✅ Successfully mined and gossipped block 32876
```

**The bug:** When the miner corrects back to block 32876, it finds the block already exists (from gossip), skips mining it, but then **claims success and increments to 32877**, creating an infinite loop.

## Root Cause

### Code Location: `rust/modal-node/src/actions/miner.rs`

**Function:** `mine_and_gossip_block` (lines 1377-1476)

```rust
// Check if we're trying to mine a block that already exists
if index < chain.height() + 1 && index < chain.blocks.len() as u64 {
    // Block already exists in the chain, skip it
    log::warn!("Block {} already exists in chain (height: {}), skipping mining", index, chain.height());
    return Ok(());  // ← Returns Ok, caller thinks it succeeded!
}
```

**Caller:** Main mining loop (lines 423-439)

```rust
match mine_and_gossip_block(...).await {
    Ok(()) => {
        log::info!("✅ Successfully mined and gossipped block {}", current_index);
        // Move to next block
        current_index += 1;  // ← Increments even when block was skipped!
    }
    Err(e) => {
        // Error handling with correction logic
    }
}
```

### The Problem

`mine_and_gossip_block` returns `Ok(())` in two cases:
1. **Actually mined a block** → Correct behavior
2. **Skipped because block exists** → **BUG:** Should not increment!

The caller can't distinguish between these two cases, so it always increments `current_index`.

## Why This Creates an Infinite Loop

1. Miner tries block N+1, gets rejected by fork choice (another miner won)
2. Corrects back to block N
3. Block N already exists (received via gossip)
4. Skips mining N, returns `Ok(())`
5. Caller logs "success" and increments to N+1
6. **Loop repeats forever at N+1**

## Solution Approaches

### Option 1: Return Different Result Types
```rust
enum MiningOutcome {
    Mined,
    Skipped,
}

fn mine_and_gossip_block(...) -> Result<MiningOutcome, Error>
```

### Option 2: Don't Skip, Let Fork Choice Handle It
Remove the "already exists" check entirely and let the fork choice logic reject it properly.

### Option 3: Check Chain Tip Before Mining
In the main loop, verify the current index is actually the next block to mine:
```rust
// Before mining, check current chain tip
let tip = get_chain_tip();
if current_index <= tip {
    current_index = tip + 1;
    continue; // Don't increment on this iteration
}
```

## Impact

- **Severity:** Critical - Node becomes stuck and can't mine new blocks
- **Frequency:** Occurs after any fork choice rejection in a multi-miner network
- **Duration:** Infinite (requires restart to recover)
- **Workaround:** Restart the node (it will resync and find the correct tip)

## Next Steps

1. ✅ Document the bug and evidence
2. ⬜ Create a test that reproduces the infinite loop
3. ⬜ Implement fix (recommend Option 1 or 3)
4. ⬜ Verify fix resolves testnet2 issue
5. ⬜ Add test to prevent regression

## Test Strategy

The `miner-gossip-race` test has been enhanced to detect this pattern:
- Look for "already exists in chain...skipping mining"
- Followed by "Successfully mined and gossipped block"
- This combination indicates the bug is present

