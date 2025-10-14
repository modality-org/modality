# Example: Sync Miner Blocks Between Nodes

This example demonstrates how to synchronize miner blocks between Modality network nodes using the CLI.

## Overview

- **Node 1**: Source node with persisted miner blocks (runs on port 10201)
- **Node 2**: Destination node that will sync blocks from Node 1 (runs on port 10202)

## Prerequisites

1. Build the `modality` CLI:
   ```bash
   cd ../../../rust
   cargo build --package modality --release
   ```

2. Node1 will automatically be set up with 3 epochs of test blocks (120 blocks) when you first run it.
   - The setup uses the `create_test_blocks` example from `modality-network-datastore`
   - Blocks are persisted to `./tmp/storage/node1`
   - Setup runs automatically if the datastore is empty

## Running the Example

### Step 1: Start Node 1

In one terminal:

```bash
./01-run-node1.sh
```

This will:
1. **Automatically create 3 epochs of test blocks** (120 blocks total) if the datastore is empty
2. Start node1 with the miner blocks

The setup creates:
- **Epoch 0**: 40 blocks (difficulty: 1000)
- **Epoch 1**: 40 blocks (difficulty: 1100)  
- **Epoch 2**: 40 blocks (difficulty: 1200)

**Note**: Node1's Peer ID is `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`

### Step 2: (Optional) Manually Set Up Blocks

If you want to set up the blocks without starting the node:

```bash
./00-setup-node1-blocks.sh
```

You can verify the blocks were created by inspecting the datastore:

```bash
./07-inspect-storage.sh
```

Or check the raw files:

```bash
ls -lh ./tmp/storage/node1/
```

### Step 3: Sync Blocks to Node 2

In another terminal, sync blocks using one of these methods:

#### Sync All Canonical Blocks

```bash
./03-sync-all-blocks.sh
```

This syncs all canonical (non-orphaned) blocks from node1 to node2 and persists them.

**Output:**
```
✅ Sync completed successfully!
   Duration: 234ms
   Blocks received: 120
   Blocks persisted: 120

📊 Block Summary:
   Total blocks: 120
   First block: 0
   Last block: 119
   Epochs: 0-2
   ...
```

#### Sync Specific Epoch

```bash
./04-sync-epoch.sh 0
```

Syncs all blocks from epoch 0 (40 blocks per epoch).

#### Sync Block Range

```bash
./05-sync-range.sh 10 20
```

Syncs blocks with indices 10 through 20 (inclusive).

## Script Reference

| Script | Description |
|--------|-------------|
| `00-setup-node1-blocks.sh` | Create 3 epochs of test blocks (auto-runs) |
| `01-run-node1.sh` | Start node1 (source with miner blocks) |
| `02-create-test-blocks.sh` | Info on creating test blocks |
| `03-sync-all-blocks.sh` | Sync all canonical blocks with persistence |
| `04-sync-epoch.sh [EPOCH]` | Sync blocks from specific epoch |
| `05-sync-range.sh [FROM] [TO]` | Sync block range |
| `06-view-blocks-json.sh` | View blocks in JSON format (no persistence) |
| `07-inspect-storage.sh` | Inspect node1's datastore and show statistics |

## Sync Modes

### All / Canonical

Syncs all canonical (non-orphaned) blocks:

```bash
modality net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/12D3KooW... \
  --mode all \
  --persist
```

### Epoch

Syncs all blocks from a specific epoch (useful for epoch-based verification):

```bash
modality net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/12D3KooW... \
  --mode epoch \
  --epoch 2 \
  --persist
```

### Range

Syncs blocks in a specific index range (useful for incremental sync):

```bash
modality net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/12D3KooW... \
  --mode range \
  --from-index 100 \
  --to-index 200 \
  --persist
```

## Persistence

### With Persistence (`--persist`)

