# Miner Block Synchronization Protocol

This document describes the miner block synchronization protocol implemented in `modality-network-node`.

## Overview

The sync protocol allows nodes to query and retrieve persisted miner blocks from other nodes using the request-response protocol over libp2p.

## Architecture

```
┌─────────────────┐                  ┌─────────────────┐
│    Node 1       │                  │    Node 2       │
│  (Has Blocks)   │                  │  (Requesting)   │
│                 │                  │                 │
│  ┌───────────┐  │                  │  ┌───────────┐  │
│  │Datastore  │  │                  │  │Datastore  │  │
│  │MinerBlocks│  │                  │  │(Empty)    │  │
│  └───────────┘  │                  │  └───────────┘  │
│                 │                  │                 │
│  ┌───────────┐  │                  │  ┌───────────┐  │
│  │ Request   │  │◄─────Request─────│  │ Request   │  │
│  │ Response  │  │                  │  │ Response  │  │
│  │ Handler   │  │                  │  │ Client    │  │
│  └───────────┘  │─────Response────►│  └───────────┘  │
│                 │                  │                 │
└─────────────────┘                  └─────────────────┘
```

## Protocol Endpoints

### 1. Get All Canonical Blocks

**Endpoint**: `/data/miner_block/canonical`

**Request**:
```json
{
  "path": "/data/miner_block/canonical",
  "data": null
}
```

**Response**:
```json
{
  "ok": true,
  "data": {
    "blocks": [ /* array of MinerBlock objects */ ],
    "count": 11
  },
  "errors": null
}
```

**Usage**: Retrieve all canonical (non-orphaned) miner blocks sorted by index.

### 2. Get Blocks by Epoch

**Endpoint**: `/data/miner_block/epoch`

**Request**:
```json
{
  "path": "/data/miner_block/epoch",
  "data": {
    "epoch": 0
  }
}
```

**Response**:
```json
{
  "ok": true,
  "data": {
    "epoch": 0,
    "blocks": [ /* array of MinerBlock objects */ ],
    "count": 40
  },
  "errors": null
}
```

**Usage**: Retrieve all canonical blocks for a specific epoch (40 blocks per epoch).

### 3. Get Block Range

**Endpoint**: `/data/miner_block/range`

**Request**:
```json
{
  "path": "/data/miner_block/range",
  "data": {
    "from_index": 3,
    "to_index": 7
  }
}
```

**Response**:
```json
{
  "ok": true,
  "data": {
    "from_index": 3,
    "to_index": 7,
    "blocks": [ /* array of 5 MinerBlock objects */ ],
    "count": 5
  },
  "errors": null
}
```

**Usage**: Retrieve blocks in a specific range (inclusive). Useful for incremental sync.

### 4. Get Specific Block

**Endpoint**: `/data/miner_block/get`

**Request**:
```json
{
  "path": "/data/miner_block/get",
  "data": {
    "hash": "block_hash_123..."
  }
}
```

**Response**:
```json
{
  "ok": true,
  "data": {
    "hash": "block_hash_123...",
    "index": 5,
    "epoch": 0,
    /* ... full MinerBlock object ... */
  },
  "errors": null
}
```

**Usage**: Retrieve a single block by its hash.

## MinerBlock Model

Each synced block contains:

```rust
pub struct MinerBlock {
    // Block identification
    pub hash: String,
    pub index: u64,
    pub epoch: u64,
    
    // Header data
    pub timestamp: i64,
    pub previous_hash: String,
    pub data_hash: String,
    pub nonce: String,
    pub difficulty: String,
    
    // Mining data
    pub nominated_peer_id: String,
    pub miner_number: u64,
    
    // Status
    pub is_canonical: bool,
    pub is_orphaned: bool,
    
    // Metadata
    pub seen_at: Option<i64>,
    pub orphaned_at: Option<i64>,
    pub orphan_reason: Option<String>,
    pub competing_hash: Option<String>,
}
```

## Implementation Details

### Request Routing

Requests are routed in `src/reqres/mod.rs`:

