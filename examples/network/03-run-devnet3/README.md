# Run Devnet3 - Static 3-Validator Network

This example demonstrates running a local devnet with **3 static validators** and no miners. This is the standard multi-validator configuration for testing consensus behavior and network dynamics.

## Overview

This example sets up:
- **3 static validators** with pre-configured identities
- **Genesis round** pre-signed by all validators
- **Local networking** (127.0.0.1) for easy testing
- **No miners** - validators are fixed in the configuration

**Note**: This demonstrates validator node setup and connectivity. Like the 2-validator example, no blocks will be produced since validators observe mining events and there are no miners in this configuration.

## Key Concepts

### Static Validator Set

The network configuration (`fixtures/network-configs/devnet3/config.json`) defines:
- A static list of 3 validator peer IDs
- Bootstrap addresses for peer discovery
- Genesis round (round 0) with certificates from all validators

The 3 validators are:
1. `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd` (Node 1)
2. `12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB` (Node 2)
3. `12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se` (Node 3)

### Validator Nodes

Each validator runs using `modal node run-validator` which:
- Loads the network configuration with static validators
- Connects to other validators via bootstrap addresses
- Subscribes to mining block gossip (though none will occur without miners)
- Maintains the canonical chain state
- Syncs from peers on startup

## Usage

### Starting the Validators

Run each validator in a separate terminal:

**Terminal 1 - Start Validator 1:**
```bash
cd examples/network/03-run-devnet3
./01-run-node1.sh
```

**Terminal 2 - Start Validator 2:**
```bash
cd examples/network/03-run-devnet3
./02-run-node2.sh
```

**Terminal 3 - Start Validator 3:**
```bash
cd examples/network/03-run-devnet3
./03-run-node3.sh
```

### Running the Test

To test all validators automatically:

```bash
cd examples/network/03-run-devnet3
./test.sh
```

This will:
1. Build the Modal CLI if needed
2. Clean up previous test data
3. Start all 3 validators
4. Verify they're running on their ports
5. Check for peer connections
6. Clean up processes

## Expected Behavior

Once all validators are running, you should see:

1. **✅ Validators connect** to each other via the bootstrap addresses
2. **✅ Peer discovery** completes (visible in logs via libp2p Identify protocol)
3. **✅ Network topology** is established with all 3 validators connected
4. **⚠️ No blocks produced** (expected - validators wait for mining events)

### What You'll See in the Logs

Successful validator startup includes:
- Network configuration loaded with static validators
- Listening on configured port (10301, 10302, or 10303)
- Bootstrap connections established
- Peer information exchanged (Identify protocol)
- Ping/pong messages between validators

## Network Configuration

### Ports
- **Validator 1**: `10301` (WebSocket)
- **Validator 2**: `10302` (WebSocket)
- **Validator 3**: `10303` (WebSocket)

### Bootstrap Configuration
Each validator bootstraps from the other two validators:
- Validator 1 → connects to validators 2 and 3
- Validator 2 → connects to validators 1 and 3
- Validator 3 → connects to validators 1 and 2

### Storage
Each validator stores its data in:
- `examples/network/03-run-devnet3/tmp/node{1,2,3}/storage/`

Storage is cleared on each run using `modal node clear-storage --yes`.

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Validator 1 │────▶│ Validator 2 │────▶│ Validator 3 │
│   :10301    │◀────│   :10302    │◀────│   :10303    │
└─────────────┘     └─────────────┘     └─────────────┘
       ▲                                       │
       └───────────────────────────────────────┘
          All validators form mesh topology
```

## Files

- `01-run-node1.sh` - Starts validator 1
- `02-run-node2.sh` - Starts validator 2
- `03-run-node3.sh` - Starts validator 3
- `test.sh` - Automated test that runs all validators
- `tmp/` - Runtime data (created automatically, gitignored)

## Configuration Files

The validators use configurations from the `fixtures/` directory:

**Network Config:**
- `fixtures/network-configs/devnet3/config.json` - Network-wide configuration with static validator list

**Node Configs:**
- `fixtures/network-node-configs/devnet3/node1.json` - Validator 1 configuration
- `fixtures/network-node-configs/devnet3/node2.json` - Validator 2 configuration
- `fixtures/network-node-configs/devnet3/node3.json` - Validator 3 configuration

**Passfiles:**
- `fixtures/passfiles/node1.mod_passfile` - Identity for validator 1
- `fixtures/passfiles/node2.mod_passfile` - Identity for validator 2
- `fixtures/passfiles/node3.mod_passfile` - Identity for validator 3

## Troubleshooting

### Validators Don't Connect

**Issue:** Validators start but don't connect to each other

**Solution:**
1. Ensure all 3 validators are running
2. Check that ports 10301, 10302, and 10303 are not in use
3. Verify bootstrap addresses in node configs match running validators
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

1. **Static Validators**: Production uses dynamic validator selection from mining epochs
2. **Local Networking**: All validators run on localhost (production uses public IPs)
3. **No Mining**: No mining activity (production has miners creating blocks)
4. **No Consensus**: Current implementation doesn't run active consensus rounds
5. **Genesis Round Only**: Only the pre-configured genesis round exists

## Use Cases

This example is useful for:
- **Testing validator connectivity** with 3 nodes
- **Verifying network topology** formation
- **Debugging peer discovery** mechanisms
- **Testing static validator** configurations
- **Development environment** setup for consensus work

## Next Steps

For active mining and block production, see:
- `examples/network/05-mining/` - Mining with dynamic validator selection
- `examples/network/04-sync-miner-blocks/` - Block synchronization between nodes

## Related Examples

- `02-run-devnet2/` - Simpler 2-validator setup
- `06-static-validators/` - Detailed static validator example with utilities
- `05-mining/` - Mining with dynamic validators

