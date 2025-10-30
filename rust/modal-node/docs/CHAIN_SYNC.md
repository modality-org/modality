# Chain Synchronization and Validation

This document explains how Modality nodes maintain chain consistency and prevent getting out of sync.

## Problem Statement

In a distributed mining network, nodes can get out of sync when:

1. **Orphan Blocks**: Node receives block N+5 but doesn't have blocks N+1 through N+4
2. **Network Partitions**: Node is disconnected and misses blocks
3. **Race Conditions**: Fastest miner wins repeatedly, other nodes fall behind
4. **Startup**: New nodes join the network without any blocks

## Solution Architecture

### 1. Gossip Validation (Chain Continuity)

**Location:** `rust/modality-network-node/src/gossip/miner/block.rs`

When receiving a gossiped block, the handler now:

```rust
// Validate we have the parent block
if miner_block.index > 0 {
    match find_by_hash(&miner_block.previous_hash) {
        None => {
            log::warn!("Missing parent block - orphan detected!");
            return Ok(()); // Reject orphan blocks
        }
        Some(parent) => {
            // Validate parent is canonical
            // Validate parent index is N-1
        }
    }
}
```

**What this prevents:**
- Orphan blocks (blocks without parents) from corrupting the chain
- Invalid blocks from being saved
- Nodes accepting blocks they can't validate

**What happens when orphan detected:**
- Block is rejected (not saved)
- Warning logged: `"Orphan block detected - need to sync!"`
- Node should sync from peers to fill gaps

### 2. Startup Sync

**Location:** `rust/modality-network-node/src/actions/miner.rs`

Before starting to mine, nodes now:

```rust
if !node.bootstrappers.is_empty() {
    // Wait for peer connections
    node.wait_for_connections().await?;
    
    // Sync from peers
    log::info!("Syncing blockchain state from peers...");
    sync_from_peers(node).await?;
}

// THEN start mining
log::info!("Starting miner...");
```

**Current Implementation:**
- Checks local chain height
- Logs sync attempt
- Waits 2 seconds for gossip to propagate blocks

**Future Enhancement:**
- Query peers for their chain heights
- Request blocks from peers with longer chains
- Validate and adopt the longest valid chain

### 3. Fork Choice with Validation

**Location:** Both `miner.rs` and `gossip/miner/block.rs`

After validating chain continuity, apply fork choice:

```rust
match find_canonical_by_index(block.index) {
    Some(existing) => {
        // Lower hash = more work = wins
        if new_block.hash < existing.hash {
            replace_and_orphan(existing, new_block);
        }
    }
    None => save(new_block),
}
```

## Chain Validation Rules

A block is accepted if ALL of these are true:

1. ✅ **Parent exists**: Block N requires block N-1 to exist
2. ✅ **Parent is canonical**: Parent must not be orphaned
3. ✅ **Index sequential**: Parent index must be exactly N-1
4. ✅ **Hash valid**: Block hash meets difficulty requirement
5. ✅ **Fork choice**: If competing block exists, lower hash wins

## Sync Strategies

### Passive Sync (Current)

Nodes rely on gossip to receive blocks:
- Blocks propagate via gossip network
- Valid blocks are accepted
- Invalid/orphan blocks are rejected

**Pros:**
- Simple implementation
- No extra network requests
- Works well for small networks

**Cons:**
- Can miss blocks if gossip fails
- Nodes may stay out of sync
- No active recovery from gaps

### Active Sync (TODO)

Nodes actively request missing blocks:

```rust
// Pseudocode for future implementation
async fn sync_from_peers(node: &Node) -> Result<()> {
    // 1. Get local height
    let local_height = get_chain_height();
    
    // 2. Query peers for their heights
    let peer_heights = query_peer_heights().await?;
    
    // 3. Find peer with longest chain
    let best_peer = peer_heights.iter().max_by_key(|p| p.height);
    
    // 4. Request missing blocks
    if best_peer.height > local_height {
        log::info!("Syncing {} blocks from peer", 
            best_peer.height - local_height);
        
        let blocks = request_blocks(best_peer, local_height, best_peer.height).await?;
        
        // 5. Validate and save blocks
        for block in blocks {
            validate_and_save(block)?;
        }
    }
    
    Ok(())
}
```

## Behavior in Different Scenarios

### Scenario 1: Node Starts Fresh

```
Node 1 (existing): Blocks 0-100
Node 2 (new):      Blocks 0 (just started)

1. Node 2 connects to Node 1
2. Node 2 calls sync_from_peers()
3. Currently: Waits for gossip
4. Future: Requests blocks 0-100 from Node 1
5. Node 2 validates and saves each block
6. Node 2 starts mining at block 101
```

### Scenario 2: Network Partition Heals

```
Node 1: Blocks 0-80 (was mining alone)
Node 2: Blocks 0-50 (got disconnected at block 50)

1. Connection restored
2. Node 1 gossips block 81
3. Node 2 receives block 81
4. Parent validation: Block 80 missing!
5. Block 81 rejected as orphan
6. Node 2 needs to sync blocks 51-80
7. Currently: Stays at block 50
8. Future: Requests blocks 51-80, then accepts 81
```

