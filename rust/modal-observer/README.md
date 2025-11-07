# modal-observer

Validation and chain observation for Modality network consensus.

## Overview

This package provides the core functionality for validator nodes in the Modality network. Validators are a second class of consensus nodes that observe mining events and maintain the canonical chain without participating in mining themselves.

## Architecture

Validators have several key responsibilities:

1. **Chain Observation**: Listen to mining block gossip events and track the canonical chain
2. **Fork Choice**: Apply cumulative difficulty-based fork choice rules (implemented in the gossip handler)
3. **Consensus Participation**: Participate in the consensus protocol using the observed mining chain
4. **Data Serving**: Maintain a full datastore and serve block data to other nodes

## Key Components

### ChainObserver

The `ChainObserver` struct provides an API for tracking the canonical mining chain with proper fork choice rules:

```rust
use modal_observer::{ChainObserver, ForkConfig};

// Create observer with optional forced fork specification
let fork_config = ForkConfig::from_pairs(vec![
    (1000, "checkpoint_hash_at_1000".to_string()),
    (5000, "checkpoint_hash_at_5000".to_string()),
]);

let observer = ChainObserver::new_with_fork_config(datastore, fork_config);
observer.initialize().await?;

// Get current chain tip
let tip = observer.get_chain_tip().await;

// Get canonical blocks
let blocks = observer.get_all_canonical_blocks().await?;

// Process incoming gossiped blocks with fork choice
let accepted = observer.process_gossiped_block(new_block).await?;

// Process a competing chain (batch of blocks)
let competing_chain = vec![block1, block2, block3];
let adopted = observer.process_competing_chain(competing_chain).await?;

// Query orphaned blocks
let orphans = observer.get_all_orphaned_blocks().await?;
let orphans_at_height = observer.get_orphaned_blocks_at_index(50).await?;
```

### Forced Fork Specification

Node operators can override automatic fork choice by specifying required blocks at specific heights, including the genesis block:

```rust
use modal_observer::ForkConfig;

// Create a forced fork config (checkpoints)
let fork_config = ForkConfig::from_pairs(vec![
    (0, "genesis_block_hash".to_string()),        // Genesis checkpoint
    (1000, "block_hash_at_1000".to_string()),
    (2000, "block_hash_at_2000".to_string()),
    (3000, "block_hash_at_3000".to_string()),
]);

let observer = ChainObserver::new_with_fork_config(datastore, fork_config);
```

**Forced Fork Behavior:**
- Blocks at forced heights **must** match the specified hash
- Blocks with wrong hash are **rejected** and marked as orphans
- Forced forks **override** first-seen rule and difficulty comparisons
- Competing chains **must respect** all forced blocks to be considered
- **Genesis block (height 0) can be specified** to enforce a specific chain origin
- Useful for:
  - Network upgrades
  - Resolving contentious forks
  - Security checkpoints
  - Enforcing canonical genesis block
  - Development/testing scenarios

### Competing Chain Processing

When a potentially better chain is discovered, the observer can process it atomically:

```rust
// Fetch blocks from competing chain (e.g., via sync or gossip)
let competing_blocks = fetch_competing_chain().await?;

// Process the entire chain:
// 1. Validates chain is sequential and connected
// 2. Stores all blocks as non-canonical
// 3. Calculates full cumulative difficulty
// 4. Only adopts if heavier than current canonical chain
let adopted = observer.process_competing_chain(competing_blocks).await?;

if adopted {
    println!("Adopted heavier competing chain!");
} else {
    println!("Kept canonical chain (competitor was lighter)");
}
```

**Key Benefits:**
- **Atomic adoption** - All blocks evaluated together before any become canonical
- **Weight verification** - Full cumulative difficulty calculated before adoption
- **Safety** - Lighter chains are rejected, blocks marked as orphans
- **No partial state** - Either entire chain is adopted or none of it

### Orphan Block Storage

The observer automatically stores blocks that cannot be accepted into the canonical chain as orphans:

1. **Competing blocks** at the same index (rejected by first-seen rule)
2. **Blocks with missing parents** (gap in the chain)
3. **Blocks with wrong parent hash** (not extending canonical chain)

Orphaned blocks are tracked with:
- `is_orphaned`: true
- `orphan_reason`: Why the block was orphaned
- `competing_hash`: For competing blocks, the hash of the canonical block that won

**Orphan Promotion**: If a block is initially orphaned due to missing parent, it will be automatically promoted to canonical when its parent arrives.

### Fork Choice Rules

The ChainObserver implements a dual fork choice strategy:

1. **Single Block Forks**: When competing blocks exist at the same index, the **first-seen block is always kept**. This prevents flip-flopping and provides stability at the block level.

2. **Multi-Block Reorganizations**: When comparing competing chain branches, cumulative difficulty is used. A reorganization is only accepted if the new branch has:
   - Higher cumulative difficulty, OR
   - Equal cumulative difficulty AND more blocks (length tiebreaker)

This ensures that lighter chains (lower cumulative difficulty) cannot replace the canonical chain, even if they are longer, while maintaining stability for single block conflicts.

## Usage

Validator nodes are started using the CLI:

```bash
modality net run-validator --dir /path/to/node/dir
```

Or with a specific config file:

```bash
modality net run-validator --config /path/to/config.json
```

## Differences from Miners

| Feature | Miners | Validators |
|---------|--------|------------|
| Mine blocks | ✅ Yes | ❌ No |
| Listen to mining gossip | ✅ Yes | ✅ Yes |
| Maintain canonical chain | ✅ Yes | ✅ Yes |
| Participate in consensus | ❌ No | ✅ Yes |
| Full datastore | ✅ Yes | ✅ Yes |

## Development

Run tests:

```bash
cargo test -p modal-observer
```

