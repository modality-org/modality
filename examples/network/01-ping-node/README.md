# Example: Ping Node

This example demonstrates how to create nodes using `modal node create` and test basic network connectivity with the ping command.

## Overview

- **Node 1**: Created using `modal node create` with **standard devnet1/node1 identity** (peer ID: `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`)
- **Node 2**: Created using `modal node create` with a new identity
- **Port**: Node1 uses standard port 10101 (matches devnet1 configuration)
- **Test**: Node2 pings Node1 to verify connectivity

## Prerequisites

1. Build the `modal` CLI:
   ```bash
   cd ../../../rust
   cargo build --package modal
   ```

## Running the Example

### Step 1: Start Node 1

In one terminal:

```bash
./01-run-node1.sh
```

This will:
1. Create a new node directory at `./tmp/node1` (if it doesn't exist)
   - Uses the `devnet1/node1` template from `modal-networks` package
   - Automatically loads both passfile and config
   - Creates storage and logs directories
2. Clear storage (for clean test runs)
3. Start the node on port 10101

The `modal node create --from-template` command:
- Loads pre-configured node templates from the `modal-networks` package
- Automatically includes both passfile and config
- No manual file copying or path references needed

**Node1 Identity:**
- Peer ID: `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`
- Port: 10101
- This matches the standard devnet1/node1 configuration

**What gets created:**
```
tmp/node1/
├── config.json       # Node configuration
├── node.passfile     # Node identity (keep this secure!)
├── storage/          # RocksDB datastore
└── logs/             # Node logs
```

### Step 2: Ping Node 1 from Node 2

In another terminal:

```bash
./02-ping-node1-from-node2.sh
```

This will:
1. Create node2 at `./tmp/node2` (if it doesn't exist) with a new identity
2. Use the standard node1 peer ID (12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd)
3. Ping node1 on port 10101 (100 times)

**Expected Output:**
```
Creating node2...
✨ Successfully created new node directory!
📁 Node directory: ./tmp/node2
🆔 Node ID: 12D3KooW...
...
Ping successful: node=12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd time=15ms
Ping successful: node=12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd time=12ms
...
```

## Using `modal node create`

The `modal node create` command sets up a complete node directory with all necessary files:

### Basic Usage

```bash
# Create with network preset (devnet1, devnet2, devnet3, testnet)
modal node create --dir ./my-node --network devnet1

# Create for testnet (includes autoupgrade)
modal node create --dir ./my-node --testnet

# Create with custom bootstrappers
modal node create --dir ./my-node --bootstrappers "/ip4/1.2.3.4/tcp/4040/ws/p2p/12D3..."
```

### What It Creates

1. **`config.json`** - Node configuration including:
   - Node ID (peer ID)
   - Storage and logs paths
   - Network bootstrappers
   - Logging configuration
   - Autoupgrade settings (if enabled)

2. **`node.passfile`** - Node identity (keypair)
   - Keep this file secure!
   - Never share it or commit it to version control
   - This determines your node's peer ID

3. **`storage/`** - RocksDB datastore directory
   - Stores blocks, state, and other data

4. **`logs/`** - Log files directory
   - Node operation logs

### Advanced Options

```bash
# Use BIP39 mnemonic for key derivation
modal node create --dir ./my-node --use-mnemonic --network devnet1

# Custom storage path
modal node create --dir ./my-node --storage-path ./data --network devnet1

# Disable logging
modal node create --dir ./my-node --logs-enabled false --network devnet1

# Enable autoupgrade for development network
modal node create --dir ./my-node --network devnet1 --enable-autoupgrade
```

## Using Node Templates

This example uses the **`--from-template`** option to load a pre-configured node from the `modal-networks` package.

### Simple Usage

```bash
# Create node with standard devnet1/node1 identity and configuration
modal node create --dir ./tmp/node1 --from-template devnet1/node1
```

**What this does:**
1. **Loads template** from `modal-networks` package (embedded in the binary)
2. **Imports passfile** ensuring the node has the standard peer ID
3. **Imports configuration** (listeners, bootstrappers, storage path, etc.)
4. Creates the complete node directory with all necessary files

### Available Templates

List all available templates:
```bash
modal node create --from-template INVALID 2>&1 | grep "Available templates"
```

Current templates:
- `devnet1/node1` - Standard devnet1/node1 configuration

### How Templates Work

Templates are stored in the `modal-networks` Rust package at `rust/modal-networks/templates/`:

```
modal-networks/
└── templates/
    └── devnet1/
        └── node1/
            ├── node.passfile  # Standard devnet1/node1 identity
            └── config.json    # Standard devnet1/node1 configuration
```

These are embedded into the binary at compile time using `include_str!`, so no external files are needed.

### Benefits

- ✅ Consistent peer ID (`12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`)
- ✅ Standard port (10101) from the template
- ✅ All devnet1 bootstrappers and settings
- ✅ Compatible with other devnet1 configurations
- ✅ Reproducible for testing
- ✅ No manual file copying or path references
- ✅ Works anywhere the binary is installed
- ✅ Templates are versioned with the code

### Advanced: Manual Import

You can still use `--from-passfile` and `--from-config` for custom setups:

```bash
modal node create \
    --dir ./tmp/node1 \
    --from-passfile ../../../fixtures/passfiles/node1.mod_passfile \
    --from-config ../../../fixtures/network-node-configs/devnet1/node1.json
```

## Node Info Command

View information about a node:

```bash
modal node info --dir ./tmp/node1
```

Output:
```
Node ID: 12D3KooW...
Storage: ./storage
Config: ./config.json
Passfile: ./node.passfile
```

## Ping Command

Test connectivity between nodes:

```bash
# Ping node1 from node2 using standard peer ID and port
modal node ping \
  --dir ./tmp/node2 \
  --target /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd \
  --times 10

# Or from within the node directory
cd ./tmp/node2
modal node ping \
  --target /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd \
  --times 10
```

## Network Presets

The `--network` flag loads bootstrappers from fixture files:

- **devnet1**: Single-node development network
- **devnet2**: Two-node development network  
- **devnet3**: Three-node development network
- **testnet**: Public test network (use `--testnet` flag instead)

For testnet:
```bash
modal node create --dir ./my-node --testnet
```

## Troubleshooting

### "config.json already exists"

The directory already has a node. Either:
- Use a different directory: `--dir ./tmp/node-new`
- Or remove the existing node: `rm -rf ./tmp/node1`

### "Ping failed: connection timeout"

- Ensure node1 is running (`./01-run-node1.sh`)
- Check node1 is listening on port 4040: `lsof -i :4040`
- Verify the peer ID matches node1's ID: `modal node info --dir ./tmp/node1`

### "Port already in use"

Another process is using port 10101. Either:
- Kill the existing process: `kill $(lsof -t -i:10101)`
- Or edit `tmp/node1/config.json` to use a different port

## Script Reference

| Script | Description |
|--------|-------------|
| `01-run-node1.sh` | Create (if needed) and start node1 |
| `02-ping-node1-from-node2.sh` | Create node2 (if needed) and ping node1 |
| `test.sh` | Automated integration test |

## Architecture

```
┌──────────────────────────────────────┐
│  02-ping-node1-from-node2.sh         │
│                                      │
│  1. modal node create --dir node2    │
│  2. Get node1's peer ID              │
│  3. modal node ping --target node1   │
└──────────────────────────────────────┘
              │
              ▼
        Ping Request
              │
              ▼
┌──────────────────────────────────────┐
│  01-run-node1.sh                     │
│                                      │
│  1. modal node create --dir node1    │
│  2. modal node run --dir node1       │
│                                      │
│  Listening on: 0.0.0.0:4040          │
└──────────────────────────────────────┘
```

## Benefits of `modal node create`

1. **Self-contained**: Each node has its own directory with all files
2. **Portable**: Can easily copy/backup entire node directory
3. **Version controlled**: Can commit node configs (but not passfiles!)
4. **Reproducible**: Same command creates identical structure
5. **Discoverable**: `modal node info` shows all node details
6. **Flexible**: Easy to create multiple nodes for testing

## Security Notes

⚠️ **IMPORTANT**: The `node.passfile` contains your node's private key!

- **DO NOT** commit passfiles to version control
- **DO NOT** share passfiles with others
- **DO** keep backups in secure locations
- **DO** use different passfiles for different environments

The `.gitignore` already excludes `*/tmp/` so these test nodes won't be committed.

## Related Documentation

- [Modal Node Create Command](../../../rust/modal/src/cmds/node/create.rs)
- [Modal Node Run Command](../../../rust/modal/src/cmds/node/run.rs)
- [Modal Node Ping Command](../../../rust/modal/src/cmds/node/ping.rs)
- [Network Configuration](../../../fixtures/network-configs/)

## See Also

- Example 02: `02-run-devnet2` - Two-node network
- Example 03: `03-run-devnet3` - Three-node network
- Example 04: `04-sync-miner-blocks` - Block synchronization

