# Fork Choice Rules

This document describes the fork choice algorithm used by Modality network nodes to resolve competing blocks.

## Overview

When multiple blocks exist at the same index (competing blocks), or when chains diverge, the network must choose which becomes canonical. Modality uses a cumulative difficulty-based fork choice rule:

**At the block level:** The block with higher difficulty wins.
**At the chain level:** The chain with higher cumulative difficulty (total work) wins.

## Rationale

The fork choice rule is based on **proof of work** - the chain that required the most computational effort to produce is considered canonical. This prevents attackers from creating longer but easier chains to displace the legitimate chain.

### Why Cumulative Difficulty?

Simply comparing chain lengths is vulnerable to attack:
- A long chain with low difficulty (easy mining) could displace a shorter chain with high difficulty
- An attacker with less hashpower could create a longer but weaker chain

Cumulative difficulty ensures:
- The chain with the most computational work always wins
- Miners are incentivized to mine at appropriate difficulty levels
- Network converges on the strongest chain, not just the longest

### Example

```
Chain A: [block 0: diff 1000] → [block 1: diff 2000] → [block 2: diff 2000]
         Total: 3 blocks, cumulative difficulty = 5000

Chain B: [block 0: diff 1000] → [block 1: diff 100] → [block 2: diff 100] → [block 3: diff 100] → [block 4: diff 100]
         Total: 5 blocks, cumulative difficulty = 1400

Winner: Chain A (higher cumulative difficulty despite fewer blocks)
```

## Implementation

Fork choice is applied in three places:

### 1. Miner Saves Own Block

When a miner saves a block it just mined:

```rust
// File: rust/modality-network-node/src/actions/miner.rs

match MinerBlock::find_canonical_by_index(&ds, index).await? {
    Some(existing) => {
        // Apply fork choice: higher difficulty wins
        let new_difficulty = mined_block.header.difficulty;
        let existing_difficulty = existing.get_difficulty_u128()?;
        
        if new_difficulty > existing_difficulty {
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
- New block might have higher difficulty

### 2. Gossip Receives Block

When a node receives a block via gossip:

```rust
// File: rust/modality-network-node/src/gossip/miner/block.rs

match MinerBlock::find_canonical_by_index(datastore, block.index).await? {
    Some(existing) => {
        let new_difficulty = miner_block.get_difficulty_u128()?;
        let existing_difficulty = existing.get_difficulty_u128()?;
        
        if new_difficulty > existing_difficulty {
            // Gossiped block has higher difficulty, replace local
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
- Nodes converge on the block with higher difficulty

### 3. Chain Reorganization (Sync)

When syncing from another node and chains diverge:

```rust
// File: rust/modality-network-node/src/actions/miner.rs

// Calculate cumulative difficulty for both chains
let local_difficulty = MinerBlock::calculate_cumulative_difficulty(&local_blocks)?;
let peer_difficulty = MinerBlock::calculate_cumulative_difficulty(&peer_blocks)?;

if peer_difficulty > local_difficulty {
    // Adopt peer chain - it has more total work
    orphan_local_blocks();
    save_peer_blocks();
} else {
    // Keep local chain
    reject_peer_blocks();
}
```

**When it triggers:**
- Node syncs from another node's chain
- Chains have diverged (different blocks at same indices)
- Local node adopts the chain with higher cumulative difficulty

## Orphaned Blocks

When a block is replaced by fork choice:

1. **Marked as orphaned**: `is_canonical = false`, `is_orphaned = true`
2. **Reason recorded**: Includes difficulty comparison details
3. **Competing hash stored**: Reference to the winning block
4. **Preserved in database**: Still queryable for analysis

```rust
orphaned.mark_as_orphaned(
    format!("Replaced by block with higher difficulty ({} vs {})", 
        new_difficulty, old_difficulty),
    Some(winning_block.hash)
);
orphaned.save(datastore).await?;
```

## Chain Reorganization

Fork choice can cause chain reorganizations (reorgs) when chains diverge:

```
Initial chain:
  0 → 1a (diff 1000) → 2a (diff 1000) → 3a (diff 1000)
  Cumulative: 3000

Receive competing chain:
  0 → 1b (diff 2000) → 2b (diff 2000)
  Cumulative: 4000

Result: Chain B becomes canonical despite having fewer blocks
  0 → 1b (diff 2000) → 2b (diff 2000)  (new canonical chain)
  
Orphaned: 1a, 2a, 3a
```

### Reorg Types

**Partial Reorg:** Chains share a common ancestor
- Only blocks after the common ancestor are affected
- Cumulative difficulty compared for divergent branches only

**Complete Reorg:** No common ancestor found
- Entire chains compared by cumulative difficulty
- All local blocks orphaned if peer chain has higher difficulty

### Reorg Safety

**Short reorgs (< 6 blocks):** Common and expected
**Long reorgs (> 40 blocks):** Indicate a serious fork or attack

Applications should:
- Wait for multiple confirmations before considering blocks final
- Monitor for reorgs via orphaned block events
- Have logic to handle state rollbacks

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

### Testing Cumulative Difficulty

To test that cumulative difficulty is working correctly:

1. Create two competing chains with different difficulties
2. Verify the chain with higher cumulative difficulty wins, even if shorter
3. Check log messages show difficulty comparisons
4. Verify orphaned blocks have correct reasons recorded

## Comparison with Other Systems

| System | Fork Choice Rule |
|--------|-----------------|
| Bitcoin | Longest chain (most cumulative work) |
| Ethereum | GHOST (Greedy Heaviest Observed SubTree) |
| Modality | Cumulative difficulty (most total work) |

Modality's approach is similar to Bitcoin's, using cumulative difficulty to determine the canonical chain. This ensures:
- Security against attacks from less powerful miners
- Proper incentives for mining at appropriate difficulty levels
- Network convergence on the strongest chain

## Cumulative Difficulty Calculation

The cumulative difficulty helper function sums the difficulty values across a chain:

```rust
/// Calculate total work (cumulative difficulty) for a chain of blocks
pub fn calculate_cumulative_difficulty(blocks: &[MinerBlock]) -> Result<u128> {
    let mut total: u128 = 0;
    for block in blocks {
        let difficulty = block.get_difficulty_u128()?;
        total = total.checked_add(difficulty)
            .context("Cumulative difficulty overflow")?;
    }
    Ok(total)
}
```

This function is used in:
- Partial chain reorgs (comparing divergent branches)
- Complete chain reorgs (comparing entire chains)
- Chain selection during sync operations

## References

- Bitcoin's longest chain: https://bitcoin.org/bitcoin.pdf
- Ethereum's GHOST: https://eprint.iacr.org/2013/881.pdf
- Nakamoto consensus: https://en.wikipedia.org/wiki/Nakamoto_consensus


