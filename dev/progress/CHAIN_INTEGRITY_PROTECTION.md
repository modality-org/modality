# Chain Integrity Protection System

## Overview

A comprehensive three-layered chain integrity system that prevents, detects, and automatically repairs blockchain data inconsistencies in the Modal network.

## Problem Statement

During blockchain operation, particularly with fork choice and chain reorganizations, blocks can become inconsistent where:
- Canonical blocks point to orphaned parents via `prev_hash`
- Subsequent blocks are built on broken chain segments
- These inconsistencies propagate until caught, causing failed syncs and network divergence

## Root Cause

The fork choice logic in `gossip/miner/block.rs` would orphan a losing block during competition but **failed to cascade orphan subsequent blocks** that were built on the orphaned block. This created "broken chains" where canonical block N+1 pointed to orphaned block N's hash.

## Solution: Three-Layer Protection

### Layer 1: Startup Validation and Repair

**Location**: `rust/modal-node/src/actions/chain_integrity.rs:run()`

**When**: Runs once on node startup before mining begins

**What it does**:
1. Loads all canonical blocks
2. Validates each block's `prev_hash` matches the previous block's `hash`
3. If a break is found, orphans all subsequent blocks from that point forward
4. Logs repair actions

**Code snippet**:
```rust
pub async fn run(datastore: &Arc<Mutex<NetworkDatastore>>) -> Result<()> {
    let mut ds = datastore.lock().await;
    let mut canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
    canonical_blocks.sort_by_key(|b| b.index);

    for block in canonical_blocks {
        if block.index == 0 { continue; }
        if block.previous_hash != last_valid_hash {
            // BREAK FOUND - Orphan this and all subsequent blocks
            ...
        }
    }
}
```

**Example output**:
```
[INFO] ✅ Chain integrity validated: 34513 blocks properly linked
OR
[ERROR] ❌ Chain break at index 34466: prev_hash 00af596409f8589c doesn't match block 34465 hash 001b93796cd2df45
[INFO] ✅ Chain repair complete: orphaned 1161 blocks
```

### Layer 2: Cascade Orphaning on Fork Choice

**Location**: `rust/modal-node/src/gossip/miner/block.rs` (fork choice logic)

**When**: Runs during gossip message processing when a heavier block replaces an existing canonical block

**What it does**:
1. Orphans the losing block (as before)
2. **NEW**: Finds all canonical blocks built on the orphaned block
3. Recursively orphans them by tracking orphaned hashes
4. Prevents broken chains from forming in the first place

**Code snippet**:
```rust
if should_replace {
    // Orphan old block
    let replaced_block_hash = existing.hash.clone();
    orphaned.save(&mut ds).await?;
    
    // CASCADE ORPHANING: Find and orphan all blocks built on replaced block
    let mut orphaned_hashes = HashSet::new();
    orphaned_hashes.insert(replaced_block_hash.clone());
    
    for block in all_canonical {
        if orphaned_hashes.contains(&block.previous_hash) {
            // This block points to an orphaned block - orphan it too
            cascade_orphaned.save(&mut ds).await?;
            orphaned_hashes.insert(block.hash.clone());
        }
    }
}
```

**Example output**:
```
[INFO] Fork choice: Replacing existing block 45917 (difficulty: 4, hash: 0020f9beb1944e28)
[INFO]    Cascade orphaning block 45918 at index 45918 (built on orphaned chain)
[INFO]    Cascade orphaning block 45919 at index 45919 (built on orphaned chain)
[WARN] ⚠️  Cascade orphaned 2 blocks built on replaced block 45917
```

### Layer 3: Rolling Integrity Check

**Location**: 
- `rust/modal-node/src/actions/chain_integrity.rs:check_recent_blocks()`
- Called from `rust/modal-node/src/actions/miner.rs` (after mining)
- Called from `rust/modal-node/src/gossip/miner/block.rs` (after accepting gossiped block)

**When**: Every 10 blocks (configurable)

**What it does**:
1. Validates the last 160 blocks (configurable window)
2. Checks `prev_hash` linkage for the window
3. If breaks are found and `repair=true`, automatically orphans broken segments
4. Lightweight and fast - only checks recent blocks

**Code snippet**:
```rust
pub async fn check_recent_blocks(
    datastore: &mut NetworkDatastore,
    window_size: usize,
    repair: bool,
) -> Result<bool> {
    // Get canonical blocks
    let canonical_blocks = MinerBlock::find_all_canonical(datastore).await?;
    
    // Check only the last N blocks
    let start_index = chain_length.saturating_sub(window_size);
    
    for index in (start_index + 1)..=max_index {
        let block = blocks_by_index.get(&index)?;
        let prev_block = blocks_by_index.get(&(index - 1))?;
        
        if block.previous_hash != prev_block.hash {
            if repair {
                // Orphan this and subsequent blocks
                ...
            }
            return Ok(false);
        }
    }
    
    Ok(true)
}
```

**Integration points**:
```rust
// In miner.rs after mining a block
if miner_block.index % 10 == 0 {
    chain_integrity::check_recent_blocks(&mut ds, 160, true).await?;
}

// In gossip/miner/block.rs after accepting a gossiped block
if miner_block.index % 10 == 0 {
    chain_integrity::check_recent_blocks(&mut ds, 160, true).await?;
}
```

**Example output**:
```
[DEBUG] ✓ Rolling integrity check passed (last 160 blocks)
OR
[ERROR] ❌ Rolling integrity check found and repaired broken blocks
[WARN] 🔧 Rolling repair: Orphaned 3 blocks from index 34466 onwards
```

## Benefits

1. **Prevention**: Cascade orphaning (Layer 2) prevents broken chains from forming
2. **Detection**: Rolling checks (Layer 3) catch issues early during operation
3. **Repair**: Startup validation (Layer 1) fixes legacy corruption
4. **Automatic**: All layers run automatically without manual intervention
5. **Minimal Overhead**: Rolling checks only scan recent blocks
6. **Network Convergence**: Clean chains enable successful auto-healing and sync

## Performance Characteristics

- **Startup Validation**: O(n) where n = total chain length, runs once
- **Cascade Orphaning**: O(m) where m = blocks after fork point, runs per fork
- **Rolling Checks**: O(w) where w = window size (160), runs every 10 blocks

## Testing

Tested with live testnet data containing actual integrity issues:
- testnet1: Repaired 6 blocks on startup
- testnet2: Repaired 7 blocks on startup  
- testnet3: Repaired 1161 blocks on startup
- All nodes validated successfully and began converging

## Files Modified

1. `rust/modal-node/src/actions/chain_integrity.rs` (NEW)
   - `run()` - Startup validation
   - `check_recent_blocks()` - Rolling validation

2. `rust/modal-node/src/actions/mod.rs`
   - Added `pub mod chain_integrity;`

3. `rust/modal-node/src/actions/miner.rs`
   - Added startup validation call
   - Added rolling integrity check after mining

4. `rust/modal-node/src/gossip/miner/block.rs`
   - Added cascade orphaning to fork choice
   - Added rolling integrity check after accepting blocks

## Configuration

- **Window Size**: 160 blocks (last ~4 hours at 1 min/block)
- **Check Frequency**: Every 10 blocks
- **Auto-Repair**: Enabled by default in all layers

## Future Improvements

- Make window size and frequency configurable via config.json
- Add metrics/telemetry for integrity issues caught
- Expose integrity status via node info command
- Consider checkpointing validated chain segments

