# Status Page: Tabbed Interface with Epoch Nominees

## Overview

The status page now features a modern tabbed interface that organizes information into three main sections: **Overview**, **Mining**, and **Sequencing**. This makes it easier to navigate and find the information you need.

## Tab Navigation

The status page includes three tabs at the top:

1. **Overview** - General node and network information
2. **Mining** - Block mining statistics and history
3. **Sequencing** - Epoch nominees and sequencer selection

**Features:**
- Tabs persist between page refreshes (your selected tab is saved)
- Clean, modern design with hover effects
- Auto-refresh every 10 seconds maintains your current tab

## Overview Tab

The Overview tab displays general node and network status:

### Stats Boxes
- **Connected Peers** - Number of peers currently connected to your node
- **Block Height** - Total number of blocks in the canonical chain
- **Cumulative Difficulty** - Total computational work across all blocks

### Information Cards
- **Node Information** - Your node's Peer ID and listeners
- **Blockchain Status** - Current round and latest block round
- **Genesis Block** - Details about Block 0
- **Connected Peers** - List of all connected peer IDs

## Mining Tab

The Mining tab focuses on block mining activity:

### Stats Boxes
- **Block Height** - Total number of blocks mined
- **Blocks Mined by Node** - Number of blocks where your node was nominated
- **Current Difficulty** - Current mining difficulty

### Mining History
- **Recent Blocks (Last 80)** - Most recent blocks in reverse chronological order
- **First 40 Blocks** - The genesis epoch blocks

Each block row shows:
- Block Index
- Epoch
- Block Hash (truncated)
- Nominated Peer (truncated)
- Timestamp
- Time Delta from previous block

## Sequencing Tab

The Sequencing tab displays epoch nominee information and shuffle order:

### Stats Boxes
- **Current Epoch** - The current epoch number
- **Block Height** - Total blocks in the chain
- **Completed Epochs** - Number of completed epochs

### Epoch Nominees (Shuffled Order)

For each completed previous epoch (up to 5 most recent), displays:

1. **Shuffle Rank** - The position (1-40) after the deterministic shuffle
2. **Block Index** - The block that nominated this peer
3. **Nominating Block Hash** - Hash of the nominating block (truncated)
4. **Nominated Peer** - The nominated peer ID (truncated)

### How the Shuffle Works

1. **Seed Calculation**: All nonces from blocks in the epoch are XOR'd together to create a deterministic seed
2. **Fisher-Yates Shuffle**: Using the seed, the Fisher-Yates algorithm shuffles the 40 nominees
3. **Ranking**: The shuffled order determines the priority of nominees (rank 1 is highest priority)

## Implementation Details

**File Modified**: `rust/modality-network-node/src/status_server.rs`

**Key Changes**:
- Added CSS for tabbed interface with smooth transitions
- Implemented JavaScript tab switching with localStorage persistence
- Reorganized content into three logical tab sections
- Added logic to calculate shuffled nominations for previous epochs
- Used `modality_utils::shuffle::fisher_yates_shuffle` for deterministic shuffling

**Tab Persistence**:
- Active tab is saved to browser's localStorage
- When page refreshes (auto or manual), your selected tab is restored
- Works across multiple browser sessions

## How to View

1. Start a Modality node with the `status_port` configured
2. Open your browser to `http://localhost:<status_port>` (default: 8080)
3. Click on the tabs to navigate between different sections
4. Page auto-refreshes every 10 seconds while maintaining your active tab

## Example Output

### Sequencing Tab - Epoch Nominees

```
Epoch 0 Nominees (Shuffled Order)
┌──────────────┬─────────────┬──────────────────────┬────────────────────┐
│ Shuffle Rank │ Block Index │ Nominating Block Hash│ Nominated Peer     │
├──────────────┼─────────────┼──────────────────────┼────────────────────┤
│ 1            │ 23          │ 0000abcd...1234efgh  │ 12D3KooW...xxHd   │
│ 2            │ 15          │ 0001bcde...2345fghi  │ 12D3KooW...yyHd   │
│ 3            │ 7           │ 0002cdef...3456ghij  │ 12D3KooW...zzHd   │
│ ...          │ ...         │ ...                  │ ...                │
│ 40           │ 39          │ 0027wxyz...9012abcd  │ 12D3KooW...aaHd   │
└──────────────┴─────────────┴──────────────────────┴────────────────────┘
```

## Use Cases

1. **Sequencer Selection**: The top 27 from the shuffle become nominated sequencers
2. **Fairness Verification**: Verify that the shuffle is deterministic and consistent
3. **Mining Analysis**: See which blocks and peers were nominated in previous epochs
4. **Network Transparency**: All nodes calculate the same shuffle order for the same epoch
5. **Node Monitoring**: Easily track your node's status across different aspects

## Technical Notes

- **BLOCKS_PER_EPOCH**: Currently set to 40 blocks per epoch
- **Deterministic**: The shuffle is deterministic - all nodes will calculate the same order
- **Seed Source**: XOR of all nonces from the epoch's blocks
- **Auto-refresh**: Page auto-refreshes every 10 seconds to show latest data
- **Tab Persistence**: Uses localStorage to remember your active tab

## Testing

To test the feature:

```bash
# Start a miner with status port
cd examples/network/05-mining
./01-mine-blocks.sh

# In another terminal, view the status page
./04-view-status-page.sh

# Navigate between tabs
# - Overview tab shows general node information
# - Mining tab shows block history
# - Sequencing tab appears after 40+ blocks are mined (one complete epoch)
```

## Related Files

- `rust/modality-network-node/src/status_server.rs` - Main implementation
- `rust/modality-utils/src/shuffle.rs` - Fisher-Yates shuffle implementation
- `rust/modal-datastore/src/models/sequencer_selection.rs` - Sequencer selection logic
- `rust/modal-miner/src/epoch.rs` - Epoch management and shuffle calculation

