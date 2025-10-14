# `modality net storage` - Network Datastore Inspection

Inspect a Modality network datastore and display miner block statistics.

## Usage

```bash
modality net storage --config <NODE_CONFIG> [OPTIONS]
```

## Arguments

| Option | Description | Required | Default |
|--------|-------------|----------|---------|
| `--config <PATH>` | Path to node configuration file | Yes | - |
| `--detailed` | Show detailed list of all blocks | No | `false` |
| `--epoch <EPOCH>` | Filter by specific epoch | No | All epochs |
| `--limit <N>` | Limit number of blocks to display in detailed view | No | `10` |

## Examples

### Basic Inspection

Display summary statistics for all miner blocks:

```bash
modality net storage --config ./configs/node1.json
```

**Output:**
```
📁 Opening datastore at: "/path/to/storage/node1"

🔍 Querying all canonical miner blocks...

📊 Miner Block Statistics
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Total Blocks: 120
  Block Range: 0 → 119
  Epochs: 3
  Unique Miners: 5
  Time Range: 2023-11-14 22:13:20 → 2023-11-15 00:12:20
  Duration: 0 days, 1 hours

📈 Blocks per Epoch:
  Epoch 0: 40 blocks (difficulty: 1000)
  Epoch 1: 40 blocks (difficulty: 1100)
  Epoch 2: 40 blocks (difficulty: 1200)

👷 Top Miners:
  QmMiner1...23def456: 24 blocks (20.0%)
  QmMiner4...01vwx234: 24 blocks (20.0%)
  QmMiner3...45pqr678: 24 blocks (20.0%)
  QmMiner5...67bcd890: 24 blocks (20.0%)
  QmMiner2...89jkl012: 24 blocks (20.0%)

✅ Storage inspection complete!
```

### Filter by Epoch

Show statistics for a specific epoch:

```bash
modality net storage --config ./configs/node1.json --epoch 1
```

**Output:**
```
🔍 Querying blocks for epoch 1...

📊 Miner Block Statistics
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Total Blocks: 40
  Block Range: 40 → 79
  Epochs: 1
  Unique Miners: 5
  Time Range: 2023-11-14 22:53:20 → 2023-11-14 23:32:20
  Duration: 0 days, 0 hours

📈 Blocks per Epoch:
  Epoch 1: 40 blocks (difficulty: 1100)

👷 Top Miners:
  QmMiner5...67bcd890: 8 blocks (20.0%)
  QmMiner4...01vwx234: 8 blocks (20.0%)
  QmMiner1...23def456: 8 blocks (20.0%)
  QmMiner2...89jkl012: 8 blocks (20.0%)
  QmMiner3...45pqr678: 8 blocks (20.0%)

✅ Storage inspection complete!
```

### Detailed View

Show detailed block information:

```bash
modality net storage --config ./configs/node1.json --detailed --limit 5
```

**Output:**
```
📊 Miner Block Statistics
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Total Blocks: 120
  Block Range: 0 → 119
  Epochs: 3
  Unique Miners: 5
  ...

📋 Block List:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

(Showing first 5 of 120 blocks)

  Block #0
    Hash: block_hash_000
    Epoch: 0
    Miner: QmMiner1...23def456
    Difficulty: 1000
    Nonce: 10000

  Block #1
    Hash: block_hash_001
    Epoch: 0
    Miner: QmMiner2...89jkl012
    Difficulty: 1000
    Nonce: 10001

  ...
```

### Combine Filters

Show detailed blocks from a specific epoch:

```bash
modality net storage --config ./configs/node1.json --epoch 2 --detailed --limit 10
```

## Statistics Displayed

### Summary Statistics

- **Total Blocks**: Count of all canonical (non-orphaned) blocks
- **Block Range**: Lowest and highest block indices
- **Epochs**: Number of unique epochs
- **Unique Miners**: Number of distinct peer IDs that mined blocks
- **Time Range**: Timestamp range of blocks
- **Duration**: Time span covered by the blocks

### Epoch Breakdown

For each epoch:
- Block count
- Difficulty level

### Top Miners

Shows up to 10 miners ordered by block count:
- Peer ID (truncated for readability)
- Number of blocks mined
- Percentage of total blocks

### Detailed Block Information

When using `--detailed`:
- Block index
- Block hash (truncated)
- Epoch
- Miner peer ID (truncated)
- Difficulty
- Nonce value

## Use Cases

### 1. **Verify Block Synchronization**

After syncing blocks from another node, verify they were persisted:

```bash
# Sync blocks
modality net mining sync --config ./node2.json --target /ip4/.../p2p/... --mode all --persist

# Verify
modality net storage --config ./node2.json
```

