# Network Filtering Feature

## Overview

Added `--network` filter option to both `modal local nodes` and `modal local killall-nodes` commands, enabling selective operations on nodes based on their network configuration.

## Features

### Network Filter Option

**Flag:** `--network <FILTER>`

**Supports:**
- Exact matches: `--network "devnet3"`
- Wildcard patterns: `--network "devnet*"` (matches devnet1, devnet2, devnet3, etc.)
- Any network name from `network_config_path` in node config

### How It Works

1. **Extracts network name** from `network_config_path` in config.json
   - Example: `"modal-networks://devnet3"` → `"devnet3"`
   
2. **Matches against filter**
   - Exact: `"devnet3"` matches only "devnet3"
   - Wildcard: `"devnet*"` matches "devnet1", "devnet2", "devnet3", etc.
   
3. **Filters nodes** before display or kill operations

## Usage Examples

### List Nodes by Network

```bash
# Show all nodes
modal local nodes

# Show only devnet nodes
modal local nodes --network "devnet*"

# Show only devnet3 nodes
modal local nodes --network "devnet3"

# Show testnet nodes
modal local nodes --network "testnet*"
```

### Kill Nodes by Network

```bash
# Kill all devnet nodes
modal local killall-nodes --network "devnet*"

# Kill only devnet3 nodes (with force)
modal local killall-nodes --network "devnet3" --force

# Dry-run first, then kill testnet nodes
modal local killall-nodes --network "testnet*" --dry-run
modal local killall-nodes --network "testnet*"
```

## Use Cases

### 1. Multi-Network Development

```bash
# Running nodes on different networks simultaneously
modal node run-validator --dir ./devnet1-node &  # devnet1
modal node run-validator --dir ./devnet3-node &  # devnet3
modal node run-validator --dir ./testnet-node &  # testnet

# Check only devnet nodes
modal local nodes --network "devnet*"

# Kill only testnet, keep devnets running
modal local killall-nodes --network "testnet*"
```

### 2. Selective Cleanup

```bash
# Clean up test networks but keep production
modal local killall-nodes --network "devnet*" --force
modal local killall-nodes --network "testnet*" --force
# Production nodes (mainnet) still running
```

### 3. Network-Specific Operations

```bash
# Check if any devnet nodes are running
if modal local nodes --network "devnet*" | grep -q "PID:"; then
    echo "Devnet nodes detected"
    # Perform devnet-specific operations
fi
```

### 4. Restart Specific Network

```bash
#!/bin/bash
# Restart all devnet3 nodes

echo "Stopping devnet3 nodes..."
modal local killall-nodes --network "devnet3"

sleep 2

echo "Starting fresh devnet3 nodes..."
./scripts/start-devnet3.sh
```

### 5. CI/CD Network Isolation

```bash
# In CI pipeline - clean up only test networks
cleanup_test_networks() {
    modal local killall-nodes --network "devnet*" --force || true
    modal local killall-nodes --network "testnet-ci*" --force || true
}

# Keep other networks running for parallel tests
```

## Output Examples

### Filtered List

```bash
$ modal local nodes --network "devnet*"
Running Modal Nodes:
================================================================================

PID: 12345
Directory: ./tmp/devnet1-node
Peer ID: 12D3KooW...
Network: modal-networks://devnet1
Listening addresses:
  • /ip4/0.0.0.0/tcp/10101/ws/p2p/12D3KooW...

PID: 12346
Directory: ./tmp/devnet3-node
Peer ID: 12D3KooWTest...
Network: modal-networks://devnet3
Listening addresses:
  • /ip4/0.0.0.0/tcp/10102/ws/p2p/12D3KooWTest...

Found 2 running node(s)
```

### No Matches

```bash
$ modal local nodes --network "mainnet*"
No running modal nodes found matching network filter.
```

### Filtered Kill

```bash
$ modal local killall-nodes --network "devnet*" --dry-run
Found 2 running node(s)

DRY RUN - would kill the following nodes:

  PID 12345: ./tmp/devnet1-node
  PID 12346: ./tmp/devnet3-node
```

## Implementation Details

### Files Modified