### Scenario 3: Competing Miners

```
Node 1 mines: Blocks 0, 1, 2, 3...
Node 2 mines: Also trying blocks 1, 2, 3...
Node 3 mines: Also trying blocks 1, 2, 3...

1. All receive block 0 via gossip
2. All start mining block 1
3. Node 1 finishes first, gossips block 1a
4. Node 2 receives 1a, validates parent (block 0 ✓)
5. Node 2 checks: Do I have block 1?
   - If yes, fork choice: keep harder hash
   - If no, save block 1a
6. All nodes converge on same block 1
7. Repeat for block 2, 3, etc.
```

### Scenario 4: Fastest Miner Dominates

```
Node 1 (fast): Wins 70% of blocks
Node 2 (slow): Wins 20% of blocks
Node 3 (slow): Wins 10% of blocks

This is EXPECTED behavior with:
- Equal difficulty
- Proof of work
- Fork choice based on hash

To balance:
1. Adjust difficulty per-miner (not implemented)
2. Use round-robin mining (not implemented)
3. Pool mining (not implemented)
```

## Monitoring Sync Status

### Check Local Chain

```bash
./02-inspect-blocks.sh
```

Look for:
```
Total Blocks: 80
Block Range: 0 → 79
```

### Check for Orphan Warnings

```bash
grep "Orphan block detected" logs
grep "Missing parent block" logs
```

If you see these frequently, the node is out of sync.

### Check Gossip Reception

```bash
grep "Accepting new gossiped block" logs | wc -l
```

Count how many blocks were received via gossip.

### Check Mining Success

```bash
grep "Successfully mined and gossipped block" logs | wc -l
```

Compare to total blocks to see success rate.

## Troubleshooting

### Problem: Node stuck at old block

**Symptoms:**
```
[INFO] Mining block at index 50...
[WARN] Orphan block detected for block 75!
```

**Solution:**
```bash
# Stop the node
# Sync manually from a peer
modality net mining sync \
  --config ./config.json \
  --target /ip4/<peer>/tcp/<port>/p2p/<peerid> \
  --mode all \
  --persist

# Restart node
```

### Problem: Nodes on different chains

**Symptoms:**
```
Node 1: Block 80 hash: 0000abc...
Node 2: Block 80 hash: 0000def...
```

**Cause:** Fork not resolved (both have different blocks at same index)

**Solution:**
- Fork choice should resolve automatically
- If not, lower hash should win
- Manual intervention: adopt longest chain

### Problem: Orphan blocks keep appearing

**Symptoms:**
```
[WARN] Missing parent block (prev_hash: 0000...)
[WARN] Orphan block detected
```

**Cause:** Gossip delivering blocks out of order

**Solution:**
- Current: Blocks are rejected, node should sync
- Future: Implement orphan block pool + active sync

## Future Enhancements

### 1. Orphan Block Pool

Store orphan blocks temporarily:
```rust
struct OrphanPool {
    orphans: HashMap<String, MinerBlock>, // hash -> block
}

// When receiving block
if parent_missing {
    orphan_pool.add(block);
    request_parent(block.previous_hash);
}

// When parent arrives
if let Some(children) = orphan_pool.get_children(block.hash) {
    for child in children {
        process_block(child);
    }
}
```

### 2. Active Peer Discovery

Query all peers for chain heights:
```rust
async fn discover_best_chain() -> (PeerId, u64) {
    let mut best = (None, 0);
    for peer in connected_peers() {
        let height = query_chain_height(peer).await?;
        if height > best.1 {
            best = (Some(peer), height);
        }
    }
    best
}
```

### 3. Batch Block Sync

Request blocks in batches:
```rust
async fn sync_range(peer: PeerId, from: u64, to: u64) -> Result<Vec<Block>> {
    let batch_size = 100;
    let mut all_blocks = Vec::new();
    
    for start in (from..=to).step_by(batch_size) {
        let end = (start + batch_size).min(to);
        let blocks = request_block_range(peer, start, end).await?;
        all_blocks.extend(blocks);
    }
    
    Ok(all_blocks)
}
```

### 4. Chain Validation

Validate entire chain after sync:
```rust
async fn validate_chain() -> Result<bool> {
    let blocks = load_all_blocks().await?;
    
    for i in 1..blocks.len() {
        if blocks[i].previous_hash != blocks[i-1].hash {
            return Ok(false);
        }
        if !validate_pow(&blocks[i]) {
            return Ok(false);
        }
    }
    
    Ok(true)
}
```

## Related Documentation

- [Fork Choice Rules](./FORK_CHOICE.md)
- [Miner Block Sync Protocol](./MINER_BLOCK_SYNC.md)
- [Gossip Protocol](./GOSSIP.md)

## Testing

Test sync behavior:

```bash
# Terminal 1: Start node 1
./node1.sh

# Terminal 2: Let it mine 50 blocks, then start node 2
./node2.sh

# Terminal 3: Check node 2 syncs
watch -n 1 'grep "Total Blocks" node2/logs'
```

Expected: Node 2 should catch up via gossip or sync.

