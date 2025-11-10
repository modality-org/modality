# Run Devnet2 - Static 2-Validator Network with Active Shoal Consensus ✅

This example demonstrates running a local devnet with **2 static validators** running **Shoal consensus** and no miners. It's the simplest multi-validator configuration and useful for testing validator connectivity and network behavior.

**Status**: ✅ **Fully functional** - Shoal consensus is active and running on both validators.

## Overview

This example sets up:
- **2 static validators** with pre-configured identities
- **Genesis round** pre-signed by both validators
- **Local networking** (127.0.0.1) for easy testing
- **No miners** - validators are fixed in the configuration

**Note**: This demonstrates validator nodes running Shoal consensus. The validators will connect to each other and run consensus rounds. Since there are no miners, the consensus will order validator operations rather than transaction blocks.

## Key Concepts

### Static Validator Set

The network configuration (`fixtures/network-configs/devnet2/config.json`) defines:
- A static list of 2 validator peer IDs
- Genesis round (round 0) with certificates from both validators
- Local bootstrapper addresses for node discovery

### Validator Identities

- **Validator 1 (node1)**
  - Peer ID: `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`
  - Port: `10201`
  - Passfile: `fixtures/passfiles/node1.mod_passfile`

- **Validator 2 (node2)**
  - Peer ID: `12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB`
  - Port: `10202`
  - Passfile: `fixtures/passfiles/node2.mod_passfile`

## Usage

### Prerequisites

Build the Modal CLI if not already built:

```bash
cd ../../../rust
cargo build --package modal
```

### Option 1: Run Validators in Separate Terminals

**Terminal 1 - Validator 1:**
```bash
cd examples/network/02-run-devnet2
./01-run-node1.sh
```

**Terminal 2 - Validator 2:**
```bash
cd examples/network/02-run-devnet2
./02-run-node2.sh
```

### Option 2: Run Automated Test

```bash
cd examples/network/02-run-devnet2
./test.sh
```

The test script will:
1. Create both nodes from templates
2. Verify configurations
3. Start both validators
4. Check connectivity
5. Clean up automatically

## What to Expect

When both validators are running:

1. ✅ **Validators connect** to each other via local addresses
2. ✅ **Genesis round loads** from network configuration
3. ✅ **Peer discovery** works via bootstrappers
4. ✅ **Network connectivity** is established
5. ✅ **Shoal consensus starts** running on each validator
6. ✅ **Consensus rounds advance** (logged every 10 rounds)

**Expected behavior**: Validators run consensus rounds but don't produce transaction blocks because:
- There are no miners creating transaction blocks
- Consensus operates on validator operations rather than transactions
- Full BFT operation requires certificate exchange (networking integration pending)

You should see in logs:
- Successful connections between validators
- Peer information exchange (Identify protocol)
- Ping/pong messages confirming connectivity
- **"🏛️  This node is a static validator - starting Shoal consensus"**
- **"✅ ShoalValidator initialized successfully"**
- **"🚀 Starting Shoal consensus loop"**
- **"⚙️  Consensus round: X"** messages every 10 rounds

## File Structure

```
02-run-devnet2/
├── README.md              # This file
├── 01-run-node1.sh        # Run validator 1
├── 02-run-node2.sh        # Run validator 2
├── test.sh                # Automated test script
└── tmp/                   # Created at runtime
    ├── node1/             # Validator 1 data
    │   ├── config.json
    │   ├── node.passfile
    │   ├── storage/
    │   └── logs/
    └── node2/             # Validator 2 data
        ├── config.json
        ├── node.passfile
        ├── storage/
        └── logs/
```

## Configuration Details

### Network Configuration

The network config (`fixtures/network-configs/devnet2/config.json`) includes:
- `validators`: List of 2 static validator peer IDs
- `bootstrappers`: Local addresses for peer discovery
- `rounds.0`: Genesis round with pre-signed certificates

### Node Configurations

Each node template (`fixtures/network-node-configs/devnet2/node*.json`) specifies:
- Passfile path (deterministic identity)
- Storage path
- Listen address and port
- Bootstrapper addresses (pointing to other validator)
- Network config path

## Verifying Connectivity

Check validator information:

```bash
# From validator 1 directory
cd tmp/node1
modal node info

# From validator 2 directory  
cd tmp/node2
modal node info
```

Check logs:

```bash
# View validator 1 logs
tail -f tmp/node1/logs/node.log

# View validator 2 logs
tail -f tmp/node2/logs/node.log
```

Look for messages indicating:
- `Behaviour(NodeBehaviourEvent: Received` - peer connection established
- `Info { public_key:` - peer information exchanged
- `Event { peer: PeerId(...), result: Ok(...)` - successful pings

## Differences from 06-static-validators

This example (`02-run-devnet2`) differs from `06-static-validators` in:

| Feature | 02-run-devnet2 | 06-static-validators |
|---------|----------------|---------------------|
| Number of validators | 2 | 3 |
| Configuration method | Templates via `modal node create` | Direct config files |
| Network name | devnet2 | static-devnet |
| Primary use | Automated testing | Manual exploration |
| Ports | 10201, 10202 | 10601, 10602, 10603 |

Both examples demonstrate the same core functionality: static validator sets without miners.

## Use Cases

This example is ideal for:
- **Testing** validator connectivity in CI/CD
- **Development** of validator node features
- **Learning** how static validators work
- **Debugging** network communication issues
- **Minimum viable** validator network (2 is smallest BFT config)

## Next Steps

After confirming validators connect:
- Try `03-run-devnet3` for a 3-validator network
- See `05-mining` to add miners to the network
- Explore `06-static-validators` for a manual configuration approach
- Review validator documentation in `rust/modal-node/docs/`