1. **rust/modal/src/cmds/local/nodes.rs**
   - Added `network: Option<String>` to `Opts`
   - Added `network_config: Option<String>` to `NodeInfo`
   - Implemented `filter_nodes_by_network()` function
   - Implemented `matches_network_filter()` with wildcard support
   - Updated `get_node_info_from_dir()` to extract `network_config_path`
   - Updated `run()` to apply filter before displaying

2. **rust/modal/src/cmds/local/killall_nodes.rs**
   - Added `network: Option<String>` to `Opts`
   - Updated `run()` to apply filter before killing

### Filter Logic

```rust
pub fn filter_nodes_by_network(nodes: Vec<NodeInfo>, filter: &str) -> Vec<NodeInfo> {
    nodes.into_iter()
        .filter(|node| {
            if let Some(network_config) = &node.network_config {
                matches_network_filter(network_config, filter)
            } else {
                false  // Exclude nodes without network config
            }
        })
        .collect()
}

fn matches_network_filter(network_config: &str, filter: &str) -> bool {
    // Extract: "modal-networks://devnet3" -> "devnet3"
    let network_name = network_config
        .strip_prefix("modal-networks://")
        .unwrap_or(network_config);
    
    // Wildcard matching
    if filter.ends_with('*') {
        let prefix = filter.trim_end_matches('*');
        network_name.starts_with(prefix)
    } else {
        network_name == filter  // Exact match
    }
}
```

## Benefits

1. **Selective Operations**: Target specific networks without affecting others
2. **Multi-Network Development**: Run multiple networks simultaneously and manage them independently
3. **Safety**: Prevent accidental kill of production nodes when cleaning up test networks
4. **Efficiency**: No need to kill all nodes when you only want to restart one network
5. **CI/CD Friendly**: Easy to isolate test networks in automated pipelines

## Pattern Matching

### Supported Patterns

| Pattern | Matches | Example |
|---------|---------|---------|
| `devnet*` | All devnet networks | devnet1, devnet2, devnet3 |
| `devnet3` | Exact match | devnet3 only |
| `testnet*` | All testnet networks | testnet, testnet-ci, testnet-staging |
| `mainnet` | Exact match | mainnet only |

### Future Enhancements

Potential additions:
- Multiple filters: `--network "devnet*" --network "testnet*"`
- Exclude patterns: `--exclude-network "devnet1"`
- Regex support: `--network-regex "devnet[13]"`
- Network groups: `--network-group "test"` (matches all test networks)

## Testing

Test the feature:

```bash
# Start nodes on different networks
cd /tmp/test-network-filter
mkdir -p devnet1 devnet3 testnet

# Create and start devnet1 node
cd devnet1
modal node create --type validator --port 11001
# Edit config.json to set network_config_path to "modal-networks://devnet1"
modal node run-validator --dir . &

# Create and start devnet3 node
cd ../devnet3
modal node create --type validator --port 11003
# Edit config.json to set network_config_path to "modal-networks://devnet3"
modal node run-validator --dir . &

# Create and start testnet node
cd ../testnet
modal node create --type validator --port 12001
# Edit config.json to set network_config_path to "modal-networks://testnet"
modal node run-validator --dir . &

# Test filtering
modal local nodes  # Should show all 3
modal local nodes --network "devnet*"  # Should show 2
modal local nodes --network "testnet*"  # Should show 1
modal local nodes --network "devnet3"  # Should show 1

# Test kill filtering
modal local killall-nodes --network "devnet*" --dry-run  # Preview
modal local killall-nodes --network "devnet*"  # Kill only devnet nodes
modal local nodes  # Should show only testnet node

# Cleanup
modal local killall-nodes
```

## Documentation

Updated:
- `docs/node-management-commands.md` - Added `--network` option to both commands
- Added network filtering examples
- Added network-specific use cases

## Related Commands

- `modal local nodes` - Now with network filtering
- `modal local killall-nodes` - Now with network filtering
- `modal node info` - Shows network config of a single node
- `modal node inspect` - Can inspect nodes filtered by network

## Backward Compatibility

✅ Fully backward compatible:
- `--network` is optional
- Without the flag, commands work exactly as before (show/kill all nodes)
- No breaking changes to existing scripts

