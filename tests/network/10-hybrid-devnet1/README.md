# Hybrid Devnet1 - Single Miner/Validator Hybrid Consensus

This example demonstrates hybrid consensus where:
- **1 miner** mines blocks and nominates a validator (itself)
- **Validators are selected from epoch N-2 mining nominations**
- **Validation starts at epoch >= 2**

## Overview

In hybrid consensus:
1. Miners produce blocks that nominate validators (no network events in blocks)
2. Validators run Shoal consensus to order network events (contract commits)
3. The validator set for mining epoch N is determined from nominations in epoch N-2

This provides a 2-epoch lookback to ensure validator sets are stable before activation.

## Test Scenario

This test runs a single node that:
1. Mines blocks (epochs 0, 1) while nominating itself
2. At epoch 2, automatically becomes a validator based on epoch 0 nominations
3. Continues mining while also validating network events

## Usage

### Run Manually

```bash
cd examples/network/10-hybrid-devnet1
./01-run-hybrid-node.sh
```

### Run as Test

```bash
cd examples/network/10-hybrid-devnet1
./test.sh
```

## What to Expect

1. **Epoch 0-1**: Node mines blocks, nominates itself, logs "waiting for epoch >= 2"
2. **Epoch 2**: Node detects it's in the validator set from epoch 0 nominations
3. **Epoch 2+**: Node runs both mining and Shoal consensus simultaneously

## Configuration

- **Network**: `devnet1-hybrid`
- **Miner nominees**: Node's own peer ID
- **Validator selection**: Epoch N-2 lookback
- **Blocks per epoch**: 40

## Key Logs to Watch

- `🎯 EPOCH X STARTED` - Epoch transitions
- `📡 Broadcasted epoch X transition` - Epoch coordination signal
- `🔔 Epoch transition detected` - Validator listening for transitions
- `🏛️ This node IS a validator for epoch X` - Validator activation
- `🚀 Starting Shoal consensus loop` - Consensus started

