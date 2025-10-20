# Example: Mining with Difficulty Adjustment

This example demonstrates mining blocks with automatic difficulty adjustment in Modality.

## Overview

- **Single Mining Node**: Continuously mines blocks with difficulty adjustment every epoch
- **Epoch Length**: 40 blocks per epoch
- **Difficulty Adjustment**: Based on mining speed (target: 60 seconds per block)
- **Persistence**: All blocks saved to local datastore

## Prerequisites

1. Build the `modality` CLI:
   ```bash
   cd ../../../rust
   cargo build --package modality --release
   ```

## Running the Example

### Step 1: Mine Blocks

```bash
./01-mine-blocks.sh
```

This will:
- Start a miner node on port 10301
- Mine blocks continuously
- Adjust difficulty after each epoch (every 40 blocks)
- Save all blocks to `./tmp/storage/miner`

**Expected Behavior:**
- **Epoch 0 (blocks 0-39)**: Difficulty stays at 1000 (initial)
- **Epoch 1 (blocks 40-79)**: Difficulty adjusts based on epoch 0 mining speed
- **Epoch 2 (blocks 80-119)**: Difficulty adjusts based on epoch 1 mining speed
- And so on...

**Difficulty Adjustment Rules:**
- **Blocks mined too fast** (< 90% of target time) → Difficulty increases
- **Blocks mined too slow** (> 110% of target time) → Difficulty decreases
- **Within target range** → Difficulty stays the same

### Step 2: View Status Page

While mining is running, view the real-time status dashboard:

```bash
./04-view-status-page.sh
```

Or visit directly in your browser: **http://localhost:8080**

The status page shows:
- **Connected Peers**: Number of active peer connections
- **Miner Blocks**: Total canonical blocks mined
- **Current Difficulty**: Mining difficulty from latest block
- **Current Epoch**: Current epoch number
- **Recent Blocks Table**: Last 80 blocks with index, epoch, hash, and nominee

The page auto-refreshes every 10 seconds with the latest data from the running node.

### Step 3: Inspect Blocks

While mining is running (or after stopping with Ctrl+C), inspect the blocks:

```bash
./02-inspect-blocks.sh
```

This shows:
- Total number of blocks
- Blocks per epoch
- Miner information
- Time range
- Current difficulty

### Step 4: Clean Storage (Optional)

To start fresh:

```bash
./00-clean-storage.sh
```

Then run `./01-mine-blocks.sh` again.

## Understanding Difficulty Adjustment

The difficulty adjustment algorithm analyzes the previous epoch:

```
Expected Time = 40 blocks × 60 seconds = 2400 seconds (40 minutes)
Actual Time = Time between first and last block in epoch

Ratio = Actual Time / Expected Time

If ratio < 0.125: Difficulty × 8    (extremely fast, 8x increase - MAX INCREASE)
If ratio < 0.25:  Difficulty × 4    (much too fast, 4x increase)
If ratio < 0.5:   Difficulty × 2    (too fast, double difficulty)
If ratio < 0.75:  Difficulty × 1.5  (fast, increase 50%)
If ratio < 0.9:   Difficulty × 1.1  (slightly fast, increase 10%)
If ratio > 2.0:   Difficulty ÷ 2    (too slow, halve difficulty - MAX DECREASE)
If ratio > 1.5:   Difficulty × 0.67 (slow, decrease 33%)
If ratio > 1.1:   Difficulty × 0.9  (slightly slow, decrease 10%)
Otherwise:        No change
```

## Example Output

### Mining Log
```
[INFO] Mining block at index 0...
[INFO] Mined block 0 with hash 0000...
[INFO] Successfully mined and gossipped block 0

[INFO] Mining block at index 1...
[INFO] Loaded 1 blocks from datastore
[INFO] Set genesis block (index: 0, hash: 0000...)
[INFO] Reconstructed chain with 1 blocks (height: 0)
[INFO] Chain ready for mining. Height: 0, Mining next index: 1
[INFO] Mined block 1 with hash 0001...
[INFO] Successfully mined and gossipped block 1

...

[INFO] Mining block at index 40...
[INFO] Loaded 40 blocks from datastore
[INFO] Reconstructed chain with 40 blocks (height: 39)
[INFO] Chain ready for mining. Height: 39, Mining next index: 40
[INFO] Mined block 40 with difficulty 1200 (adjusted from 1000)
```

### Storage Inspection
```
📊 Blockchain Storage Statistics
================================

📦 Total Blocks: 45
   Canonical: 45
   Orphaned: 0

⏱  Time Range:
   First Block: 2025-10-20 03:00:00 UTC
   Last Block:  2025-10-20 04:30:00 UTC
   Duration: 1 hours, 30 minutes

📈 Blocks per Epoch:
   Epoch 0: 40 blocks (difficulty: 1000)
   Epoch 1: 5 blocks (difficulty: 1200)

🏆 Top Miners:
   12D3KooW...gQiX: 45 blocks
```

## Debugging

The miner includes detailed logging to help debug issues:

```bash
export RUST_LOG=debug
./01-mine-blocks.sh
```

Key log messages to watch for:
- `Loaded X blocks from datastore` - Shows how many blocks were loaded
- `Reconstructed chain with X blocks (height: Y)` - Shows chain reconstruction
- `Chain ready for mining. Height: X, Mining next index: Y` - Shows mining state
- `Mined block X with difficulty Y` - Shows the difficulty used

## Troubleshooting

### "Error mining block X: Invalid block index"

This means the chain reconstruction isn't working properly. Check the logs for:
- How many blocks were loaded
- What the chain height is after reconstruction
- What index is being mined

### "Block X already exists in chain"

This is normal if you restart the miner. It will skip already-mined blocks and continue from where it left off.

### Difficulty not changing

Make sure you've mined more than 40 blocks. Difficulty only adjusts after completing a full epoch.

## Script Reference

| Script | Description |
|--------|-------------|
| `00-clean-storage.sh` | Remove all mined blocks and start fresh |
| `01-mine-blocks.sh` | Start the miner node |
| `02-inspect-blocks.sh` | View storage statistics and blocks |
| `03-view-difficulty-progression.sh` | View difficulty changes over time |
| `04-view-status-page.sh` | Open the HTTP status dashboard |

## Architecture

```
┌──────────────────────────────────────┐
│         Miner Node                   │
│                                      │
│  1. Load historical blocks from DB   │
│  2. Reconstruct blockchain state     │
│  3. Calculate next difficulty        │
│  4. Mine block with PoW              │
│  5. Save block to DB                 │
│  6. Loop to step 1                   │
│                                      │
│  Difficulty recalculates every       │
│  40 blocks based on mining speed     │
└──────────────────────────────────────┘
         │
         ▼
    ./tmp/storage/miner/
    (RocksDB Datastore)
```

## Related Documentation

- [Mining Documentation](../../../rust/modality-network-mining/README.md)
- [Epoch Manager](../../../rust/modality-network-mining/src/epoch.rs)
- [Miner Action](../../../rust/modality-network-node/src/actions/miner.rs)

## Notes

- The miner runs indefinitely until stopped (Ctrl+C)
- All blocks are persisted automatically
- The chain state is fully reconstructed on each block to ensure correct difficulty calculation
- This example is self-contained and doesn't require network connectivity

