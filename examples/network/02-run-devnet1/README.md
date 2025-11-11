# Run Devnet1 - Static Single-Validator Network with Active Shoal Consensus ✅

This example demonstrates running a local devnet with **1 static validator** running **Shoal consensus** and no miners. It's the simplest possible validator configuration and useful for testing single-validator node behavior and development.

**Status**: ✅ **Fully functional** - Shoal consensus is active and running on the single validator.

## Overview

This example sets up:
- **1 static validator** with pre-configured identity
- **Genesis round** pre-signed by the validator
- **Local networking** (127.0.0.1) for easy testing
- **No miners** - single validator is fixed in the configuration

**Note**: This demonstrates a single validator node running Shoal consensus. Since there's only one validator, it will run consensus without needing to coordinate with other validators.

## Key Concepts

### Static Validator Set

The network configuration (`fixtures/network-configs/devnet1/config.json`) defines:
- A static list with 1 validator peer ID
- Genesis round (round 0) with certificate from the validator
- No bootstrappers needed (single node)

### Validator Identity

- **Validator 1 (node1)**
  - Peer ID: `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`
  - Port: `10101`
  - Passfile: `fixtures/passfiles/node1.mod_passfile`

## Usage

### Prerequisites

Build the Modal CLI if not already built:

```bash
cd ../../../rust
cargo build --package modal
```

### Option 1: Run Validator Directly

**Single Terminal:**
```bash
cd examples/network/02-run-devnet1
./01-run-node1.sh
```

### Option 2: Run Automated Test

```bash
cd examples/network/02-run-devnet1
./test.sh
```

The test script will:
1. Create the node from template
2. Verify configuration
3. Start the validator
4. Check that consensus is running
5. Clean up automatically

## What to Expect

When the validator is running:

1. ✅ **Validator starts** with devnet1 configuration
2. ✅ **Genesis round loads** from network configuration
3. ✅ **Shoal consensus starts** running on the validator
4. ✅ **Consensus rounds advance** (logged every 10 rounds)

**Expected behavior**: The single validator runs consensus rounds but doesn't need to coordinate with other validators since it's the only one in the network.

You should see in logs:
- Node startup messages
- **"🏛️  This node is a static validator - starting Shoal consensus"**
- **"✅ ShoalValidator initialized successfully"**
- **"🚀 Starting Shoal consensus loop"**
- **"⚙️  Consensus round: X"** messages every 10 rounds

## File Structure

```
02-run-devnet1/
├── README.md              # This file
├── 01-run-node1.sh        # Run validator 1
├── test.sh                # Automated test script
└── tmp/                   # Created at runtime
    ├── node1/             # Validator 1 data
    │   ├── config.json
    │   ├── node.passfile
    │   ├── storage/
    │   └── logs/
    └── test-logs/         # Test execution logs
```

## Configuration Details

### Network Configuration

The network config (`fixtures/network-configs/devnet1/config.json`) includes:
- Single validator peer ID in the validator set
- Genesis round with pre-signed certificate
- No bootstrappers (single node doesn't need peer discovery)

### Node Configuration

The node template (`fixtures/network-node-configs/devnet1/node1.json`) specifies:
- Passfile path (deterministic identity)
- Storage path
- Listen address and port (10101)
- Network config path

## Verifying Operation

Check validator information:

```bash
# From validator 1 directory
cd tmp/node1
modal node info
```

Check logs:

```bash
# View validator logs
tail -f tmp/node1/logs/node.log
```

Look for messages indicating:
- `This node is a static validator` - validator mode confirmed
- `ShoalValidator initialized` - consensus started
- `Consensus round: X` - consensus is progressing

## Differences from Other Examples

This example (`02-run-devnet1`) differs from other examples:

| Feature | 02-run-devnet1 | 02-run-devnet2 | 03-run-devnet3 |
|---------|----------------|----------------|----------------|
| Number of validators | 1 | 2 | 3 |
| Bootstrappers | None (single node) | Yes | Yes |
| Network name | devnet1 | devnet2 | devnet3 |
| Primary use | Single-node testing | 2-validator BFT | 3-validator BFT |
| Port | 10101 | 10201, 10202 | 10301, 10302, 10303 |

## Use Cases

This example is ideal for:
- **Development** of single validator node features
- **Testing** validator node behavior in isolation
- **Learning** how static validators work
- **Debugging** consensus implementation without network complexity
- **CI/CD** testing for single-node scenarios

## Next Steps

After confirming the validator runs:
- Try `02-run-devnet2` for a 2-validator network
- Try `03-run-devnet3` for a 3-validator network
- See `05-mining` to add miners to the network
- Review validator documentation in `rust/modal-node/docs/`

