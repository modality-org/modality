# Node Management Commands Implementation Summary

## New Commands Implemented

### 1. `modal node kill`
**File:** `rust/modal/src/cmds/node/kill.rs`

Kills a running modal node process by reading the PID from the `node.pid` file and sending SIGTERM (graceful) or SIGKILL (force) signal.

**Features:**
- Graceful shutdown with SIGTERM (default)
- Force kill with `--force` flag
- Automatic PID file cleanup
- Handles stale PID files

---

### 2. `modal node pid`
**File:** `rust/modal/src/cmds/node/pid.rs`

Displays the PID of a running node, reading from the `node.pid` file and verifying the process is actually running.

**Features:**
- Simple PID output (scriptable)
- Process verification on Unix systems
- Error handling for stale PID files

---

### 3. `modal node address`
**File:** `rust/modal/src/cmds/node/address.rs`

Displays the listening addresses of a node in multiaddr format with peer ID appended.

**Features:**
- Lists all configured listeners with `/p2p/<peer_id>` suffix
- `--one` flag to show only one address
- `--prefer-local` to prioritize loopback addresses (127.0.0.1, ::1)
- `--prefer-public` to prioritize public IP addresses
- Output format ready for use with `modal node ping`

---

### 4. `modal local nodes`
**File:** `rust/modal/src/cmds/nodes.rs`

Discovers and displays all running modal node processes on the system.

**Features:**
- Process scanning using `pgrep`
- PID file discovery in common directories
- Displays PID, directory, peer ID, and listening addresses
- `--verbose` flag for full paths
- Deduplicates nodes found through multiple methods

**Discovery Strategy:**
1. Scans for processes matching "modal.*run" pattern
2. Searches for `node.pid` files in:
   - Current directory
   - `./tmp` and subdirectories
   - `../tmp` and subdirectories
   - `../../tmp` and subdirectories

---

## Supporting Infrastructure

### PID File Management
**Files:**
- `rust/modal-node/src/pid.rs` - PID file utilities
- `rust/modal-node/src/lib.rs` - Module export

**Functions:**
- `write_pid_file(node_dir)` - Creates `node.pid` file with current process ID
- `read_pid_file(node_dir)` - Reads PID from file
- `remove_pid_file(node_dir)` - Deletes PID file
- `get_pid_file_path(node_dir)` - Returns path to PID file

**Integration:**
All node run commands now automatically manage PID files:
- `rust/modal/src/cmds/node/run_validator.rs`
- `rust/modal/src/cmds/node/run_miner.rs`
- `rust/modal/src/cmds/node/run_observer.rs`

Each command calls `write_pid_file()` on startup and `remove_pid_file()` on shutdown.

---

## Dependencies

### Added to `rust/modal/Cargo.toml`:
```toml
[target.'cfg(unix)'.dependencies]
nix = { version = "0.29", features = ["signal"] }
```

Used for Unix signal handling (SIGTERM, SIGKILL) and process verification.

---

## Script Updates

### Updated Example Scripts
Several example scripts were updated to use `modal node kill` instead of manual PID management:

