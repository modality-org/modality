# MinerBlock Model

The `MinerBlock` model stores proof-of-work mining blocks in the datastore, including both canonical (main chain) blocks and orphaned blocks.

## Overview

`MinerBlock` is designed to persist mining block data from the `modality-network-mining` package. It tracks:
- **Canonical blocks**: Blocks that are part of the main blockchain
- **Orphaned blocks**: Blocks that were mined but didn't make it into the main chain (due to chain reorganizations, competing blocks, etc.)

## Fields

### Block Identification
- `hash` (String): Unique block hash
- `index` (u64): Block index in the chain
- `epoch` (u64): Which epoch (40-block period) this block belongs to

### Block Header Data
- `timestamp` (i64): Unix timestamp when block was created
- `previous_hash` (String): Hash of the previous block
- `data_hash` (String): Hash of the block data
- `nonce` (String): Proof-of-work nonce (stored as string since u128)
- `difficulty` (String): Mining difficulty (stored as string since u128)

### Block Data
- `nominated_peer_id` (String): Peer ID nominated by the miner
- `miner_number` (u64): Arbitrary number chosen by the miner

### Chain Status
- `is_orphaned` (bool): Whether this block is orphaned
- `is_canonical` (bool): Whether this block is in the main chain

### Metadata (Optional)
- `seen_at` (Option\<i64\>): When the block was first seen
- `orphaned_at` (Option\<i64\>): When the block was marked as orphaned
- `orphan_reason` (Option\<String\>): Why it was orphaned
- `height_at_time` (Option\<u64\>): Chain height when block was seen
- `competing_hash` (Option\<String\>): Hash of the winning block (if orphaned)

## Creating Blocks

### Canonical Block

```rust
use modal_datastore::models::MinerBlock;

let block = MinerBlock::new_canonical(
    "block_hash_123".to_string(),
    1,                          // index
    0,                          // epoch
    1234567890,                 // timestamp
    "prev_hash".to_string(),
    "data_hash".to_string(),
    12345,                      // nonce
    1000,                       // difficulty
    "peer_id_abc123".to_string(),
    42,                         // miner_number
);
```

### Orphaned Block

```rust
let orphaned = MinerBlock::new_orphaned(
    "orphan_hash".to_string(),
    1,                          // index
    0,                          // epoch
    1234567890,                 // timestamp
    "prev_hash".to_string(),
    "data_hash".to_string(),
    99999,                      // nonce
    1000,                       // difficulty
    "peer_id_xyz789".to_string(),
    99,                         // miner_number
    "Chain reorg".to_string(),  // orphan reason
    Some("winning_hash".to_string()), // competing hash
);
```

## Saving and Loading

```rust
use modal_datastore::{NetworkDatastore, Model};
use std::collections::HashMap;

// Save a block
block.save(&datastore).await?;

// Load by hash
let mut keys = HashMap::new();
keys.insert("hash".to_string(), "block_hash_123".to_string());
let loaded = MinerBlock::find_one(&datastore, keys).await?;
```

## Querying

### Find Canonical Blocks in an Epoch

```rust
// Get all canonical blocks in epoch 0
let epoch_blocks = MinerBlock::find_canonical_by_epoch(&datastore, 0).await?;
for block in epoch_blocks {
    println!("Block {} at index {}", block.hash, block.index);
}
```

### Find All Orphaned Blocks

```rust
let orphaned = MinerBlock::find_all_orphaned(&datastore).await?;
for block in orphaned {
    println!("Orphaned: {} - Reason: {:?}", 
        block.hash, block.orphan_reason);
}
```

### Find Blocks by Index

```rust
// Get all blocks (both canonical and orphaned) at a specific index
let blocks = MinerBlock::find_by_index(&datastore, 42).await?;

// Get only the canonical block at an index
if let Some(canonical) = MinerBlock::find_canonical_by_index(&datastore, 42).await? {
    println!("Canonical block at 42: {}", canonical.hash);
}
```

## Marking Blocks as Orphaned

```rust
// Load a canonical block
let mut block = /* ... load block ... */;

// Mark it as orphaned
block.mark_as_orphaned(
    "Replaced by longer chain".to_string(),
    Some("new_winning_hash".to_string())
);

// Save the updated block
block.save(&datastore).await?;
```

## Storage Structure

Blocks are stored with the key pattern:
```
/miner_blocks/hash/${hash}
```

This allows efficient lookup by hash and prefix-based iteration over all blocks.

## Integration with modality-network-mining

To convert from a `modality-network-mining` block to a `MinerBlock`:

```rust
use modality_network_mining::Block as MiningBlock;
use modal_datastore::models::MinerBlock;

fn convert_mining_block(mining_block: &MiningBlock, epoch: u64) -> MinerBlock {
    MinerBlock::new_canonical(
        mining_block.header.hash.clone(),
        mining_block.header.index,
        epoch,
        mining_block.header.timestamp.timestamp(),
        mining_block.header.previous_hash.clone(),
        mining_block.header.data_hash.clone(),
        mining_block.header.nonce,
        mining_block.header.difficulty,
        mining_block.data.nominated_peer_id.clone(),
        mining_block.data.miner_number,
    )
}
```

## Use Cases

### Chain Reorganization Tracking

When a chain reorg occurs:
1. Mark affected blocks as orphaned with `mark_as_orphaned()`
2. Save new canonical blocks
3. Query orphaned blocks for analysis

```rust
// Find blocks that need to be orphaned
let blocks_to_orphan = MinerBlock::find_by_index(&datastore, reorg_index).await?;

for mut block in blocks_to_orphan {
    if block.is_canonical {
        block.mark_as_orphaned(
            "Chain reorganization".to_string(),
            Some(new_canonical_hash.clone())
        );
        block.save(&datastore).await?;
    }
}
```

### Mining Statistics

```rust
// Get all blocks in an epoch for analysis
let epoch_blocks = MinerBlock::find_canonical_by_epoch(&datastore, 0).await?;

// Calculate epoch statistics
let total_blocks = epoch_blocks.len();
let avg_difficulty: f64 = epoch_blocks.iter()
    .map(|b| b.get_difficulty_u128().unwrap() as f64)
    .sum::<f64>() / total_blocks as f64;

// Count unique nominated peer IDs
let unique_peers: HashSet<_> = epoch_blocks.iter()
    .map(|b| &b.nominated_peer_id)
    .collect();
```

### Orphan Rate Analysis

```rust
// Analyze orphan rates over time
let all_orphaned = MinerBlock::find_all_orphaned(&datastore).await?;

// Group by epoch
let mut orphans_by_epoch: HashMap<u64, Vec<_>> = HashMap::new();
for block in all_orphaned {
    orphans_by_epoch.entry(block.epoch)
        .or_insert_with(Vec::new)
        .push(block);
}

// Calculate orphan rate per epoch
for (epoch, orphans) in orphans_by_epoch {
    let canonical = MinerBlock::find_canonical_by_epoch(&datastore, epoch).await?;
    let orphan_rate = orphans.len() as f64 / (orphans.len() + canonical.len()) as f64;
    println!("Epoch {}: orphan rate = {:.2}%", epoch, orphan_rate * 100.0);
}
```

## Example

See `examples/miner_block_usage.rs` for a complete working example:

```bash
cargo run --package modal-datastore --example miner_block_usage
```

