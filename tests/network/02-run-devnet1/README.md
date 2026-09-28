# Run Devnet1 - Static Single-Sequencer Network with Active Shoal Consensus ✅

This example demonstrates running a local devnet with **1 static sequencer** running **Shoal consensus** and no miners. It's the simplest possible sequencer configuration and useful for testing single-sequencer node behavior and development.

**Status**: ✅ **Fully functional** - Shoal consensus is active and running on the single sequencer.

## Overview

This example sets up:
- **1 static sequencer** with pre-configured identity
- **Genesis round** pre-signed by the sequencer
- **Local networking** (127.0.0.1) for easy testing
- **No miners** - single sequencer is fixed in the configuration

**Note**: This demonstrates a single sequencer node running Shoal consensus. Since there's only one sequencer, it will run consensus without needing to coordinate with other sequencers.

## Key Concepts

### Static Sequencer Set

The network configuration (`fixtures/network-configs/devnet1/config.json`) defines:
- A static list with 1 sequencer peer ID
- Genesis round (round 0) with certificate from the sequencer
- No bootstrappers needed (single node)

### Sequencer Identity

- **Sequencer 1 (node1)**
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

### Option 1: Run Sequencer Directly

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
3. Start the sequencer
4. Check that consensus is running
5. Clean up automatically

## What to Expect

When the sequencer is running:

1. ✅ **Sequencer starts** with devnet1 configuration
2. ✅ **Genesis round loads** from network configuration
3. ✅ **Shoal consensus starts** running on the sequencer
4. ✅ **Consensus rounds advance** (logged every 10 rounds)

**Expected behavior**: The single sequencer runs consensus rounds but doesn't need to coordinate with other sequencers since it's the only one in the network.

You should see in logs:
- Node startup messages
- **"🏛️  This node is a static sequencer - starting Shoal consensus"**
- **"✅ ShoalSequencer initialized successfully"**
- **"🚀 Starting Shoal consensus loop"**
- **"⚙️  Consensus round: X"** messages every 10 rounds

## File Structure

```
02-run-devnet1/
├── README.md              # This file
├── 01-run-node1.sh        # Run sequencer 1
├── test.sh                # Automated test script
└── tmp/                   # Created at runtime
    ├── node1/             # Sequencer 1 data
    │   ├── config.json
    │   ├── node.modal_passfile
    │   ├── storage/
    │   └── logs/
    └── test-logs/         # Test execution logs
```

## Configuration Details

### Network Configuration

The network config (`fixtures/network-configs/devnet1/config.json`) includes:
- Single sequencer peer ID in the sequencer set
- Genesis round with pre-signed certificate
- No bootstrappers (single node doesn't need peer discovery)

### Node Configuration

The node template (`fixtures/network-node-configs/devnet1/node1.json`) specifies:
- Passfile path (deterministic identity)
- Storage path
- Listen address and port (10101)
- Network config path

## Verifying Operation

Check sequencer information:

```bash
# From sequencer 1 directory
cd tmp/node1
modal node info
```

Check logs:

```bash
# View sequencer logs
tail -f tmp/node1/logs/node.log
```

Look for messages indicating:
- `This node is a static sequencer` - sequencer mode confirmed
- `ShoalSequencer initialized` - consensus started
- `Consensus round: X` - consensus is progressing

## Differences from Other Examples

This example (`02-run-devnet1`) differs from other examples:

| Feature | 02-run-devnet1 | 02-run-devnet2 | 03-run-devnet3 |
|---------|----------------|----------------|----------------|
| Number of sequencers | 1 | 2 | 3 |
| Bootstrappers | None (single node) | Yes | Yes |
| Network name | devnet1 | devnet2 | devnet3 |
| Primary use | Single-node testing | 2-sequencer BFT | 3-sequencer BFT |
| Port | 10101 | 10201, 10202 | 10301, 10302, 10303 |

## Use Cases

This example is ideal for:
- **Development** of single sequencer node features
- **Testing** sequencer node behavior in isolation
- **Learning** how static sequencers work
- **Debugging** consensus implementation without network complexity
- **CI/CD** testing for single-node scenarios

## Next Steps

After confirming the sequencer runs:
- Try `02-run-devnet2` for a 2-sequencer network
- Try `03-run-devnet3` for a 3-sequencer network
- See `05-mining` to add miners to the network
- Review sequencer documentation in `rust/modality-node/docs/`