1. **examples/network/08-network-partition/**
   - `05-partition-single-node.sh`
   - `06-partition-two-nodes.sh`

2. **examples/network/07-contract-assets/**
   - `07-stop-validator.sh`

3. **examples/network/js-sdk/**
   - `02-stop-devnet1.sh`
   - `test.sh`

**Pattern:**
```bash
# Prefer modal node kill if available
if command -v modal >/dev/null 2>&1 && [ -d "./tmp/node1" ]; then
    modal node kill --dir ./tmp/node1
else
    # Fallback to PID-based killing
    kill $(cat ./tmp/node1.pid)
fi
```

---

## Testing

### Test Script
**File:** `examples/network/test-nodes-command.sh`

Comprehensive test script that:
1. Starts multiple test nodes
2. Tests `modal nodes` discovery
3. Tests `modal nodes --verbose`
4. Cleans up test nodes

---

## Documentation

### New Documentation Files

1. **docs/node-management-commands.md**
   - Complete reference for all four new commands
   - Usage examples
   - Use cases (development, monitoring, CI/CD)
   - Integration examples

2. **examples/network/test-nodes-command.sh**
   - Executable test/demo script

---

## Read-Only Database Access

As part of this work, several commands were updated to use read-only database access to prevent locking issues on running nodes:

### Updated Commands:
- `modal node info` - Uses `NetworkDatastore::create_in_directory_readonly()`
- `modal node inspect` - Uses `NetworkDatastore::create_in_directory_readonly()`

### Implementation:
**File:** `rust/modal-datastore/src/network_datastore.rs`

Added `create_in_directory_readonly()` method that opens RocksDB in read-only mode, allowing multiple readers without exclusive locks.

---

## Key Benefits

1. **Simplified Process Management**
   - No need to manually track PIDs in scripts
   - Automatic PID file management
   - Clean process termination

2. **Better Observability**
   - `modal nodes` provides system-wide view of running nodes
   - Easy to see all listening addresses
   - Scriptable output for automation

3. **Improved Developer Experience**
   - Commands designed for shell scripting
   - Clean, parseable output
   - Consistent interface across all node operations

4. **Safer Operations**
   - Graceful shutdown by default
   - Stale PID file handling
   - Process verification before operations

---

## Example Workflows

### Development Workflow
```bash
# Start nodes
modal node run-validator --dir ./node1 &
modal node run-validator --dir ./node2 &

# Check what's running
modal local nodes

# Get connection address
ADDR=$(modal node address --dir ./node1 --one)
echo "Connect to: $ADDR"

# Clean up
modal node kill --dir ./node1
modal node kill --dir ./node2
```

### Monitoring Script
```bash
#!/bin/bash
while true; do
    if ! modal node pid --dir ./production-node >/dev/null 2>&1; then
        echo "Node down! Restarting..."
        modal node run-validator --dir ./production-node &
    fi
    sleep 60
done
```

### Network Testing
```bash
# Test connectivity to all running nodes
modal local nodes | grep "^  •" | while read _ addr; do
    echo "Testing $addr"
    modal node ping "$addr"
done
```

---

## Files Changed

### New Files (9):
1. `rust/modal/src/cmds/node/kill.rs`
2. `rust/modal/src/cmds/node/pid.rs`
3. `rust/modal/src/cmds/node/address.rs`
4. `rust/modal/src/cmds/nodes.rs`
5. `rust/modal-node/src/pid.rs`
6. `docs/node-management-commands.md`
7. `examples/network/test-nodes-command.sh`

### Modified Files (15):
1. `rust/modal/src/main.rs`
2. `rust/modal/src/cmds/mod.rs`
3. `rust/modal/src/cmds/node/mod.rs`
4. `rust/modal/Cargo.toml`
5. `rust/modal-node/src/lib.rs`
6. `rust/modal/src/cmds/node/run_validator.rs`
7. `rust/modal/src/cmds/node/run_miner.rs`
8. `rust/modal/src/cmds/node/run_observer.rs`
9. `rust/modal-datastore/src/network_datastore.rs`
10. `rust/modal/src/cmds/node/info.rs`
11. `rust/modal/src/cmds/node/inspect.rs`
12. `examples/network/08-network-partition/05-partition-single-node.sh`
13. `examples/network/08-network-partition/06-partition-two-nodes.sh`
14. `examples/network/07-contract-assets/07-stop-validator.sh`
15. `examples/network/js-sdk/02-stop-devnet1.sh`
16. `examples/network/js-sdk/test.sh`

---

## Platform Compatibility

**Fully Supported:**
- Linux (all distributions)
- macOS

**Partial Support:**
- Windows and other platforms (basic functionality, limited discovery)

**Unix-Specific Features:**
- Signal handling (SIGTERM, SIGKILL)
- Process verification
- `pgrep` and `lsof` for process discovery

---

## Next Steps

1. Run the test script to verify all commands work:
   ```bash
   bash examples/network/test-nodes-command.sh
   ```

2. Update existing example scripts to use new commands

3. Consider adding:
   - `modal nodes kill-all` - Kill all discovered nodes
   - `modal node restart` - Restart a running node
   - `modal node logs` - Tail logs of a running node
   - `modal node status` - Show running/stopped status

4. Potential improvements:
   - Support for custom search paths in `modal local nodes`
   - JSON output format for machine parsing
   - Filter by node type (miner/validator/observer)
   - Show additional metrics (uptime, memory usage, etc.)
   - Add more local development commands (e.g., `modal local clean`, `modal local restart-all`)

