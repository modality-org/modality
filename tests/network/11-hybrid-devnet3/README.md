# Hybrid Devnet3 - Three Miner/Validator Hybrid Consensus

This example demonstrates hybrid consensus with multiple nodes where:
- **3 miners** mine blocks and nominate validators (rotating nominations)
- **Validators are selected from epoch N-2 mining nominations**
- **Validation starts at epoch >= 2**
- **3 validators** run Shoal consensus to order network events

## Overview

In hybrid consensus:
1. Miners produce blocks that nominate validators (no network events in blocks)
2. The validator set for mining epoch N is determined from nominations in epoch N-2
3. Validators run Shoal consensus to order network events (contract commits)

This test demonstrates:
- Multiple miners proposing different validators
- Validator set selection from shuffled epoch N-2 nominations
- Multi-validator Shoal consensus

## Test Scenario

This test runs 3 nodes, each:
1. Mines blocks (epochs 0, 1) with rotating validator nominations
2. At epoch 2, nodes that were nominated in epoch 0 become validators
3. Continues mining while selected validators also run consensus

## Usage

### Run Manually

Terminal 1:
```bash
cd examples/network/11-hybrid-devnet3
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
cd examples/network/11-hybrid-devnet3
./test.sh
```

## What to Expect

1. **Epoch 0-1**: All 3 nodes mine blocks, each nominating validators
2. **Epoch 2**: Validator set calculated from epoch 0 nominations
3. **Epoch 2+**: Selected validators run Shoal consensus while all continue mining

## Configuration

- **Network**: `devnet3-hybrid`
- **Miners**: 3 nodes rotating validator nominations
- **Validator selection**: Epoch N-2 lookback with shuffling
- **Blocks per epoch**: 40
- **Consensus**: Byzantine fault tolerant (BFT) with f=0 (3 nodes, need 2f+1=3)

## Key Logs to Watch

- `🎯 EPOCH X STARTED` - Epoch transitions
- `📡 Broadcasted epoch X transition` - Coordination signals
- `Validator set for epoch X: N validators` - Set calculation
- `🏛️ This node IS a validator` / `This node is NOT` - Selection results
- `🚀 Starting Shoal consensus loop` - Consensus activation

