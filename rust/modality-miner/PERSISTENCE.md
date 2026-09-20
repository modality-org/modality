# Blockchain Persistence with NetworkDatastore

This document describes the persistence feature for the modal-miner blockchain.

## Overview

The `persistence` feature flag enables saving and loading blockchain data to/from `NetworkDatastore`, providing:

- **Automatic block persistence** during mining
- **Chain recovery** after restarts
- **Orphan block tracking** for chain reorganizations
- **Epoch-based queries** for efficient data retrieval

## Enabling Persistence

Add the feature to your `Cargo.toml`:

```toml
[dependencies]
modal-miner = { path = "../modal-miner", features = ["persistence"] }
modal-datastore = { path = "../modal-datastore" }
tokio = { version = "1", features = ["rt", "macros"] }
```

## Architecture

### Data Flow

```
┌──────────────┐
│   Blockchain │
│              │
│  mine_block_ │
│  with_       │
│  persistence │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Persistence  │
│ Trait        │
│              │
│ save_block() │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ MinerBlock   │
│ Model        │
│              │
│ (Datastore)  │
└──────────────┘
```

### Components

1. **BlockchainPersistence Trait**
   - Defines persistence operations
   - Implemented for `NetworkDatastore`
   - Async operations using tokio

2. **MinerBlock Model**
   - Represents persisted blocks
   - Includes canonical and orphaned states
   - Stored in datastore at `/miner_blocks/hash/${hash}`

3. **Blockchain Methods**
   - `mine_block_with_persistence()`: Mine and save automatically
   - `add_block_with_persistence()`: Add pre-mined blocks
   - `load_or_create()`: Load existing chain or create genesis

## API Reference

### BlockchainPersistence Trait

```rust
#[async_trait]
pub trait BlockchainPersistence {
    /// Save a block to the datastore
    async fn save_block(&self, block: &Block, epoch: u64) -> Result<(), MiningError>;
    
    /// Load all canonical blocks from the datastore
    async fn load_canonical_blocks(&self) -> Result<Vec<Block>, MiningError>;
    
    /// Load blocks for a specific epoch
    async fn load_epoch_blocks(&self, epoch: u64) -> Result<Vec<Block>, MiningError>;
    
    /// Mark a block as orphaned
    async fn mark_block_orphaned(
        &self,
        block_hash: &str,
        reason: String,
        competing_hash: Option<String>,
    ) -> Result<(), MiningError>;
}
```

### Blockchain Persistence Methods

```rust
impl Blockchain {
    /// Create blockchain with datastore
    pub fn new_with_datastore(
        config: ChainConfig,
        genesis_peer_id: String,
        datastore: Arc<NetworkDatastore>,
    ) -> Self;
    
    /// Load existing chain or create new one
    pub async fn load_or_create(
        config: ChainConfig,
        genesis_peer_id: String,
        datastore: Arc<NetworkDatastore>,
    ) -> Result<Self, MiningError>;
    
    /// Mine and persist block
    pub async fn mine_block_with_persistence(
        &mut self,
        nominated_peer_id: String,
        miner_number: u64,
    ) -> Result<Block, MiningError>;
    
    /// Add and persist pre-mined block
    pub async fn add_block_with_persistence(
        &mut self,
        block: Block,
    ) -> Result<(), MiningError>;
}
```

## Usage Examples

### Basic Usage

```rust
use modal_miner::{Blockchain, ChainConfig, BlockchainPersistence};
use modal_datastore::NetworkDatastore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create datastore
    let datastore = Arc::new(NetworkDatastore::create_in_directory("./chain_data")?);
    
    // Create or load blockchain
    let mut chain = Blockchain::load_or_create(
        ChainConfig::default(),
        "QmGenesis...".to_string(),
        datastore.clone(),
    ).await?;
    
    // Mine blocks with automatic persistence
    let block = chain.mine_block_with_persistence(
        "QmMiner1...".to_string(),
        42,
    ).await?;
    
    println!("Block {} persisted!", block.header.index);
    Ok(())
}
```

### Load and Continue Mining

```rust
// Load existing chain
let mut chain = Blockchain::load_or_create(
    config,
    genesis_peer_id,
    datastore.clone(),
).await?;

println!("Loaded chain at height: {}", chain.height());

// Continue mining
for i in 0..10 {
    chain.mine_block_with_persistence(
        format!("QmMiner{}", i),
        1000 + i,
    ).await?;
}
```

### Query Persisted Blocks

```rust
// Load all canonical blocks
let blocks = datastore.load_canonical_blocks().await?;
println!("Total blocks: {}", blocks.len());

// Load specific epoch
let epoch_0 = datastore.load_epoch_blocks(0).await?;
println!("Epoch 0 has {} blocks", epoch_0.len());

// Each epoch has up to 40 blocks
assert!(epoch_0.len() <= 40);
```

