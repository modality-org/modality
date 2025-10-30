# Automatic Sync Mechanism

## Overview

The automatic sync mechanism detects when a node is behind the network and automatically triggers synchronization to catch up with the canonical chain.

## How It Works

### 1. Orphan Detection

When a node receives a gossiped block via the miner gossip protocol, it validates that the parent block exists:

```rust
// In gossip/miner/block.rs
if miner_block.index > 0 {
    match MinerBlock::find_by_hash(datastore, &miner_block.previous_hash).await? {
        None => {
            // Orphan detected! Missing parent block
            log::warn!("Orphan block detected - triggering sync!");
            
            // Trigger sync via broadcast channel
            if let Some(tx) = sync_trigger_tx {
                tx.send(miner_block.index)?;
            }
            return Ok(());
        }
        Some(parent) => {
            // Validate parent...
        }
    }
}
```

**Key Points:**
- Orphan blocks are **not** saved to the datastore
- A sync trigger is sent with the target block index
- The gossip handler continues processing other blocks

### 2. Sync Trigger Channel

The Node struct contains a broadcast channel for sync triggers:

```rust
pub struct Node {
    // ...
    pub sync_trigger_tx: tokio::sync::broadcast::Sender<u64>,
}
```

**Why Broadcast Channel?**
- Multiple tasks can subscribe to sync triggers
- Non-blocking send (won't slow down gossip processing)
- Rate limiting happens at the receiver level

### 3. Sync Listener Task

The miner spawns a dedicated sync listener task that:

1. **Subscribes to sync triggers** from the gossip handler
2. **Rate limits** sync requests (5-second cooldown)
3. **Checks if sync is needed** by comparing local height to target
4. **Logs the sync attempt** (placeholder for actual implementation)

```rust
tokio::spawn(async move {
    let mut last_sync_time = std::time::Instant::now();
    let sync_cooldown = std::time::Duration::from_secs(5);
    
    while let Ok(target_index) = sync_trigger_rx.recv().await {
        // Rate limit syncs
        if last_sync_time.elapsed() < sync_cooldown {
            continue;
        }
        
        log::info!("🔄 Sync requested for blocks up to index {}", target_index);
        
        // Get current height and sync if behind
        if local_height < target_index {
            sync_blocks_simple(local_height, target_index).await?;
        }
    }
});
```

### 4. Rate Limiting

**Purpose:** Prevent sync storm when many orphan blocks are received in quick succession.

**Mechanism:**
- Track last sync time
- Ignore sync triggers within the cooldown period (5 seconds)
- Log when cooldown is active for debugging

## Architecture

```
┌─────────────────────┐
│  Gossip Handler     │
│  (block.rs)         │
└──────────┬──────────┘
           │
           │ Orphan Detected
           │
           ▼
┌─────────────────────┐
│  Sync Trigger TX    │  (Broadcast Channel)
│  (Node)             │
└──────────┬──────────┘
           │
           │ Subscribe
           │
           ▼
┌─────────────────────┐
│  Sync Listener      │
│  (miner.rs task)    │
└──────────┬──────────┘
           │
           │ Request Blocks
           │
           ▼
┌─────────────────────┐
│  Sync Function      │
│  (placeholder)      │
└─────────────────────┘
```

## Current Status

**Implemented:**
- ✅ Orphan detection in gossip handler
- ✅ Sync trigger channel in Node
- ✅ Sync listener task in miner
- ✅ Rate limiting for sync requests
- ✅ Local height checking

**Placeholder (Future Work):**
- 🔄 Active block request from peers
- 🔄 Peer selection (currently uses first bootstrapper)
- 🔄 Block range requests via request-response protocol
- 🔄 Parallel sync from multiple peers

## Benefits

1. **Automatic Recovery:** Nodes automatically catch up when they fall behind
2. **Resilient to Network Issues:** No manual intervention needed
3. **Rate Limited:** Won't overwhelm the network with sync requests
4. **Non-Blocking:** Gossip processing continues while sync is triggered
5. **Observable:** Clear logging shows when and why syncs occur

## Example Logs

```
[INFO] Received block 239 but missing parent block (prev_hash: 000001650b93266d). Orphan block detected - triggering sync!
[INFO] 🔄 Sync triggered for missing blocks up to index 239
[INFO] 🔄 Sync requested for blocks up to index 239
[INFO] Syncing blocks from 232 to 239
[INFO] Sync completed - blocks should be received via gossip
```

## Testing

To test the automatic sync mechanism:

1. Start three nodes
2. Stop the winning node
3. Observe other nodes detect orphans
4. Watch sync triggers fire
5. See nodes attempt to sync missing blocks

## Future Enhancements

1. **Active Sync:** Implement actual block requests using the request-response protocol
2. **Smart Peer Selection:** Choose peers based on connection quality and chain height
3. **Batch Sync:** Request multiple block ranges in parallel
4. **Checkpoints:** Use checkpoints to verify sync integrity
5. **Metrics:** Track sync performance and success rates

