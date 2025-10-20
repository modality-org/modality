# CLI: Mining Sync Command

The `modality net mining sync` command allows you to synchronize miner blocks from a remote node directly from the command line.

## Usage

```bash
modality net mining sync --config <CONFIG> --target <TARGET> [OPTIONS]
```

## Required Arguments

- `--config <CONFIG>`: Path to your node configuration file (JSON)
- `--target <TARGET>`: Target node multiaddress to sync from
  - Format: `/ip4/<IP>/tcp/<PORT>/p2p/<PEER_ID>`
  - Example: `/ip4/127.0.0.1/tcp/10001/p2p/12D3KooWHdjCtuKYeP1TVL5C6m3XWYyon3raEmbEVyyNN2ZA3wcb`

## Optional Arguments

- `--mode <MODE>`: Sync mode (default: `all`)
  - `all` or `canonical`: Sync all canonical blocks
  - `epoch`: Sync blocks from a specific epoch
  - `range`: Sync blocks in a specific index range

- `--epoch <EPOCH>`: Epoch number (required for `epoch` mode)

- `--from-index <FROM_INDEX>`: Start index (required for `range` mode)

- `--to-index <TO_INDEX>`: End index (required for `range` mode)

- `--format <FORMAT>`: Output format (default: `summary`)
  - `summary`: Human-readable summary
  - `json`: Raw JSON response

- `--persist`: Persist synced blocks to local datastore
  - Saves blocks to your node's local database
  - Skips blocks that already exist (idempotent)
  - Useful for bootstrapping or catching up

## Examples

### Sync All Blocks (View Only)

```bash
modality net mining sync \
  --config ./node_config.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW...
```

### Sync All Blocks (With Persistence)

```bash
modality net mining sync \
  --config ./node_config.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW... \
  --persist
```

**Output (with --persist):**
```
✅ Sync completed successfully!
   Duration: 342ms
   Blocks received: 45
   Blocks persisted: 45

📊 Block Summary:
   First block: 0
   Last block: 44

   First 5 blocks:
   - Block   0: epoch=0, peer=QmMiner1abc123, hash=block_hash_000
   - Block   1: epoch=0, peer=QmMiner2def456, hash=block_hash_001
   - Block   2: epoch=0, peer=QmMiner3ghi789, hash=block_hash_002
   - Block   3: epoch=0, peer=QmMiner1abc123, hash=block_hash_003
   - Block   4: epoch=0, peer=QmMiner2def456, hash=block_hash_004
   ... (40 more blocks)
```

### Sync Specific Epoch

```bash
modality net mining sync \
  --config ./node_config.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW... \
  --mode epoch \
  --epoch 2
```

**Output:**
```
✅ Sync completed successfully!
   Duration: 156ms
   Blocks received: 40
   Epoch: 2

📊 Block Summary:
   First block: 80
   Last block: 119

   First 5 blocks:
   - Block  80: epoch=2, peer=QmMiner4jkl012, hash=block_hash_080
   - Block  81: epoch=2, peer=QmMiner5mno345, hash=block_hash_081
   ...
```

### Sync Block Range

```bash
modality net mining sync \
  --config ./node_config.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW... \
  --mode range \
  --from-index 10 \
  --to-index 20
```

**Output:**
```
✅ Sync completed successfully!
   Duration: 89ms
   Blocks received: 11
   Range: 10 to 20

📊 Block Summary:
   First block: 10
   Last block: 20

   First 5 blocks:
   - Block  10: epoch=0, peer=QmMiner3ghi789, hash=block_hash_010
   - Block  11: epoch=0, peer=QmMiner4jkl012, hash=block_hash_011
   - Block  12: epoch=0, peer=QmMiner5mno345, hash=block_hash_012
   - Block  13: epoch=0, peer=QmMiner1abc123, hash=block_hash_013
   - Block  14: epoch=0, peer=QmMiner2def456, hash=block_hash_014
   ... (6 more blocks)
```

### JSON Output

```bash
modality net mining sync \
  --config ./node_config.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW... \
  --format json
```

**Output:**
```json
{
  "blocks": [
    {
      "hash": "block_hash_000",
      "index": 0,
      "epoch": 0,
      "timestamp": 1234567890,
      "previous_hash": "0",
      "data_hash": "data_hash_000",
      "nonce": "10000",
      "difficulty": "1000",
      "nominated_peer_id": "QmMiner1abc123",
      "miner_number": 1000,
      "is_canonical": true,
      "is_orphaned": false,
      "seen_at": 1760411414,
      "orphaned_at": null,
      "orphan_reason": null,
      "height_at_time": 0,
      "competing_hash": null
    },
    ...
  ],
  "count": 45
}
```

## Persistence Behavior

The `--persist` flag triggers persistence logic in the `modality-network-node` package. When enabled:

1. **Idempotent**: Syncing the same blocks multiple times is safe. Duplicate blocks are automatically skipped.

2. **Conflict Detection**: The system checks if a block with the same hash already exists before saving.

