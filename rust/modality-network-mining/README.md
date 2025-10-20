# Modality Network Mining

A proof-of-work blockchain implementation for the Modality network with epoch-based difficulty adjustment and deterministic nomination shuffling.

## Features

- **Proof-of-Work Mining**: SHA-256 based block mining with configurable difficulty
- **Epoch-Based Difficulty Adjustment**: Automatic difficulty adjustment every 40 blocks (1 epoch)
- **Nominated Peer IDs**: Each block nominates a network peer ID for downstream use (e.g., validator selection)
- **Deterministic Shuffling**: Fisher-Yates shuffle based on XOR of epoch nonces for fair nomination ordering
- **Persistence Support** (optional): Save and load blockchain data using `NetworkDatastore`
- **Comprehensive Validation**: Block and chain validation with detailed error reporting

## Architecture

### Core Components

- **Block**: Contains header (hash, nonce, difficulty) and data (nominated peer ID, miner number)
- **Blockchain**: Manages the chain of blocks, validation, and mining
- **Miner**: Handles proof-of-work computation
- **EpochManager**: Manages difficulty adjustment and nomination shuffling per epoch
- **Persistence** (optional feature): Save/load blocks to/from NetworkDatastore

### Block Structure

Each block consists of:

#### Block Header
- `index`: Block number in the chain
- `timestamp`: When the block was created
- `previous_hash`: Hash of the previous block
- `data_hash`: Hash of the block data
- `nonce`: Proof-of-work nonce
- `difficulty`: Target difficulty for mining
- `hash`: SHA-256 hash of the block header

#### Block Data
- **Nominated Peer ID**: Peer ID nominated by the miner (to be used downstream for various purposes)
- **Miner Number**: An arbitrary u64 number selected by the miner

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
modality-network-mining = { path = "../modality-network-mining" }

# With persistence support
modality-network-mining = { path = "../modality-network-mining", features = ["persistence"] }
```

## Usage

### Basic Mining (Without Persistence)

```rust
use modality_network_mining::{Blockchain, ChainConfig};

// Create a new blockchain
let config = ChainConfig {
    initial_difficulty: 1000,
    target_block_time_secs: 60, // 1 minute per block
};

let genesis_peer_id = "QmGenesisAbc123...";
let mut chain = Blockchain::new(config, genesis_peer_id.to_string());

// Mine a block
let nominated_peer_id = "QmMiner1Def456...";
let miner_number = 12345;

let block = chain.mine_block(nominated_peer_id.to_string(), miner_number)?;
println!("Mined block {}: {}", block.header.index, block.header.hash);
```

### With Persistence

```rust
use modality_network_mining::{Blockchain, ChainConfig, BlockchainPersistence};
use modality_network_datastore::NetworkDatastore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create or open datastore
    let datastore = Arc::new(NetworkDatastore::create_in_directory("./blockchain_data")?);
    
    let config = ChainConfig::default();
    let genesis_peer_id = "QmGenesis...";
    
    // Load existing chain or create new
    let mut chain = Blockchain::load_or_create(
        config,
        genesis_peer_id.to_string(),
        datastore.clone(),
    ).await?;
    
    // Mine blocks with automatic persistence
    let block = chain.mine_block_with_persistence(
        "QmMiner1...".to_string(),
        42,
    ).await?;
    
    println!("Block persisted: {}", block.header.hash);
    
    // Query persisted blocks
    let canonical = datastore.load_canonical_blocks().await?;
    println!("Total canonical blocks: {}", canonical.len());
    
    Ok(())
}
```

### Epoch-Based Nomination Shuffling

```rust
// Mine a complete epoch (40 blocks)
for i in 0..40 {
    let peer_id = format!("QmMiner{}", i);
    chain.mine_block(peer_id, 1000 + i)?;
}

// Get shuffled nominations for epoch 0
if let Some(shuffled) = chain.get_epoch_shuffled_nominations(0) {
    // Returns Vec<(block_index, nominated_peer_id)> in shuffled order
    for (idx, peer_id) in shuffled.iter().take(10) {
        println!("Position {}: Block {} nominated {}", idx, idx, peer_id);
    }
}

// Or just get the shuffled peer IDs
if let Some(peer_ids) = chain.get_epoch_shuffled_peer_ids(0) {
    println!("Shuffled peer IDs: {:?}", peer_ids);
}
```

### Query the Chain

```rust
// Get blocks by nominated peer ID
let peer_id = "QmMiner1...";
let blocks = chain.get_blocks_by_nominated_peer(peer_id);
let count = chain.count_blocks_by_nominated_peer(peer_id);

// Get blocks by epoch
let epoch_0_blocks = chain.get_epoch_blocks(0);

// Get specific blocks
let latest = chain.latest_block();
let genesis = chain.get_block_by_index(0);
let by_hash = chain.get_block_by_hash("block_hash_here");

