# Fork Choice Rules

This document describes the fork choice algorithm used by Modality network nodes to resolve competing blocks.

## Overview

When multiple blocks exist at the same index (competing blocks), the network must choose which one becomes canonical. Modality uses a simple but effective fork choice rule:

**The block with the lowest hash wins.**

## Rationale

Lower hash values have more leading zeros, which means they were harder to mine (more computational work). This is similar to Bitcoin's longest chain rule but applied at the block level.

### Example

```
Block A at index 5: hash = 0000abc123...  (4 leading zeros)
Block B at index 5: hash = 000def4567...  (3 leading zeros)

Winner: Block A (lower hash = more work)
```

## Implementation

Fork choice is applied in three places:

### 1. Miner Saves Own Block

When a miner saves a block it just mined:

```rust
// File: rust/modality-network-node/src/actions/miner.rs

match MinerBlock::find_canonical_by_index(&ds, index).await? {
    Some(existing) => {
        // Apply fork choice: lower hash wins
        if miner_block.hash < existing.hash {
            // Replace existing with new block
            mark_as_orphaned(existing);
            save(miner_block);
        } else {
            // Keep existing block
            skip_save();
        }
    }
    None => {
        // No competition, save normally
        save(miner_block);
    }
}
```

**When it triggers:**
- Mining succeeds but gossip fails
- Miner retries and produces a different block for the same index
- New block might have a harder (lower) hash

### 2. Gossip Receives Block

When a node receives a block via gossip:

```rust
// File: rust/modality-network-node/src/gossip/miner/block.rs

match MinerBlock::find_canonical_by_index(datastore, block.index).await? {
    Some(existing) => {
        if gossiped_block.hash < existing.hash {
            // Gossiped block is harder, replace local
            mark_as_orphaned(existing);
            save(gossiped_block);
        } else {
            // Keep local block
            ignore_gossip();
        }
    }
    None => {
        save(gossiped_block);
    }
}
```

**When it triggers:**
- Two miners produce blocks for the same index
- Network receives both via gossip
- Nodes converge on the block with lowest hash

### 3. Sync Receives Blocks

When syncing from another node:

```rust
// File: rust/modality-network-node/src/actions/sync_blocks.rs

for block in synced_blocks {
    apply_fork_choice(block);
}
```

**When it triggers:**
- Node syncs from another node's chain
- Remote chain might have different blocks at same indices
- Local node adopts blocks with lower hashes

## Orphaned Blocks

When a block is replaced by fork choice:

1. **Marked as orphaned**: `is_canonical = false`, `is_orphaned = true`
2. **Reason recorded**: "Replaced by block with harder hash"
3. **Competing hash stored**: Reference to the winning block
4. **Preserved in database**: Still queryable for analysis

```rust
orphaned.mark_as_orphaned(
    "Replaced by block with harder hash".to_string(),
    Some(winning_block.hash)
);
orphaned.save(datastore).await?;
```

## Chain Reorganization

Fork choice can cause chain reorganizations (reorgs):

```
Initial chain:
  0 → 1a → 2a → 3a

Receive competing blocks:
  0 → 1b → 2b → 3b

If blocks 1b, 2b, 3b all have lower hashes:
  0 → 1b → 2b → 3b  (new canonical chain)
  
Orphaned: 1a, 2a, 3a
```

### Reorg Safety

**Short reorgs (< 6 blocks):** Common and expected
**Long reorgs (> 40 blocks):** Indicate a serious fork or attack

Applications should:
- Wait for multiple confirmations before considering blocks final
- Monitor for reorgs via orphaned block events
- Have logic to handle state rollbacks

## Future Enhancements

### Longest Chain Rule

Current: Block-level fork choice (lowest hash at each index)
Future: Chain-level fork choice (longest valid chain wins)

```rust
fn compare_chains(chain_a: &[Block], chain_b: &[Block]) -> Chain {
    if chain_a.len() > chain_b.len() {
        return chain_a;  // Longer chain wins
    }
    if chain_a.len() < chain_b.len() {
        return chain_b;
    }
    // Same length: compare cumulative difficulty
    if cumulative_difficulty(chain_a) > cumulative_difficulty(chain_b) {
        return chain_a;  // More work wins
    }
    return chain_b;
}
```

### Cumulative Difficulty

Instead of comparing individual hashes, compare total work:

```rust
fn cumulative_difficulty(chain: &[Block]) -> u128 {
    chain.iter()
        .map(|block| hash_to_difficulty(&block.hash))
        .sum()
}
```

## Testing

Fork choice is tested in:

1. **Unit tests**: `rust/modality-network-node/src/gossip/miner/block.rs`
2. **Integration tests**: `rust/modality-network-node/tests/fork_choice_tests.rs`
3. **Example**: `examples/network/05-mining` (duplicate prevention)

### Manual Testing

```bash
cd examples/network/05-mining

# Terminal 1: Start miner
./01-mine-blocks.sh

# Terminal 2: Inspect for duplicates
./02-inspect-blocks.sh

# Should show no duplicate indices, all unique block indices
```

## Comparison with Other Systems

| System | Fork Choice Rule |
|--------|-----------------|
| Bitcoin | Longest chain (most cumulative work) |
| Ethereum | GHOST (Greedy Heaviest Observed SubTree) |
| Modality | Lowest hash per block (simplest) |

Modality's approach is simpler but effective for:
- Small networks
- Development/testing
- Applications that don't need deep reorg resistance

For production, consider implementing longest chain or cumulative difficulty.

## References

- Bitcoin's longest chain: https://bitcoin.org/bitcoin.pdf
- Ethereum's GHOST: https://eprint.iacr.org/2013/881.pdf
- Nakamoto consensus: https://en.wikipedia.org/wiki/Nakamoto_consensus

