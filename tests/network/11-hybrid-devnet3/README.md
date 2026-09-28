# Hybrid Devnet3 - Three Miner/Sequencer Hybrid Consensus

This example demonstrates hybrid consensus with multiple nodes where:
- **3 miners** mine blocks and nominate sequencers (rotating nominations)
- **Sequencers are selected from epoch N-2 mining nominations**
- **Validation starts at epoch >= 2**
- **3 sequencers** run Shoal consensus to order network events

## Overview

In hybrid consensus:
1. Miners produce blocks that nominate sequencers (no network events in blocks)
2. The sequencer set for mining epoch N is determined from nominations in epoch N-2
3. Sequencers run Shoal consensus to order network events (contract commits)

This test demonstrates:
- Multiple miners proposing different sequencers
- Sequencer set selection from shuffled epoch N-2 nominations
- Multi-sequencer Shoal consensus

## Test Scenario

This test runs 3 nodes, each:
1. Mines blocks (epochs 0, 1) with rotating sequencer nominations
2. At epoch 2, nodes that were nominated in epoch 0 become sequencers
3. Continues mining while selected sequencers also run consensus

## Usage

### Run Manually

Terminal 1:
```bash
cd tests/network/11-hybrid-devnet3
./01-run-miner1.sh
```

Terminal 2:
```bash
./02-run-miner2.sh
```

Terminal 3:
```bash
./03-run-miner3.sh
```

### Run as Test

```bash
cd tests/network/11-hybrid-devnet3
./test.sh
```

## What to Expect

1. **Epoch 0-1**: All 3 nodes mine blocks, each nominating sequencers
2. **Epoch 2**: Sequencer set calculated from epoch 0 nominations
3. **Epoch 2+**: Selected sequencers run Shoal consensus while all continue mining

## Configuration

- **Network**: `devnet3-hybrid`
- **Miners**: 3 nodes rotating sequencer nominations
- **Sequencer selection**: Epoch N-2 lookback with shuffling
- **Blocks per epoch**: 40
- **Consensus**: Byzantine fault tolerant (BFT) with f=0 (3 nodes, need 2f+1=3)

## Key Logs to Watch

- `🎯 EPOCH X STARTED` - Epoch transitions
- `📡 Broadcasted epoch X transition` - Coordination signals
- `Sequencer set for epoch X: N sequencers` - Set calculation
- `🏛️ This node IS a sequencer` / `This node is NOT` - Selection results
- `🚀 Starting Shoal consensus loop` - Consensus activation

