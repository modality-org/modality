# Fork Choice Integration

The miner now inherits the observer's sophisticated fork choice logic by depending on `modal-observer`. This enables proper handling of chain reorganizations and competing forks.

## Overview

The integration adds the following capabilities to the miner:

1. **Fork Detection**: Automatically detects competing forks when blocks are received
2. **Cumulative Difficulty Comparison**: Evaluates chain weight based on total difficulty
3. **Chain Reorganization**: Switches to heavier chains when appropriate
4. **Orphan Handling**: Properly marks and tracks orphaned blocks
5. **Forced Fork Support**: Allows operators to specify required blocks at specific heights

## Key Components

### MinerForkChoice

The `MinerForkChoice` struct wraps the `ChainObserver` and provides a miner-friendly API:

```rust
use modal_miner::{MinerForkChoice, ForkConfig};
use modal_datastore::NetworkDatastore;
use std::sync::Arc;
use tokio::sync::Mutex;

let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory()?));
let fork_choice = MinerForkChoice::new(datastore);
```

### Blockchain Integration

When persistence is enabled, the `Blockchain` automatically creates and uses a `MinerForkChoice` instance:

```rust
let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_directory(path)?));
let chain = Blockchain::load_or_create(config, genesis_peer, datastore).await?;
```

## Fork Choice Rules

### 1. Actualized Difficulty

The observer calculates **actualized difficulty** (based on actual hash values, not just target difficulty) for competing forks and selects the heaviest chain:

```rust
// Automatic actualized difficulty comparison
let accepted = chain.process_gossiped_block(block).await?;
```

Actualized difficulty = `max_target / hash_value`. A lower hash (more leading zeros) means more work was performed.

### 2. Single Block Fork Resolution

For single-block forks (two blocks at the same height):
- Block with **higher actualized difficulty wins**
- Equal actualized difficulty uses **first-seen tiebreaker** (keep existing block)

### 3. Tiebreaking

When two chains have equal cumulative actualized difficulty, the longer chain wins.

### 4. Forced Forks

Operators can specify required blocks at specific heights to enforce specific fork choices:

```rust
use modal_miner::ForkConfig;

let fork_config = ForkConfig::from_pairs(vec![
    (1000, "abc123...".to_string()),  // Block 1000 must have this hash
    (2000, "def456...".to_string()),  // Block 2000 must have this hash
]);

let fork_choice = MinerForkChoice::new_with_fork_config(datastore, fork_config);
```

## API Methods

### Processing Blocks

```rust
// Process a gossiped block from the network
let accepted: bool = chain.process_gossiped_block(block).await?;

// Add a mined block with fork choice (used internally by mine_block_with_persistence)
chain.add_block_with_fork_choice(block).await?;
```

### Querying Chain State

```rust
// Get the current chain tip height
let tip = fork_choice.get_chain_tip().await?;

// Get a specific block
let block = fork_choice.get_canonical_block(index).await?;
```

## Chain Reorganization

When a heavier competing fork is detected, the miner automatically:

1. Marks existing blocks as orphaned
2. Adopts the competing fork as canonical
3. Updates the in-memory chain state
4. Logs the reorganization event

Example log output:

```
INFO: Chain reorganization evaluation at fork point 100: 
      existing branch (5 blocks, difficulty 50000) vs 
      new branch (6 blocks, difficulty 55000)
INFO: New branch has higher cumulative difficulty - accepting reorganization
INFO: Marked 5 existing blocks as orphaned
INFO: Adopted 6 blocks from competing fork
```

## Migration Guide

### Old API (without fork choice)

```rust
let datastore = Arc::new(NetworkDatastore::create_in_directory(path)?);
let chain = Blockchain::load_or_create(config, genesis_peer, datastore).await?;
```

### New API (with fork choice)

```rust
use tokio::sync::Mutex;

let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_directory(path)?));
let chain = Blockchain::load_or_create(config, genesis_peer, datastore).await?;
// Fork choice is automatically initialized and used
```

**Key Change**: The datastore must now be wrapped in `Arc<Mutex<NetworkDatastore>>` instead of `Arc<NetworkDatastore>` to support the observer's concurrent access patterns.

## Testing

The fork choice integration includes comprehensive tests:

```bash
cargo test -p modal-miner --features persistence
```

Key test scenarios:
- Fork detection and selection
- Chain reorganization
- Cumulative difficulty calculation
- Orphan handling
- Forced fork specification

## Performance Considerations

1. **Memory**: The miner reloads canonical chain from datastore after accepting blocks to stay in sync
2. **Disk I/O**: Fork choice decisions involve querying the datastore for competing chains
3. **Logging**: Fork choice events are logged at INFO level for operational visibility

## Example Usage

See `examples/persistence_demo.rs` for a complete example of using the miner with fork choice enabled.

## Dependencies

- `modal-observer ^0.1.0`: Provides the ChainObserver and fork choice logic
- `modal-datastore ^0.1.0`: Required for persistence and fork tracking
- `tokio`: Required for async/await support

## Further Reading

- Observer fork choice implementation: `rust/modal-observer/src/chain_observer.rs`
- Chain reorganization tests: `rust/modal-observer/tests/chain_observer_integration.rs`
- Observer README: `rust/modal-observer/README.md`

