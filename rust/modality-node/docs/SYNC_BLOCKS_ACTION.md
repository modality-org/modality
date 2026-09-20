# Sync Blocks Action

The `sync_blocks` action provides a high-level API for syncing miner blocks from a remote node with optional persistence.

## Location

`modality_network_node::actions::sync_blocks`

## Usage

```rust
use modality_network_node::actions::sync_blocks;
use modality_network_node::node::Node;

// Sync blocks without persistence
let result = sync_blocks::run(
    &mut node,
    "/ip4/127.0.0.1/tcp/10001/p2p/12D3KooW...".to_string(),
    "/data/miner_block/canonical".to_string(),
    "".to_string(),
    false, // persist = false
).await?;

// Sync blocks WITH persistence
let result = sync_blocks::run(
    &mut node,
    "/ip4/127.0.0.1/tcp/10001/p2p/12D3KooW...".to_string(),
    "/data/miner_block/canonical".to_string(),
    "".to_string(),
    true, // persist = true
).await?;

println!("Blocks received: {}", result.response.data.is_some());
if let Some(count) = result.persisted_count {
    println!("Blocks persisted: {}", count);
}
if let Some(skipped) = result.skipped_count {
    println!("Blocks skipped: {}", skipped);
}
```

## API

### Function Signature

```rust
pub async fn run(
    node: &mut Node,
    target: String,
    path: String,
    data: String,
    persist: bool,
) -> Result<SyncResult>
```

### Parameters

- `node: &mut Node` - The network node to use for syncing
- `target: String` - Target multiaddress (must end with `/p2p/<PEER_ID>`)
- `path: String` - Request path (e.g., `/data/miner_block/canonical`)
- `data: String` - Request data as JSON string (empty string for no data)
- `persist: bool` - Whether to persist blocks to the datastore

### Return Type

```rust
pub struct SyncResult {
    pub response: reqres::Response,
    pub persisted_count: Option<usize>,
    pub skipped_count: Option<usize>,
}
```

- `response` - The raw response from the remote node
- `persisted_count` - Number of blocks saved (only if `persist = true`)
- `skipped_count` - Number of duplicate blocks skipped (only if `persist = true`)

## Examples

### Sync All Canonical Blocks

```rust
let result = sync_blocks::run(
    &mut node,
    target_addr,
    "/data/miner_block/canonical".to_string(),
    "".to_string(),
    true,
).await?;

if result.response.ok {
    println!("✓ Sync successful");
    if let Some(count) = result.persisted_count {
        println!("  Persisted: {}", count);
    }
}
```

### Sync Blocks by Epoch

```rust
let epoch_data = serde_json::json!({ "epoch": 5 }).to_string();

let result = sync_blocks::run(
    &mut node,
    target_addr,
    "/data/miner_block/epoch".to_string(),
    epoch_data,
    true,
).await?;
```

### Sync Block Range

```rust
let range_data = serde_json::json!({
    "from_index": 10,
    "to_index": 20
}).to_string();

let result = sync_blocks::run(
    &mut node,
    target_addr,
    "/data/miner_block/range".to_string(),
    range_data,
    true,
).await?;
```

### Query Only (No Persistence)

```rust
// Just fetch blocks, don't save them
let result = sync_blocks::run(
    &mut node,
    target_addr,
    "/data/miner_block/canonical".to_string(),
    "".to_string(),
    false, // persist = false
).await?;

// Process blocks from result.response.data
if let Some(data) = result.response.data {
    // ... process blocks ...
}
```

## Persistence Logic

When `persist = true`, the action:

1. **Fetches** blocks from the remote node
2. **Validates** the response
3. **Checks** each block against the local datastore (by hash)
4. **Skips** blocks that already exist
5. **Saves** new blocks to the datastore
6. **Logs** progress at INFO and DEBUG levels
7. **Returns** counts of saved and skipped blocks

### Idempotency

The persistence is idempotent - running the sync multiple times with the same blocks is safe:

```rust
// First run: saves all blocks
let result1 = sync_blocks::run(&mut node, addr, path, data, true).await?;
// result1.persisted_count = Some(100)
// result1.skipped_count = Some(0)

// Second run: skips all existing blocks
let result2 = sync_blocks::run(&mut node, addr, path, data, true).await?;
// result2.persisted_count = Some(0)
// result2.skipped_count = Some(100)
```

## Error Handling

Errors can occur at several stages:

```rust
match sync_blocks::run(&mut node, addr, path, data, true).await {
    Ok(result) => {
        if !result.response.ok {
            eprintln!("Remote node returned error: {:?}", result.response.errors);
        }
    }
    Err(e) => {
        eprintln!("Sync failed: {}", e);
        // Could be:
        // - Invalid multiaddress
        // - Connection failure
        // - Deserialization error
        // - Datastore error
    }
}
```

## Logging

The action produces structured logs:

```
[INFO] ✓ Persisted 45 blocks to datastore
[INFO] Skipped 0 blocks (already in datastore)
[DEBUG] Saved block block_hash_001 to datastore
[DEBUG] Block block_hash_002 already exists, skipping
```

## Integration with CLI

The CLI command uses this action:

```rust
// In modality/src/cmds/net/mining/sync.rs
let sync_result = actions::sync_blocks::run(
    &mut node,
    target,
    path,
    data_str,
    opts.persist, // Pass --persist flag through
).await?;
```

This design keeps business logic in the library and UI logic in the CLI.

## Related

- [Miner Block Sync Protocol](MINER_BLOCK_SYNC.md) - Protocol specification
- [CLI Documentation](../../modality/docs/CLI_MINING_SYNC.md) - CLI usage guide
- [Request Action](../src/actions/request.rs) - Lower-level request API