3. **Logging**: Progress is logged:
   - `INFO` level: Shows count of blocks persisted and skipped
   - `DEBUG` level: Shows individual blocks being saved or skipped

4. **Error Handling**: If a block fails to save, the sync stops and reports the error.

5. **Datastore Location**: Blocks are saved to the datastore path specified in your node configuration file.

6. **Programmatic Access**: The persistence logic is available in `modality_network_node::actions::sync_blocks` for programmatic use.

### Example Log Output

```
[2025-10-14T03:15:22Z INFO  modality::cmds::net::mining::sync] ✓ Persisted 45 blocks to local datastore
[2025-10-14T03:15:22Z INFO  modality::cmds::net::mining::sync] Skipped 0 blocks (already in datastore)
```

When re-running with existing blocks:
```
[2025-10-14T03:15:30Z INFO  modality::cmds::net::mining::sync] ✓ Persisted 0 blocks to local datastore
[2025-10-14T03:15:30Z INFO  modality::cmds::net::mining::sync] Skipped 45 blocks (already in datastore)
```

## Node Configuration File

The `--config` file is a JSON file with your node configuration:

```json
{
  "id": "node1",
  "passfile_path": "./node1.mod_passfile",
  "storage_path": "./node1_data",
  "listeners": [
    "/ip4/0.0.0.0/tcp/10001"
  ],
  "bootstrappers": []
}
```

## Use Cases

### 1. Bootstrap a New Node

When setting up a new node, sync and persist the entire blockchain:

```bash
modality net mining sync \
  --config ./new_node.json \
  --target /ip4/seed.modality.network/tcp/10001/p2p/12D3KooW... \
  --mode all \
  --persist
```

### 2. Catch Up After Downtime

If your node was offline, sync and persist missing blocks:

```bash
# Get current local height (e.g., 150)
LOCAL_HEIGHT=150

# Sync blocks from current height to remote tip
modality net mining sync \
  --config ./node.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW... \
  --mode range \
  --from-index $LOCAL_HEIGHT \
  --to-index 200 \
  --persist
```

### 3. Verify Epoch Data

Verify blocks for a specific epoch:

```bash
modality net mining sync \
  --config ./node.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW... \
  --mode epoch \
  --epoch 5 \
  --format json > epoch5_blocks.json
```

### 4. Export Blockchain Data

Export all blocks for analysis:

```bash
modality net mining sync \
  --config ./node.json \
  --target /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW... \
  --format json > blockchain_export.json
```

## Error Handling

### Connection Failed

```
Error: Failed to connect to peer
```

**Solution**: Verify the target multiaddress is correct and the remote node is running.

### Authentication Failed

```
Error: Sync failed: Some(Object {"error": String("Authentication required")})
```

**Solution**: Ensure your node configuration has proper credentials.

### Invalid Mode

```
Error: Invalid mode: invalid. Use 'all', 'epoch', or 'range'
```

**Solution**: Use one of the supported sync modes.

### Missing Parameters

```
Error: --epoch is required for epoch mode
```

**Solution**: Provide the required parameter for the chosen mode.

## Performance Tips

1. **Use Range Mode for Large Syncs**: Instead of syncing all blocks at once, use range mode with batches:
   ```bash
   for i in {0..10..100}; do
     modality net mining sync --mode range --from-index $i --to-index $((i+99)) --persist ...
   done
   ```
   
   **Note**: The persistence layer automatically skips duplicate blocks, so re-running sync with `--persist` is safe and idempotent.

2. **Local Network**: For faster syncs, use the local network address of the target node.

3. **JSON Output for Processing**: Use `--format json` and pipe to `jq` for processing:
   ```bash
   modality net mining sync --format json ... | jq '.blocks[] | select(.epoch == 0)'
   ```

## Integration with Scripts

### Bash Script Example

```bash
#!/bin/bash

CONFIG="./node.json"
TARGET="/ip4/127.0.0.1/tcp/10001/p2p/12D3KooW..."

# Sync all blocks
echo "Syncing blockchain..."
modality net mining sync \
  --config "$CONFIG" \
  --target "$TARGET" \
  --format json > blockchain.json

# Count blocks
BLOCK_COUNT=$(jq '.blocks | length' blockchain.json)
echo "Synced $BLOCK_COUNT blocks"

# Find blocks by specific miner
jq '.blocks[] | select(.nominated_peer_id == "QmMiner1...")' blockchain.json
```

## Related Commands

- `modality node run`: Start a Modality network node
- `modality node ping`: Test connection to a node
- See [examples/miner_block_sync.rs](../../modality-network-node/examples/miner_block_sync.rs) for programmatic sync

## Protocol Details

The sync command uses the request-response protocol endpoints:

- `GET /data/miner_block/canonical` - All blocks
- `GET /data/miner_block/epoch` - Epoch blocks
- `GET /data/miner_block/range` - Block range

See [MINER_BLOCK_SYNC.md](../../modality-network-node/docs/MINER_BLOCK_SYNC.md) for protocol details.

