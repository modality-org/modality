# Modality Network Mining

A proof-of-work blockchain implementation for the Modality network, featuring epoch-based difficulty adjustment and hash-tax-based mining.

## Features

- **Proof-of-Work Mining**: Uses the hash_tax module for efficient mining
- **Epoch Management**: 40 blocks per epoch with automatic difficulty adjustment
- **Transaction Support**: Full transaction lifecycle with merkle tree verification
- **Difficulty Adjustment**: Dynamic difficulty based on block mining time
- **Chain Validation**: Comprehensive validation of blocks and chains

## Architecture

### Blocks

Each block contains:
- **Header**: Index, timestamp, previous hash, merkle root, nonce, difficulty, and hash
- **Transactions**: List of transactions included in the block

### Epochs

The blockchain is divided into epochs of 40 blocks each. At the end of each epoch, the difficulty is automatically adjusted based on how quickly blocks were mined:

- If blocks were mined too quickly, difficulty increases
- If blocks were mined too slowly, difficulty decreases
- Target block time is configurable (default: 10 minutes per block)

### Mining

Mining uses the `hash_tax` module from `modality-utils` to find a valid nonce that satisfies the difficulty requirement. The miner:

1. Takes a block with transactions
2. Finds a nonce that produces a hash below the difficulty target
3. Returns the mined block with valid nonce and hash

## Usage

### Create a new blockchain

```rust
use modality_network_mining::{Blockchain, ChainConfig};

// With default configuration
let mut chain = Blockchain::new_default();

// With custom configuration
let config = ChainConfig {
    initial_difficulty: 1000,
    target_block_time_secs: 600, // 10 minutes
};
let mut chain = Blockchain::new(config);
```

### Add transactions and mine

```rust
use modality_network_mining::Transaction;

// Add a transaction
let tx = Transaction::new(
    "alice".to_string(),
    "bob".to_string(),
    100,
    Some("Payment for services".to_string()),
);

chain.add_transaction(tx);

// Mine pending transactions
let mined_block = chain.mine_pending_transactions("miner_address", 50)?;
```

### Query the chain

```rust
// Get blockchain height
let height = chain.height();

// Get current epoch
let epoch = chain.current_epoch();

// Get balance for an address
let balance = chain.get_balance("alice");

// Get block by hash
let block = chain.get_block_by_hash("block_hash");

// Get all blocks in an epoch
let epoch_blocks = chain.get_epoch_blocks(0);

// Validate entire chain
chain.validate_chain()?;
```

### Direct mining

```rust
use modality_network_mining::{Block, Miner};

// Create a miner
let miner = Miner::new_default();

// Create a block
let block = Block::new(1, "prev_hash".to_string(), vec![], 1000);

// Mine it
let mined_block = miner.mine_block(block)?;

// Verify it
assert!(miner.verify_block(&mined_block)?);
```

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

See the `tests` directory for comprehensive examples of:
- Creating and mining blocks
- Managing transactions
- Epoch transitions and difficulty adjustment
- Chain validation
- Balance tracking

## Testing

Run the test suite:

```bash
cargo test
```

Run with output:

```bash
cargo test -- --nocapture
```

## License

MIT

