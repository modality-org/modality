# Efficient Common Ancestor Finding

This document explains how to use the `/data/miner_block/find_ancestor` reqres route to efficiently find the common ancestor between two blockchain chains.

## Overview

When two nodes have divergent blockchain chains (e.g., due to a network partition or competing blocks), they need to find the **common ancestor** - the last block where their chains agree. This is crucial for:

1. **Chain synchronization** - Knowing which blocks to request from peers
2. **Fork resolution** - Determining which chain has more work (cumulative difficulty)
3. **Reorg detection** - Identifying when a reorganization is needed

## The Problem with Naive Approaches

A naive approach would send all local block hashes to the remote peer and ask which ones match. For a chain with `n` blocks, this has:
- **Time Complexity**: O(n) - proportional to chain length
- **Network Usage**: O(n) - must send all hashes
- **Not scalable**: Becomes slow and bandwidth-intensive as the chain grows

## The Efficient Solution

The `/data/miner_block/find_ancestor` route uses a **binary search** approach:

1. **Exponential Search Phase**: Check blocks at exponential intervals [tip, tip-1, tip-2, tip-4, tip-8, tip-16, ...]
2. **Binary Search Phase**: Once the divergence range is found, perform binary search within that range

This reduces the complexity to:
- **Time Complexity**: O(log n)
- **Network Usage**: O(log n) requests
- **Scalable**: Works efficiently even with millions of blocks

## Route Specification

### Endpoint
```
POST /data/miner_block/find_ancestor
```

### Request Format
```json
{
  "check_points": [
    { "index": 100, "hash": "hash_at_100" },
    { "index": 99, "hash": "hash_at_99" },
    { "index": 98, "hash": "hash_at_98" },
    { "index": 96, "hash": "hash_at_96" },
    { "index": 92, "hash": "hash_at_92" },
    ...
  ]
}
```

### Response Format
```json
{
  "ok": true,
  "data": {
    "chain_length": 101,
    "matches": [
      { "index": 100, "hash": "hash_at_100", "matches": false },
      { "index": 99, "hash": "hash_at_99", "matches": false },
      { "index": 98, "hash": "hash_at_98", "matches": true },
      ...
    ],
    "highest_match": 98,
    "cumulative_difficulty": "1234567890"
  },
  "errors": null
}
```

## Usage from Rust

The helper function `find_common_ancestor_efficient` in `actions/miner.rs` implements the full binary search algorithm:

```rust
use crate::actions::miner::find_common_ancestor_efficient;

// Find common ancestor with a peer
match find_common_ancestor_efficient(
    &swarm,
    peer_addr.to_string(),
    &datastore
).await {
    Ok(Some(ancestor_index)) => {
        println!("Common ancestor found at block {}", ancestor_index);
        // Now request blocks from ancestor_index + 1 onwards
    }
    Ok(None) => {
        println!("No common ancestor - chains have different genesis");
    }
    Err(e) => {
        eprintln!("Error finding ancestor: {}", e);
    }
}
```

## Example Scenario

```
Local chain:  [0] -> [1] -> [2] -> [3] -> [4] -> [5]
Remote chain: [0] -> [1] -> [2] -> [3] -> [4'] -> [5'] -> [6']

The chains diverged at block 4.
```

### Phase 1: Exponential Search

**Request 1**: Check blocks [5, 4, 3, 1, 0]
```json
{
  "check_points": [
    { "index": 5, "hash": "local_hash_5" },
    { "index": 4, "hash": "local_hash_4" },
    { "index": 3, "hash": "local_hash_3" },
    { "index": 1, "hash": "local_hash_1" },
    { "index": 0, "hash": "local_hash_0" }
  ]
}
```

**Response 1**: 
```json
{
  "chain_length": 7,
  "matches": [
    { "index": 5, "hash": "local_hash_5", "matches": false },
    { "index": 4, "hash": "local_hash_4", "matches": false },
    { "index": 3, "hash": "local_hash_3", "matches": true },
    { "index": 1, "hash": "local_hash_1", "matches": true },
    { "index": 0, "hash": "local_hash_0", "matches": true }
  ],
  "highest_match": 3
}
```

Now we know:
- Blocks 0-3 match (common)
- Block 4+ diverged
- Search space: [3, 4]

### Phase 2: Binary Search (if needed)

Since the search space is just 1 block apart, we've found the answer: **common ancestor is block 3**.

For larger gaps, we'd continue with binary search:
```
Check midpoint between 3 and 4... (but gap is 1, so we're done)
```

## Performance Comparison

For a chain of 1 million blocks with divergence at block 500,000:

| Approach | Requests | Data Sent |
|----------|----------|-----------|
| Naive | 1 | ~64 MB |
| Binary Search | ~20 | ~1.3 KB |

**Result**: 50,000x less data and instant response!

## Integration with Chain Sync

After finding the common ancestor, you can efficiently sync:

```rust
// 1. Find common ancestor
let ancestor = find_common_ancestor_efficient(&swarm, peer_addr, &datastore).await?;

// 2. Get remote chain info
let chain_info = request_chain_info(&swarm, peer_addr).await?;

// 3. Compare cumulative difficulty
if remote_difficulty > local_difficulty {
    // 4. Request blocks from ancestor+1 to remote_tip
    let missing_blocks = request_block_range(
        &swarm,
        peer_addr,
        ancestor.unwrap() + 1,
        chain_info.chain_length - 1
    ).await?;
    
    // 5. Perform chain reorganization
    attempt_chain_reorg(&mut datastore, missing_blocks).await?;
}
```

## Notes

- The algorithm works even if chains have completely diverged (returns `None`)
- Handles edge cases like empty chains, genesis-only chains
- Timeout protection on all network requests
- Logarithmic complexity ensures scalability to any chain length

