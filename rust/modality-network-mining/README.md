# Modality Network Mining

A proof-of-work blockchain implementation for the Modality network, featuring epoch-based difficulty adjustment and hash-tax-based mining.

## Features

- **Proof-of-Work Mining**: Uses the hash_tax module for efficient mining
- **Epoch Management**: 40 blocks per epoch with automatic difficulty adjustment
- **Ed25519 Public Key System**: Each block records the miner's public key
- **Arbitrary Miner Number**: Miners can record any number they choose in each block
- **Difficulty Adjustment**: Dynamic difficulty based on block mining time
- **Chain Validation**: Comprehensive validation of blocks and chains

## Architecture

### Blocks

Each block contains:
- **Header**: Index, timestamp, previous hash, data hash, nonce, difficulty, and hash
- **Block Data**: 
  - **Nominated Public Key**: Ed25519 public key nominated by the miner (to be used downstream)
  - **Miner Number**: An arbitrary u64 number selected by the miner

### Epochs

The blockchain is divided into epochs of 40 blocks each. At the end of each epoch:

**Difficulty Adjustment:**
- If blocks were mined too quickly, difficulty increases
- If blocks were mined too slowly, difficulty decreases
- Target block time is configurable (default: 1 minute per block)

**Nomination Shuffling:**
- All nonces from the epoch are XORed together to create a deterministic seed
- The 40 nominated public keys are shuffled using the Fisher-Yates algorithm with this seed
- The shuffled nominations can be used downstream for consensus, governance, or other purposes
- The shuffle is completely deterministic: same blocks always produce the same shuffle

### Mining

Mining uses the `hash_tax` module from `modality-utils` to find a valid nonce that satisfies the difficulty requirement. The miner:

1. Creates block data with their public key and chosen number
2. Finds a nonce that produces a hash below the difficulty target
3. Returns the mined block with valid nonce and hash

## Usage

### Create a new blockchain

```rust
use modality_network_mining::{Blockchain, ChainConfig, SigningKey};

// Generate a signing key for genesis
let genesis_key = SigningKey::from_bytes(&[1u8; 32]);

// With default configuration
let mut chain = Blockchain::new_default(genesis_key.verifying_key());

// With custom configuration
let config = ChainConfig {
    initial_difficulty: 1000,
    target_block_time_secs: 60, // 1 minute
};
let mut chain = Blockchain::new(config, genesis_key.verifying_key());
```

### Mine blocks

```rust
use modality_network_mining::SigningKey;

// Generate a key to nominate
let nominated_key = SigningKey::from_bytes(&[2u8; 32]);

// Mine a block nominating a public key with an arbitrary number
let mined_block = chain.mine_block(nominated_key.verifying_key(), 12345)?;

println!("Mined block {} with nominated key and number {}", 
    mined_block.header.index, 
    mined_block.data.miner_number
);
```

### Query the chain

```rust
// Get blockchain height
let height = chain.height();

// Get current epoch
let epoch = chain.current_epoch();

// Count blocks that nominated a specific public key
let nominated_key = SigningKey::from_bytes(&[2u8; 32]);
let count = chain.count_blocks_by_nominated_key(&nominated_key.verifying_key());

// Get all blocks that nominated a key
let blocks = chain.get_blocks_by_nominated_key(&nominated_key.verifying_key());

// Get block by hash
let block = chain.get_block_by_hash("block_hash");

// Get all blocks in an epoch
let epoch_blocks = chain.get_epoch_blocks(0);

// Get shuffled nominations for a complete epoch
if let Some(shuffled) = chain.get_epoch_shuffled_nominations(0) {
    // Returns Vec<(block_index, nominated_key)> in shuffled order
    for (idx, key) in shuffled.iter().take(10) {
        println!("Position in shuffle: {}, Original block: {}", idx, idx);
    }
}

// Get just the shuffled keys (without indices)
if let Some(keys) = chain.get_epoch_shuffled_keys(0) {
    println!("Got {} shuffled keys", keys.len());
}

// Validate entire chain
chain.validate_chain()?;
```

### Direct mining

```rust
use modality_network_mining::{Block, BlockData, Miner, SigningKey};

// Create a miner
let miner = Miner::new_default();

// Create key to nominate
let nominated_key = SigningKey::from_bytes(&[2u8; 32]);

// Create block data with nominated key
let data = BlockData::new(nominated_key.verifying_key(), 12345);

// Create a block
let block = Block::new(1, "prev_hash".to_string(), data, 1000);

// Mine it
let mined_block = miner.mine_block(block)?;

// Verify it
assert!(miner.verify_block(&mined_block)?);
```

## Block Data Structure

The key innovation is that **blocks do not store transactions**. Instead, each block records:

1. **Nominated Public Key**: An Ed25519 public key chosen by the miner (to be used downstream for various purposes)
2. **Arbitrary Number**: Any u64 number the miner chooses to include

This design allows for:
- Miners to nominate any public key they choose
- The nominated key can be used downstream for consensus, governance, or other purposes
- Flexible use cases where the "miner number" can represent various things
- Minimal block size
- Easy tracking of nomination statistics per public key

## Configuration

### Chain Configuration

- `initial_difficulty`: Starting difficulty for the chain
- `target_block_time_secs`: Target time between blocks (affects difficulty adjustment)

### Epoch Configuration

- `blocks_per_epoch`: Number of blocks per epoch (default: 40)
- `target_block_time_secs`: Target block time
- `initial_difficulty`: Starting difficulty
- `min_difficulty` / `max_difficulty`: Bounds for difficulty adjustment

### Miner Configuration

- `max_tries`: Maximum attempts to find a valid nonce
- `hash_func_name`: Hash function to use (default: "sha256")

## Examples

See the `examples` directory for comprehensive examples:
- `basic_usage.rs`: Creating chains, mining blocks, and querying data
- `epoch_shuffle_demo.rs`: Demonstrating epoch-based nomination shuffling

Run an example:
```bash
cargo run --package modality-network-mining --example basic_usage
cargo run --package modality-network-mining --example epoch_shuffle_demo
```

## Testing

Run the test suite:

```bash
cargo test --package modality-network-mining
```

Run with output:

```bash
cargo test --package modality-network-mining -- --nocapture
```

## License

MIT
