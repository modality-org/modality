# Node Management Commands

This document describes the new node management commands added to the `modal` CLI.

## Overview

The following commands have been added to improve node process management and discovery:

1. `modal node kill` - Kill a running node process
2. `modal node pid` - Display the PID of a running node
3. `modal node address` - Display listening addresses of a node
4. `modal local nodes` - Find all running modal node processes
5. `modal local killall-nodes` - Kill all running modal node processes

## Commands

### `modal node kill`

Kill a running modal node process.

**Usage:**
```bash
modal node kill [OPTIONS]
```

**Options:**
- `--config <CONFIG>` - Path to node configuration file
- `--dir <DIR>` - Node directory containing config.json (defaults to current directory)
- `--force`, `-f` - Force kill (SIGKILL) instead of graceful shutdown (SIGTERM)

**Examples:**
```bash
# Kill node in current directory (graceful)
modal node kill --dir .

# Force kill a specific node
modal node kill --dir ./tmp/node1 --force

# Kill using config file
modal node kill --config ./node1/config.json
```

**How it works:**
- Reads the PID from `node.pid` file in the node directory
- Sends SIGTERM for graceful shutdown (default) or SIGKILL for force kill
- Cleans up the PID file after killing the process
- Handles stale PID files gracefully

---

### `modal node pid`

Display the PID of a running node.

**Usage:**
```bash
modal node pid [OPTIONS]
```

**Options:**
- `--config <CONFIG>` - Path to node configuration file
- `--dir <DIR>` - Node directory containing config.json (defaults to current directory)

**Examples:**
```bash
# Get PID of node in current directory
modal node pid --dir .

# Use in a script
NODE_PID=$(modal node pid --dir ./tmp/node1)
echo "Node is running with PID: $NODE_PID"

# Check if node is running
if modal node pid --dir ./tmp/node1 > /dev/null 2>&1; then
    echo "Node is running"
else
    echo "Node is not running"
fi

# Kill using the PID command
kill $(modal node pid --dir ./tmp/node1)
```

**How it works:**
- Reads the PID from `node.pid` file
- Verifies the process is actually running (on Unix systems)
- Outputs only the PID number (useful for scripting)
- Returns error if process is not running

---

### `modal node address`

Display the listening addresses of a node in multiaddr format.

**Usage:**
```bash
modal node address [OPTIONS]
```

**Options:**
- `--config <CONFIG>` - Path to node configuration file
- `--dir <DIR>` - Node directory containing config.json (defaults to current directory)
- `-1`, `--one` - Show only one address
- `--prefer-local` - Prefer local/loopback IP addresses (127.0.0.1, ::1)
- `--prefer-public` - Prefer public IP addresses

**Examples:**
```bash
# Show all listening addresses
modal node address --dir ./tmp/node1

# Get one local address for same-machine connections
LOCAL_ADDR=$(modal node address --dir ./tmp/node1 --prefer-local --one)
echo "Connect locally using: $LOCAL_ADDR"

# Get one public address for remote connections
PUBLIC_ADDR=$(modal node address --dir ./tmp/node1 --prefer-public --one)
echo "Connect remotely using: $PUBLIC_ADDR"

# Use in a ping command
modal node ping $(modal node address --dir ./tmp/node1 --one)

# List all addresses
modal node address --dir ./tmp/node1 | while read addr; do
    echo "Available at: $addr"
done
```

**Output format:**
- One multiaddr per line
- Includes `/p2p/<peer_id>` suffix for complete addressing
- Example: `/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW...`

**How it works:**
- Reads the node config to get listeners and peer ID
- Constructs complete multiaddrs with peer ID suffix
- Sorts addresses based on preference flags
- Outputs addresses that can be used directly with `modal node ping`

---

### `modal local nodes`

Find and display all running modal node processes on the system.

**Usage:**
```bash
modal local nodes [OPTIONS]
```

**Options:**
- `--verbose`, `-v` - Show verbose output with full paths

**Examples:**
```bash
# Find all running nodes
modal local nodes

# Show full paths
modal local nodes --verbose

# Count running nodes
modal local nodes | grep "^PID:" | wc -l

# Find a specific node
modal local nodes | grep "node1"
```

**Output:**
```
Running Modal Nodes:
================================================================================

PID: 12345
Directory: ./tmp/node1
Peer ID: 12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd
Listening addresses:
  • /ip4/0.0.0.0/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd

PID: 12346
Directory: ./tmp/node2
Peer ID: 12D3KooWTest456789012345678901234567890123456789012
Listening addresses:
  • /ip4/0.0.0.0/tcp/10102/ws/p2p/12D3KooWTest456789012345678901234567890123456789012

Found 2 running node(s)
```