### 2. **Monitor Blockchain Growth**

Check current blockchain state:

```bash
modality net storage --config ./node.json
```

Look at the "Total Blocks" and "Epochs" values to see growth over time.

### 3. **Analyze Mining Distribution**

See which miners are producing the most blocks:

```bash
modality net storage --config ./node.json
```

Check the "Top Miners" section for distribution analysis.

### 4. **Investigate Specific Epochs**

When debugging epoch-related issues:

```bash
# Check epoch 5 specifically
modality net storage --config ./node.json --epoch 5 --detailed
```

### 5. **Audit Block Data**

Inspect detailed block information for auditing:

```bash
# Show first 50 blocks in detail
modality net storage --config ./node.json --detailed --limit 50
```

### 6. **Compare Node States**

Compare datastores across different nodes:

```bash
# Node 1
modality net storage --config ./node1.json > node1_stats.txt

# Node 2
modality net storage --config ./node2.json > node2_stats.txt

# Compare
diff node1_stats.txt node2_stats.txt
```

## Configuration File

The command reads the node configuration to determine the datastore location:

```json
{
  "passfile_path": "path/to/passfile",
  "storage_path": "./storage/node1",
  "listeners": ["/ip4/0.0.0.0/tcp/10001/ws"]
}
```

The `storage_path` field specifies where the RocksDB datastore is located.

## Behavior

### Datastore Access

- Opens the datastore in **read-only mode** (uses `create_in_directory` which is safe for existing datastores)
- Does not modify any data
- Safe to run while the node is running

### Block Filtering

- Only shows **canonical blocks** (non-orphaned)
- Orphaned blocks are automatically excluded from statistics
- Epoch filtering applies before statistics calculation

### Performance

- Loads all blocks into memory for statistics
- For large blockchains (>10,000 blocks), this may take a few seconds
- The `--limit` flag only affects the detailed view, not the query

## Error Handling

### "Storage path does not exist"

```
Error: Storage path does not exist: "/path/to/storage"
```

**Solution:** Verify the storage path in the config file exists.

### "Config does not specify a storage_path"

```
Error: Config does not specify a storage_path
```

**Solution:** Add `storage_path` to your node config file.

### "No miner blocks found in datastore"

```
⚠️  No miner blocks found in datastore
```

**Cause:** The datastore is empty or contains no miner blocks yet.

**Solution:** 
- Ensure blocks have been mined or synced
- Check if you're using the correct config file
- Verify the storage path points to the right location

### "Failed to open datastore"

```
Error: Failed to open datastore
```

**Possible causes:**
- Datastore is corrupted
- Insufficient permissions
- Another process has an exclusive lock

**Solution:** Check filesystem permissions and datastore integrity.

## Integration Examples

### Shell Script for Monitoring

```bash
#!/bin/bash
# monitor_blockchain.sh

CONFIG="./node.json"

while true; do
  clear
  echo "=== Blockchain Status ==="
  date
  echo ""
  modality net storage --config "$CONFIG"
  echo ""
  echo "Refreshing in 60 seconds..."
  sleep 60
done
```

### Export Statistics to JSON

```bash
# Get block count
TOTAL_BLOCKS=$(modality net storage --config ./node.json | grep "Total Blocks:" | awk '{print $3}')
echo "Total blocks: $TOTAL_BLOCKS"

# Get epoch count
EPOCHS=$(modality net storage --config ./node.json | grep "Epochs:" | awk '{print $2}')
echo "Epochs: $EPOCHS"
```

### Compare Before/After Sync

```bash
# Before sync
echo "Before sync:"
modality net storage --config ./node.json

# Perform sync
modality net mining sync --config ./node.json --target ... --persist

# After sync
echo -e "\nAfter sync:"
modality net storage --config ./node.json
```

## Implementation Details

- **Query Method**: Uses `MinerBlock::find_all_canonical` or `MinerBlock::find_canonical_by_epoch`
- **Storage Backend**: RocksDB via `modality-network-datastore`
- **Model**: Uses the `MinerBlock` model with `Model` trait
- **Async**: Fully asynchronous using `tokio`

## Related Commands

- [`modality net mining sync`](./CLI_MINING_SYNC.md) - Sync miner blocks from a node
- [`modality net run-node`](./CLI_RUN_NODE.md) - Run a network node

## See Also

- [Miner Block Model](../../modality-network-datastore/docs/MINER_BLOCK.md)
- [Network Datastore](../../modality-network-datastore/README.md)
- [Mining Package](../../modality-network-mining/README.md)