### Orphan Handling

```rust
// Mark a block as orphaned during chain reorganization
datastore.mark_block_orphaned(
    "block_hash_123...",
    "Chain reorganization - longer chain found".to_string(),
    Some("winning_block_hash...".to_string()),
).await?;
```

## Data Model

### MinerBlock Structure

```rust
pub struct MinerBlock {
    // Block identification
    pub hash: String,
    pub index: u64,
    pub epoch: u64,
    
    // Block header
    pub timestamp: i64, // Unix timestamp
    pub previous_hash: String,
    pub data_hash: String,
    pub nonce: String, // Stored as string (u128 in Block)
    pub difficulty: String, // Stored as string (u128 in Block)
    
    // Block data
    pub nominated_peer_id: String,
    pub miner_number: u64,
    
    // Status
    pub is_canonical: bool,
    pub is_orphaned: bool,
    
    // Metadata
    pub seen_at: Option<i64>,
    pub orphaned_at: Option<i64>,
    pub orphan_reason: Option<String>,
    pub height_at_time: Option<u64>,
    pub competing_hash: Option<String>,
}
```

### Storage Layout

Blocks are stored in RocksDB:
- Key: `/miner_blocks/hash/${block_hash}`
- Value: JSON-serialized `MinerBlock`

### Canonical vs Orphaned

- **Canonical blocks** (`is_canonical = true`): Part of the main chain
- **Orphaned blocks** (`is_orphaned = true`): Valid but not in main chain (due to reorg)

## Performance Considerations

### Memory Usage

- **In-Memory**: Full blockchain kept in `Vec<Block>`
- **On-Disk**: All blocks persisted to datastore
- **Trade-off**: Fast access vs. persistent storage

### Disk I/O

- **Write**: O(1) per block (single datastore put)
- **Read All**: O(n) scan through all blocks
- **Read Epoch**: O(n) filtered scan (can be optimized with indices)

### Async Operations

All persistence operations are async to avoid blocking:
```rust
// Non-blocking mining
let block = chain.mine_block_with_persistence(peer_id, number).await?;

// Non-blocking load
let blocks = datastore.load_canonical_blocks().await?;
```

## Error Handling

```rust
use modal_miner::MiningError;

match chain.mine_block_with_persistence(peer_id, number).await {
    Ok(block) => println!("Mined: {}", block.header.hash),
    Err(MiningError::PersistenceError(e)) => {
        eprintln!("Failed to persist block: {}", e);
    }
    Err(e) => eprintln!("Mining error: {}", e),
}
```

## Testing

Run persistence tests:

```bash
# All tests with persistence
cargo test --features persistence

# Only persistence module tests
cargo test --features persistence persistence

# Run persistence demo
cargo run --example persistence_demo --features persistence
```

## Migration Guide

### From Non-Persistent to Persistent

**Before:**
```rust
let mut chain = Blockchain::new(config, genesis_peer_id);
let block = chain.mine_block(peer_id, number)?;
```

**After:**
```rust
let datastore = Arc::new(NetworkDatastore::create_in_directory("./data")?);
let mut chain = Blockchain::load_or_create(config, genesis_peer_id, datastore).await?;
let block = chain.mine_block_with_persistence(peer_id, number).await?;
```

### Backward Compatibility

The persistence feature is opt-in:
- Without the feature, blockchain works in-memory only
- With the feature, both in-memory and persistent modes available
- Existing code using `mine_block()` continues to work

## Best Practices

1. **Use Shared Datastore**: Wrap in `Arc` for multiple references
2. **Handle Errors**: Always check persistence errors
3. **Periodic Checkpoints**: Consider saving state periodically
4. **Chain Validation**: Validate chain after loading
5. **Orphan Management**: Track orphaned blocks for analytics

## Future Enhancements

- [ ] Indexed queries (by epoch, peer ID, etc.)
- [ ] Compaction for old blocks
- [ ] Incremental loading (don't load entire chain)
- [ ] Snapshot/restore functionality
- [ ] Metrics and monitoring

## Examples

See `examples/persistence_demo.rs` for a complete working example demonstrating:
- Creating a blockchain with persistence
- Mining blocks with automatic saving
- Loading the blockchain after restart
- Querying persisted blocks
- Continuing mining on loaded chain

Run it with:
```bash
cargo run --example persistence_demo --features persistence
```

## Related Documentation

- [README.md](./README.md) - Main package documentation
- [NetworkDatastore Documentation](../modal-datastore/README.md)
- [MinerBlock Model](../modal-datastore/docs/MINER_BLOCK.md)