**How it works:**
- Uses `pgrep` to find modal processes running with `run` commands
- Scans common directories for `node.pid` files
- Reads config.json from discovered node directories
- Displays PID, directory, peer ID, and listening addresses
- Deduplicates nodes found through multiple methods

**Discovery methods:**
1. Process scanning - Finds all `modal` processes with `run` in command line
2. PID file scanning - Searches for `node.pid` files in:
   - Current directory
   - `./tmp` and subdirectories
   - `../tmp` and subdirectories
   - `../../tmp` and subdirectories

---

### `modal local killall-nodes`

Kill all running modal node processes discovered on the system.

**Usage:**
```bash
modal local killall-nodes [OPTIONS]
```

**Options:**
- `--force`, `-f` - Force kill (SIGKILL) instead of graceful shutdown (SIGTERM)
- `--dry-run` - Show what would be killed without actually killing

**Examples:**
```bash
# Kill all nodes gracefully
modal local killall-nodes

# Force kill all nodes
modal local killall-nodes --force

# Preview what would be killed without doing it
modal local killall-nodes --dry-run

# Use in cleanup scripts
cleanup() {
    echo "Stopping all test nodes..."
    modal local killall-nodes --force
}
trap cleanup EXIT
```

**Output:**
```
Found 3 running node(s)

Killing all nodes with SIGTERM...

Killing PID 12345 (./tmp/node1)... ✓
Killing PID 12346 (./tmp/node2)... ✓
Killing PID 12347 (./tmp/node3)... ✓

Summary:
  Killed: 3
```

**How it works:**
- Uses the same discovery mechanism as `modal local nodes`
- Sends SIGTERM (graceful) or SIGKILL (force) to each process
- Cleans up PID files automatically
- Handles stale processes gracefully
- Shows summary of operations

**Safety Features:**
- Dry-run mode for testing
- Clear output showing what's being killed
- Graceful shutdown by default
- Automatic cleanup of PID files

---

## Implementation Details

### PID File Management

All node run commands (`run-validator`, `run-miner`, `run-observer`) now automatically:
1. Create a `node.pid` file when starting
2. Remove the `node.pid` file when exiting gracefully

The PID file is located at `<node_directory>/node.pid` and contains only the process ID as a plain integer.

### Platform Support

These commands are fully supported on Unix-like systems (Linux, macOS). On other platforms:
- `modal node kill` falls back to basic process termination
- `modal node pid` works but without process verification
- `modal nodes` has limited discovery capabilities

### Integration with Existing Scripts

These commands can be integrated into existing shell scripts to replace manual PID tracking:

**Before:**
```bash
# Old approach
modal node run-validator --dir ./tmp/node1 > node1.log 2>&1 &
echo $! > ./tmp/node1.pid

# Later...
kill $(cat ./tmp/node1.pid)
rm ./tmp/node1.pid
```

**After:**
```bash
# New approach - PID file managed automatically
modal node run-validator --dir ./tmp/node1 > node1.log 2>&1 &

# Later...
modal node kill --dir ./tmp/node1
```

## Use Cases

### 1. Development and Testing
```bash
# Start multiple nodes
modal node run-validator --dir ./node1 &
modal node run-validator --dir ./node2 &
modal node run-validator --dir ./node3 &

# Check what's running
modal local nodes

# Get address of a node to connect another peer
BOOTSTRAP=$(modal node address --dir ./node1 --one)
modal node run-validator --dir ./node4 --bootstrapper "$BOOTSTRAP" &

# Clean up everything at once
modal local killall-nodes
```

### 2. Monitoring Scripts
```bash
#!/bin/bash
# Monitor node health

if ! modal node pid --dir ./production-node > /dev/null 2>&1; then
    echo "ERROR: Production node is not running!"
    # Restart logic here
fi
```

### 3. Network Testing
```bash
# Find all nodes and test connectivity
modal local nodes | grep "^  •" | while read -r _ addr; do
    echo "Testing connection to $addr"
    modal node ping "$addr" || echo "  Failed to ping"
done
```

### 4. CI/CD Integration
```bash
# In test scripts
cleanup() {
    echo "Cleaning up nodes..."
    modal local killall-nodes --force || true
}
trap cleanup EXIT

# Test execution...
```

### 5. Quick Reset During Development
```bash
# Nuclear option - kill everything and start fresh
modal local killall-nodes --force
rm -rf ./tmp/node*

# Start clean nodes
./setup-test-network.sh
```

## See Also

- `modal node info` - Display detailed information about a node (read-only mode)
- `modal node inspect` - Inspect a node's state (works on running nodes)
- `modal node run-validator` - Run a validator node
- `modal node run-miner` - Run a mining node
- `modal node run-observer` - Run an observer node