Saves blocks to node2's datastore. Blocks are:
- **Idempotent**: Can run sync multiple times safely
- **Duplicate-aware**: Skips blocks that already exist
- **Validated**: Checks block hashes before saving

### Without Persistence

Omit `--persist` to just view blocks without saving:

```bash
modality net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/12D3KooW... \
  --format json
```

Useful for:
- Inspecting block data
- Piping to `jq` for filtering
- Testing connectivity

## Output Formats

### Summary (Default)

Human-readable summary with block statistics:

```bash
--format summary  # default
```

### JSON

Machine-readable JSON for scripting:

```bash
--format json | jq '.blocks[0]'
```

## Advanced Examples

### Incremental Sync

Sync in batches for large blockchains:

```bash
for i in {0..100..10}; do
  ./05-sync-range.sh $i $((i+9))
  sleep 1
done
```

### Find Blocks by Miner

```bash
./06-view-blocks-json.sh | jq '.blocks[] | select(.nominated_peer_id | startswith("QmMiner1"))'
```

### Count Blocks per Epoch

```bash
./06-view-blocks-json.sh | jq '[.blocks[] | .epoch] | group_by(.) | map({epoch: .[0], count: length})'
```

### Inspect Storage Statistics

```bash
# Summary stats
./07-inspect-storage.sh

# Detailed view
modality net storage --config ./configs/node1.json --detailed --limit 20

# Filter by epoch
modality net storage --config ./configs/node1.json --epoch 1
```

## Verification

After syncing, verify blocks were persisted to node2:

### 1. Inspect with Storage Command

```bash
modality net storage --config ./configs/node2.json
```

This shows:
- Total blocks
- Epoch breakdown
- Miner distribution
- Block range
- Time range

### 2. Check Datastore Directory

```bash
ls -lh ./tmp/storage/node2/
```

### 3. Re-run Sync

Re-running the sync should show no new persisted blocks:

```bash
./03-sync-all-blocks.sh
```

Output should show "Blocks persisted: 0" and "Skipped: 120"

## Troubleshooting

### "Connection timeout"

- Ensure node1 is running (`./01-run-node1.sh`)
- Check node1's Peer ID matches in scripts
- Verify port 10201 is not blocked

### "No blocks found"

- The automatic setup might have failed
- Manually run `./00-setup-node1-blocks.sh`
- Check node1's storage path has data: `ls ./tmp/storage/node1/`
- Ensure the Rust build completes successfully

### "Failed to save block"

- Check node2's storage path is writable
- Ensure node2's datastore isn't corrupted
- Check disk space

## Architecture

```
┌─────────────────┐                    ┌─────────────────┐
│    Node 1       │                    │    Node 2       │
│  (Source)       │                    │  (Destination)  │
│                 │                    │                 │
│  Datastore:     │                    │  Datastore:     │
│  [45 blocks]    │◄───── Sync ────────│  [Empty]        │
│                 │       Request      │                 │
│                 │                    │                 │
│  Port: 10201    │──── Response ─────►│  Port: 10202    │
└─────────────────┘    (Blocks JSON)   └─────────────────┘
                                              │
                                              ▼
                                        [Persist with
                                         --persist flag]
```

## Related Documentation

- [CLI Mining Sync](../../../rust/modality/docs/CLI_MINING_SYNC.md) - Complete sync CLI reference
- [CLI Storage Inspection](../../../rust/modality/docs/CLI_STORAGE.md) - Storage inspection CLI reference
- [Miner Block Sync Protocol](../../../rust/modality-network-node/docs/MINER_BLOCK_SYNC.md) - Protocol specification
- [Sync Blocks Action](../../../rust/modality-network-node/docs/SYNC_BLOCKS_ACTION.md) - Programmatic API

## Notes

- Blocks are stored in the datastore specified in node config's `storage_path`
- The sync operation is atomic per block (either all succeed or none persist)
- Orphaned blocks are not synced (only canonical blocks)
- Persistence uses the `MinerBlock` model in the datastore