// Chain info
let height = chain.height();
let current_epoch = chain.current_epoch();
let next_difficulty = chain.get_next_difficulty();
```

### Validation

```rust
// Validate entire chain
match chain.validate_chain() {
    Ok(_) => println!("Chain is valid!"),
    Err(e) => eprintln!("Chain validation failed: {}", e),
}

// Direct mining with validation
use modality_network_mining::{Block, BlockData, Miner};

let miner = Miner::new_default();
let data = BlockData::new("QmMiner...".to_string(), 999);
let block = Block::new(1, "prev_hash".to_string(), data, 1000);

// Mine and verify
let mined_block = miner.mine_block(block)?;
assert!(miner.verify_block(&mined_block)?);
```

## Epoch Management

An **epoch** is a period of 40 blocks. At the end of each epoch:

1. **Difficulty Adjustment**: Based on actual vs. target block time
2. **Nomination Shuffling**: Deterministic shuffle of nominated peer IDs using XOR of all nonces as seed

### Difficulty Adjustment

- If blocks are mined **faster** than target → difficulty **increases** (up to 8x per epoch)
- If blocks are mined **slower** than target → difficulty **decreases** (minimum 0.5x/halve per epoch)
- Adjustment scales based on how far off the actual time is from expected time

### Nomination Shuffling

The shuffling process:
1. XOR all nonces from blocks in the epoch to create a seed
2. Use seed for deterministic Fisher-Yates shuffle
3. Output is a shuffled list of (block_index, peer_id) pairs

This can be used for:
- Validator selection
- Consensus participation
- Reward distribution
- Governance voting order

## Persistence

The optional `persistence` feature integrates with `modality-network-datastore`:

```bash
# Build with persistence
cargo build --features persistence

# Run tests with persistence
cargo test --features persistence

# Run persistence example
cargo run --example persistence_demo --features persistence
```

### Persistence API

```rust
// Trait: BlockchainPersistence (implemented for NetworkDatastore)
trait BlockchainPersistence {
    async fn save_block(&self, block: &Block, epoch: u64) -> Result<()>;
    async fn load_canonical_blocks(&self) -> Result<Vec<Block>>;
    async fn load_epoch_blocks(&self, epoch: u64) -> Result<Vec<Block>>;
    async fn mark_block_orphaned(&self, block_hash: &str, reason: String, ...) -> Result<()>;
}

// Blockchain methods with persistence
chain.mine_block_with_persistence(peer_id, number).await?;
chain.add_block_with_persistence(block).await?;
Blockchain::load_or_create(config, genesis_peer_id, datastore).await?;
```

## Examples

Run the examples:

```bash
# Basic blockchain usage
cargo run --example basic_usage

# Epoch shuffling demonstration
cargo run --example epoch_shuffle_demo

# Persistence demo (requires persistence feature)
cargo run --example persistence_demo --features persistence
```

## Configuration

```rust
pub struct ChainConfig {
    /// Initial difficulty for mining
    pub initial_difficulty: u128,
    
    /// Target time between blocks in seconds
    pub target_block_time_secs: u64,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            initial_difficulty: 1000,
            target_block_time_secs: 60, // 1 minute
        }
    }
}

pub struct MinerConfig {
    /// Maximum attempts before giving up
    pub max_nonce: u128,
}
```

## Constants

```rust
/// The number of blocks in each epoch
pub const BLOCKS_PER_EPOCH: u64 = 40;
```

## Error Handling

```rust
pub enum MiningError {
    MiningFailed(String),
    InvalidBlock(String),
    InvalidChain(String),
    BlockNotFound(String),
    InvalidNonce,
    SerializationError(String),
    HashError(String),
    PersistenceError(String), // Only with persistence feature
}
```

## Testing

```bash
# Run all tests (without persistence)
cargo test

# Run with persistence tests
cargo test --features persistence

# Run specific test
cargo test test_full_blockchain_lifecycle
```

## Performance

- **Mining**: Variable based on difficulty. Lower difficulty = faster mining.
- **Validation**: O(n) for chain validation, O(1) for single block
- **Epoch Operations**: O(n) where n = BLOCKS_PER_EPOCH (40)
- **Persistence**: Async I/O with RocksDB backend

## Use Cases

1. **Network Validator Selection**: Use shuffled nominations to determine validator sets
2. **Consensus Mechanism**: Proof-of-work provides Sybil resistance
3. **Reward Distribution**: Track miner contributions via nominated peer IDs
4. **Governance**: Use shuffle order for proposal voting
5. **Timestamping**: Immutable record of events

## Dependencies

- `sha2`: SHA-256 hashing
- `chrono`: Timestamp handling
- `serde`/`serde_json`: Serialization
- `thiserror`: Error handling
- `modality-utils`: Fisher-Yates shuffle and utilities
- `modality-network-datastore`: Optional persistence (feature-gated)
- `async-trait`, `tokio`: Optional async support (feature-gated)

## License

MIT

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## Related Packages

- `modality-network-datastore`: Persistent storage for blockchain data
- `modality-network-consensus`: Consensus mechanisms using mining data
- `modality-utils`: Shared utilities including crypto functions