```rust
match path.as_str() {
    "/data/miner_block/get" => {
        reqres_data::miner_block::get::handler(...)
    }
    "/data/miner_block/canonical" => {
        reqres_data::miner_block::list_canonical::handler(...)
    }
    "/data/miner_block/epoch" => {
        reqres_data::miner_block::by_epoch::handler(...)
    }
    "/data/miner_block/range" => {
        reqres_data::miner_block::range::handler(...)
    }
    // ...
}
```

### Handlers

Each handler is implemented in `src/reqres/data/miner_block/`:

- `get.rs` - Get block by hash
- `list_canonical.rs` - List all canonical blocks
- `by_epoch.rs` - Get blocks for an epoch
- `range.rs` - Get block range

## Usage Example

See `examples/miner_block_sync.rs` for a complete working example.

### Running the Example

```bash
cargo run --package modality-network-node --example miner_block_sync
```

### Example Output

```
=== Miner Block Sync Example ===

📦 Setting up datastores...
  ✓ Datastores created

⛏  Creating miner blocks in Node 1's datastore...
  ✓ Genesis block: hash=block_hash_000
  ✓ Block 1: hash=block_hash_001, peer=QmMiner01
  ...
  ✓ Total blocks created: 11 (0-10)

🌐 Creating network nodes...
  ✓ Node 1:
    Peer ID: 12D3KooW...
    Listening: /ip4/0.0.0.0/tcp/0
  ✓ Node 2:
    Peer ID: 12D3KooW...
    Listening: /ip4/0.0.0.0/tcp/0

🔗 Connecting Node 2 to Node 1...
  ✓ Nodes connected

🔄 Syncing miner blocks from Node 1 to Node 2...
  ✓ Received 11 blocks from Node 1

📊 Synced Blocks:
  Block  0: epoch=0, peer=QmMiner00, hash=block_hash_000
  Block  1: epoch=0, peer=QmMiner01, hash=block_hash_001
  ...
  Block 10: epoch=0, peer=QmMiner00, hash=block_hash_010

✅ Sync completed successfully!
```

## Sync Strategies

### Full Sync

Request all canonical blocks:

```rust
let request = Request {
    path: "/data/miner_block/canonical".to_string(),
    data: None,
};

let response = swarm.send_request(&peer_id, request);
```

### Incremental Sync

Sync blocks in batches:

```rust
let current_height = local_blockchain.height();
let batch_size = 100;

for batch_start in (current_height..remote_height).step_by(batch_size) {
    let request = Request {
        path: "/data/miner_block/range".to_string(),
        data: Some(serde_json::json!({
            "from_index": batch_start,
            "to_index": batch_start + batch_size,
        })),
    };
    
    // Process batch...
}
```

### Epoch-Based Sync

Sync by epoch for better organization:

```rust
for epoch in 0..current_epoch {
    let request = Request {
        path: "/data/miner_block/epoch".to_string(),
        data: Some(serde_json::json!({ "epoch": epoch })),
    };
    
    // Process epoch blocks...
}
```

## Transport Configuration

The example supports both TCP and WebSocket transports:

### TCP (Default)

```rust
let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
```

### WebSocket

```rust
let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/10001/ws".parse()?;
```

**Note**: TCP is recommended for local testing due to simpler configuration.

## Integration with Mining

The sync protocol integrates with `modal-miner`:

1. **Mining Node**: Mines blocks with persistence
   ```rust
   chain.mine_block_with_persistence(peer_id, number).await?;
   ```

2. **Datastore**: Blocks automatically saved to `MinerBlock` model

3. **Sync Node**: Queries persisted blocks via request-response

4. **Verification**: Can verify and add to local blockchain

## Security Considerations

1. **Block Validation**: Always validate received blocks before adding to chain
2. **Rate Limiting**: Implement rate limiting for sync requests
3. **Peer Trust**: Only sync from trusted or verified peers
4. **Data Integrity**: Verify block hashes and signatures

## Future Enhancements

- [ ] Chunked block transfer for large datasets
- [ ] Delta sync (only missing blocks)
- [ ] Block header-first sync
- [ ] Compressed block transfer
- [ ] Sync progress tracking
- [ ] Automatic retry on failure
- [ ] Peer discovery and selection
- [ ] Chain verification during sync

## Related Documentation

- [Miner Block Model](../../modal-datastore/docs/MINER_BLOCK.md)
- [Mining Package](../../modal-miner/README.md)
- [Request-Response Protocol](./REQRES.md)

