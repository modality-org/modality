# Run Devnet3 - Static 3-Sequencer Network with Active Shoal Consensus ✅

This example demonstrates running a local devnet with **3 static sequencers** running **Shoal consensus** and no miners. This is the standard multi-sequencer configuration for testing consensus behavior and network dynamics.

**Status**: ✅ **Fully functional** - Shoal consensus is active and running on all sequencers.

## Overview

This example sets up:
- **3 static sequencers** with pre-configured identities
- **Genesis round** pre-signed by all sequencers
- **Local networking** (127.0.0.1) for easy testing
- **No miners** - sequencers are fixed in the configuration

**Note**: This demonstrates sequencer nodes running Shoal consensus. The sequencers will connect to each other and run consensus rounds, creating certificates and advancing through epochs. Since there are no miners, the consensus will order sequencer operations rather than transaction blocks.

## Key Concepts

### Static Sequencer Set

The network configuration (`fixtures/network-configs/devnet3/config.json`) defines:
- A static list of 3 sequencer peer IDs
- Bootstrap addresses for peer discovery
- Genesis round (round 0) with certificates from all sequencers

The 3 sequencers are:
1. `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd` (Node 1)
2. `12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB` (Node 2)
3. `12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se` (Node 3)

### Sequencer Nodes

Each sequencer runs using `modal node run-sequencer` which:
- Loads the network configuration with static sequencers
- Connects to other sequencers via bootstrap addresses
- Subscribes to mining block gossip (though none will occur without miners)
- Maintains the canonical chain state
- Syncs from peers on startup

## Usage

### Starting the Sequencers

Run each sequencer in a separate terminal:

**Terminal 1 - Start Sequencer 1:**
```bash
cd examples/network/03-run-devnet3
./01-run-node1.sh
```

**Terminal 2 - Start Sequencer 2:**
```bash
cd examples/network/03-run-devnet3
./02-run-node2.sh
```

**Terminal 3 - Start Sequencer 3:**
```bash
cd examples/network/03-run-devnet3
./03-run-node3.sh
```

### Running the Test

To test all sequencers automatically:

```bash
cd examples/network/03-run-devnet3
./test.sh
```

This will:
1. Build the Modal CLI if needed
2. Clean up previous test data
3. Start all 3 sequencers
4. Verify they're running on their ports
5. Check for peer connections
6. Clean up processes

## Expected Behavior

Once all sequencers are running, you should see:

1. **✅ Sequencers connect** to each other via the bootstrap addresses
2. **✅ Peer discovery** completes (visible in logs via libp2p Identify protocol)
3. **✅ Network topology** is established with all 3 sequencers connected
4. **✅ Shoal consensus starts** running on each sequencer
5. **✅ Consensus rounds advance** (logged every 10 rounds: "⚙️  Consensus round: X")
6. **✅ Sequencers create** ShoalSequencer instances with the static committee

### What You'll See in the Logs

Successful sequencer startup with consensus includes:
- Network configuration loaded with static sequencers
- Listening on configured port (10301, 10302, or 10303)
- Bootstrap connections established
- Peer information exchanged (Identify protocol)
- Ping/pong messages between sequencers
- **"🏛️  This node is a static sequencer - starting Shoal consensus"**
- **"📋 Sequencer index: X/3"** - shows sequencer position
- **"✅ ShoalSequencer initialized successfully"**
- **"🚀 Starting Shoal consensus loop"**
- **"⚙️  Consensus round: X"** - appears every 10 rounds

## Network Configuration

### Ports
- **Sequencer 1**: `10301` (WebSocket)
- **Sequencer 2**: `10302` (WebSocket)
- **Sequencer 3**: `10303` (WebSocket)

### Bootstrap Configuration
Each sequencer bootstraps from the other two sequencers:
- Sequencer 1 → connects to sequencers 2 and 3
- Sequencer 2 → connects to sequencers 1 and 3
- Sequencer 3 → connects to sequencers 1 and 2

### Storage
Each sequencer stores its data in:
- `examples/network/03-run-devnet3/tmp/node{1,2,3}/storage/`

Storage is cleared on each run using `modal node clear-storage --yes`.

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Sequencer 1 │────▶│ Sequencer 2 │────▶│ Sequencer 3 │
│   :10301    │◀────│   :10302    │◀────│   :10303    │
└─────────────┘     └─────────────┘     └─────────────┘
       ▲                                       │
       └───────────────────────────────────────┘
          All sequencers form mesh topology
```

## Files

- `01-run-node1.sh` - Starts sequencer 1
- `02-run-node2.sh` - Starts sequencer 2
- `03-run-node3.sh` - Starts sequencer 3
- `test.sh` - Automated test that runs all sequencers
- `tmp/` - Runtime data (created automatically, gitignored)

## Configuration Files

The sequencers use configurations from the `fixtures/` directory:

**Network Config:**
- `fixtures/network-configs/devnet3/config.json` - Network-wide configuration with static sequencer list

**Node Configs:**
- `fixtures/network-node-configs/devnet3/node1.json` - Sequencer 1 configuration
- `fixtures/network-node-configs/devnet3/node2.json` - Sequencer 2 configuration
- `fixtures/network-node-configs/devnet3/node3.json` - Sequencer 3 configuration

**Passfiles:**
- `fixtures/passfiles/node1.mod_passfile` - Identity for sequencer 1
- `fixtures/passfiles/node2.mod_passfile` - Identity for sequencer 2
- `fixtures/passfiles/node3.mod_passfile` - Identity for sequencer 3

## Troubleshooting

### Sequencers Don't Connect

**Issue:** Sequencers start but don't connect to each other

**Solution:**
1. Ensure all 3 sequencers are running
2. Check that ports 10301, 10302, and 10303 are not in use
3. Verify bootstrap addresses in node configs match running sequencers
4. Check logs for connection errors

### Port Already in Use

**Issue:** Error about port already in use

**Solution:**
```bash
# Find and kill processes using the ports
lsof -ti:10301 | xargs kill -9
lsof -ti:10302 | xargs kill -9
lsof -ti:10303 | xargs kill -9
```

### Storage Issues

**Issue:** "Storage error" or "Database locked"

**Solution:**
```bash
# Clean up storage directories
rm -rf tmp/node1/storage tmp/node2/storage tmp/node3/storage
```

## Differences from Production

This devnet differs from production networks in several ways:

1. **Static Sequencers**: Production uses dynamic sequencer selection from mining epochs
2. **Local Networking**: All sequencers run on localhost (production uses public IPs)
3. **No Mining**: No mining activity (production has miners creating blocks)
4. **Consensus Infrastructure Only**: Consensus loop runs but full BFT operation requires networking integration (certificate exchange via gossip)
5. **Genesis Round Only**: Only the pre-configured genesis round exists

## Use Cases

This example is useful for:
- **Testing sequencer connectivity** with 3 nodes
- **Verifying network topology** formation
- **Debugging peer discovery** mechanisms
- **Testing static sequencer** configurations
- **Development environment** setup for consensus work

## Next Steps

For active mining and block production, see:
- `examples/network/05-mining/` - Mining with dynamic sequencer selection
- `examples/network/04-sync-miner-blocks/` - Block synchronization between nodes

## Related Examples

- `02-run-devnet2/` - Simpler 2-sequencer setup
- `06-static-sequencers/` - Detailed static sequencer example with utilities
- `05-mining/` - Mining with dynamic sequencers

