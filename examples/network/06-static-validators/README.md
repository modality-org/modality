# Static Validators Example

This example demonstrates how to run a static devnet with a fixed set of validators and no miners. This is useful for development and testing environments where you want predictable validator behavior without the complexity of mining.

## Overview

This example sets up a local network with:
- **3 static validators** (no mining required)
- **Genesis round** pre-configured with validator certificates
- **No miners** (validators are fixed in the network configuration)
- **Validators observe mining events** (but no blocks will be produced without miners)

**Note**: This example demonstrates how to run static validator nodes that are configured and connected. However, since there are no miners and consensus is not enabled in the current validator implementation, no blocks will be produced. The validators are successfully running and connected to each other, ready to observe mining events when miners are added.

## Key Concepts

### Static Validators

Unlike production networks that use dynamic validator selection from mining epochs, this devnet uses a **static validator set** defined in the network configuration. The validators are:

1. `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd` (Validator 1)
2. `12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB` (Validator 2)
3. `12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se` (Validator 3)

These peer IDs correspond to the passfiles in `fixtures/passfiles/`.

### Network Configuration

The network configuration (`configs/static-devnet.json`) includes:
- `validators` field: List of static validator peer IDs
- `rounds` field: Pre-configured genesis round (round 0) with certificates
- `bootstrappers` field: Initial peers for network discovery

## Files

```
06-static-validators/
├── configs/
│   ├── static-devnet.json   # Network config with static validators
│   ├── validator1.json       # Node config for validator 1
│   ├── validator2.json       # Node config for validator 2
│   └── validator3.json       # Node config for validator 3
├── 00-clean-storage.sh       # Clean validator storage
├── 01-run-validator1.sh      # Run validator 1
├── 02-run-validator2.sh      # Run validator 2
├── 03-run-validator3.sh      # Run validator 3
├── 04-view-validators-status.sh    # Check status of all validators
├── 05-view-consensus-state.sh      # View consensus state
├── 06-run-all-validators.sh        # Run all validators at once
└── README.md                 # This file
```

## Prerequisites

- Modal CLI installed and available in your PATH
- The passfiles in `fixtures/passfiles/` should exist:
  - `node1.mod_passfile`
  - `node2.mod_passfile`
  - `node3.mod_passfile`

## Usage

### Option 1: Run Validators Individually (Recommended for Development)

Open 3 separate terminals and run each validator:

**Terminal 1:**
```bash
cd examples/network/06-static-validators
./01-run-validator1.sh
```

**Terminal 2:**
```bash
cd examples/network/06-static-validators
./02-run-validator2.sh
```

**Terminal 3:**
```bash
cd examples/network/06-static-validators
./03-run-validator3.sh
```

### Option 2: Run All Validators in Background

```bash
cd examples/network/06-static-validators
./06-run-all-validators.sh
```

This will start all validators in the background and save logs to `tmp/validator*.log`.

### Viewing Status

Check the status of all validators:

```bash
./04-view-validators-status.sh
```

View the consensus state:

```bash
./05-view-consensus-state.sh
```

### Cleaning Up

To clean the storage directories:

```bash
./00-clean-storage.sh
```

To stop validators running in the background, use the PIDs displayed when you started them:

```bash
kill <PID1> <PID2> <PID3>
```

## Expected Behavior

Once all validators are running, you should see:

1. **✅ Validators connect** to each other via the bootstrapper addresses
2. **✅ Genesis round loads** from the network configuration
3. **✅ Peer information is exchanged** between validators
4. **✅ Successful pings** and connections are established

**Note on Block Production**: The validators are correctly configured and running, but **no blocks will be produced** in this example because:
- Validator nodes are designed to **observe mining events** from miners
- There are **no miners** in this static setup
- **Consensus is not enabled** in the current validator node implementation

This example successfully demonstrates:
- Setting up a network with static validators
- Validator nodes connecting and communicating
- Loading a static validator configuration

To have actual block production, you would need to either:
- Add miners to produce mining blocks, OR
- Enable consensus in the validator node code (currently commented out in `rust/modal-node/src/actions/server.rs`)

## How It Works

1. **Network Initialization**: Each validator loads the `static-devnet.json` network config, which includes:
   - The static validator set (3 pre-defined peer IDs)
   - Genesis round (round 0) with pre-signed certificates

2. **Validator Nodes**: Each validator:
   - Subscribes to mining block gossip
   - Maintains connections to other validators
   - Waits for mining events to process
   - Syncs with peers on startup

3. **Network Communication**: The validators:
   - Exchange peer information via libp2p
   - Ping each other to verify connectivity
   - Are ready to observe and process mining blocks when miners join

## Network Ports

- Validator 1: `10601`
- Validator 2: `10602`
- Validator 3: `10603`

All validators listen on `0.0.0.0` (all interfaces) for local testing.

## Storage Locations

- Validator 1: `./tmp/storage/validator1`
- Validator 2: `./tmp/storage/validator2`
- Validator 3: `./tmp/storage/validator3`

## Comparison with Mining-Based Networks

| Aspect | Static Validators (this example) | Mining-Based (e.g., testnet) |
|--------|----------------------------------|------------------------------|
| Validator Selection | Fixed in config | Dynamic from mining epochs |
| Mining Required | No | Yes |
| Validator Changes | Requires config update | Automatic each epoch |
| Use Case | Development, testing | Production |
| Complexity | Lower | Higher |

## Troubleshooting

### Validators not connecting

- Ensure all validators are running
- Check that ports 10601, 10602, 10603 are available
- Verify passfiles exist and are readable

### Consensus not progressing

- Verify at least 2/3 validators are running (2 out of 3)
- Check logs for errors
- Ensure `--enable-consensus` flag is used

### Storage errors

- Run `./00-clean-storage.sh` to reset
- Check disk space
- Verify write permissions in `./tmp/`

## Advanced Configuration

### Adding More Validators

1. Generate a new passfile
2. Add the peer ID to the `validators` array in `static-devnet.json`
3. Add the peer to the genesis round with appropriate certificates
4. Create a new node config file
5. Create a run script for the new validator

### Changing Network Parameters

Edit `configs/static-devnet.json` to:
- Change port numbers in `bootstrappers`
- Modify the genesis round structure
- Add additional network metadata

## Related Documentation

- [Static Validators Implementation](../../../STATIC_VALIDATORS_IMPLEMENTATION.md)
- [Network Configuration Guide](../../../rust/modal-networks/README.md)
- [Consensus Documentation](../../../rust/modal-validator-consensus/README.md)

## See Also

- Example 02: `02-run-devnet2` - Dynamic devnet with 2 nodes
- Example 03: `03-run-devnet3` - Dynamic devnet with 3 nodes  
- Example 05: `05-mining` - Mining example with dynamic validators

