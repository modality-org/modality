# `modal local killall-nodes` Command

## Overview

The `modal local killall-nodes` command provides a convenient way to kill all running modal node processes discovered on the local system in a single operation.

## Features

### Core Functionality
- **Auto-discovery**: Finds all running modal nodes using the same mechanism as `modal local nodes`
- **Graceful shutdown**: Uses SIGTERM by default for clean process termination
- **Force kill**: Optional SIGKILL for immediate termination
- **Dry-run mode**: Preview what would be killed without actually doing it
- **Automatic cleanup**: Removes PID files after killing processes
- **Error handling**: Gracefully handles stale processes and errors

### Command Options

```bash
modal local killall-nodes [OPTIONS]
```

**Options:**
- `--force`, `-f` - Force kill with SIGKILL instead of graceful SIGTERM
- `--dry-run` - Show what would be killed without executing

## Usage Examples

### Basic Usage

```bash
# Kill all running nodes gracefully
modal local killall-nodes

# Force kill all nodes immediately
modal local killall-nodes --force

# Preview without killing
modal local killall-nodes --dry-run
```

### Development Workflow

```bash
# Quick cleanup after testing
modal local killall-nodes

# Nuclear reset - kill everything and clean directories
modal local killall-nodes --force
rm -rf ./tmp/node*
```

### CI/CD Integration

```bash
#!/bin/bash
# Test script with automatic cleanup

cleanup() {
    echo "Cleaning up all test nodes..."
    modal local killall-nodes --force || true
}
trap cleanup EXIT INT TERM

# Run tests...
./run-network-tests.sh
```

### Safe Experimentation

```bash
# Check what's running
modal local nodes

# Dry run to see what would be killed
modal local killall-nodes --dry-run

# Actually kill them
modal local killall-nodes
```

## Output Format

### Normal Operation

```
Found 3 running node(s)

Killing all nodes with SIGTERM...

Killing PID 12345 (./tmp/node1)... ✓
Killing PID 12346 (./tmp/node2)... ✓
Killing PID 12347 (./tmp/node3)... ✓

Summary:
  Killed: 3
```

### With Errors

```
Found 3 running node(s)

Killing all nodes with SIGTERM...

Killing PID 12345 (./tmp/node1)... ✓
Killing PID 12346 (./tmp/node2)... ⚠️  Process not running (stale)
Killing PID 12347 (./tmp/node3)... ✗ Error: Permission denied

Summary:
  Killed: 1
  Errors: 1
```

### Dry Run

```
Found 3 running node(s)

DRY RUN - would kill the following nodes:

  PID 12345: ./tmp/node1
  PID 12346: ./tmp/node2
  PID 12347: ./tmp/node3
```

## Implementation Details

### Discovery Mechanism

The command uses `discover_running_nodes()` from the `nodes` module, which:
1. Scans for processes matching "modal.*run" pattern
2. Searches for `node.pid` files in common directories
3. Verifies processes are actually running
4. Returns a list of `NodeInfo` structs

### Process Termination

**On Unix systems:**
- SIGTERM (graceful): Allows processes to cleanup before exiting
- SIGKILL (force): Immediate termination without cleanup

**On Windows:**
- Uses `taskkill` command with appropriate flags

### Cleanup Operations

After killing each process, the command:
1. Removes the `node.pid` file if it exists
2. Handles stale PID files (process already dead)
3. Reports any errors encountered

## Safety Considerations

### Dry-Run First
Always test with `--dry-run` when unsure:
```bash
modal local killall-nodes --dry-run
```

### Graceful vs Force
- **Graceful (`SIGTERM`)**: Allows nodes to save state, close connections cleanly
- **Force (`SIGKILL`)**: Immediate termination, may leave inconsistent state

**Recommendation**: Use graceful by default, force only when needed

### Scope
The command only affects:
- Modal node processes (run-validator, run-miner, run-observer)
- Processes discoverable through PID files in standard locations

It will NOT affect:
- Other modal commands (e.g., one-off commands)
- Processes in non-standard locations
- Processes without PID files

## Comparison with Alternatives

### vs. Individual `modal node kill`

**Before:**
```bash
modal node kill --dir ./tmp/node1
modal node kill --dir ./tmp/node2
modal node kill --dir ./tmp/node3
# ... repeat for all nodes
```

**After:**
```bash
modal local killall-nodes
```

### vs. Shell Script Loop

**Before:**
```bash
for dir in ./tmp/node*; do
    if [ -d "$dir" ]; then
        modal node kill --dir "$dir" --force || true
    fi
done
```

**After:**
```bash
modal local killall-nodes --force
```

### vs. Manual `kill` Command

**Before:**
```bash
pgrep -f "modal.*run" | xargs kill -9
# Leaves PID files, no error handling, no dry-run
```

**After:**
```bash
modal local killall-nodes --force
# Cleans PID files, handles errors, supports dry-run
```

## Use Cases

### 1. Test Cleanup
```bash
# End of test run
echo "Tests complete, cleaning up..."
modal local killall-nodes
```

### 2. Development Reset
```bash
# Things went wrong, start over
modal local killall-nodes --force
rm -rf ./tmp/node*
./scripts/setup-devnet.sh
```

### 3. CI/CD Teardown
```bash
# Ensure clean state between test runs
- name: Cleanup
  run: |
    modal local killall-nodes --force || true
  if: always()
```

### 4. Emergency Stop
```bash
# Quick way to stop everything during development
alias stopall='modal local killall-nodes --force'
```

### 5. Before Upgrades
```bash
# Stop all nodes before upgrading binary
modal local killall-nodes
cargo build --release
# Restart nodes...
```

## Files

### New Files
- `rust/modal/src/cmds/local/killall_nodes.rs` - Command implementation
- `rust/modal/src/cmds/local/mod.rs` - Local commands module
- `rust/modal/src/cmds/local/nodes.rs` - Moved from `cmds/nodes.rs`
- `examples/network/test-killall-nodes.sh` - Test script

### Modified Files
- `rust/modal/src/main.rs` - Added `KillallNodes` to `LocalCommands`
- `rust/modal/src/cmds/mod.rs` - Added `local` module
- `docs/node-management-commands.md` - Added documentation

## Testing

Run the test script:
```bash
bash examples/network/test-killall-nodes.sh
```

The test:
1. Starts multiple nodes
2. Verifies they're running
3. Tests dry-run mode
4. Tests graceful kill
5. Tests force kill
6. Verifies all nodes stopped

## Future Enhancements

Potential additions:
- `--filter` option to kill only certain types (e.g., only validators)
- `--except` option to exclude specific nodes
- `--wait` option to wait for graceful shutdown before force killing
- `--restart` option to restart nodes after killing
- JSON output format for scripting
- Timeout handling for stuck processes

## Related Commands

- `modal local nodes` - List all running nodes
- `modal node kill` - Kill a specific node
- `modal node pid` - Get PID of a specific node
- `modal node run-validator/miner/observer` - Start nodes

## Migration from Old Scripts

Replace manual cleanup logic:

**Old:**
```bash
cleanup() {
    for pid_file in ./tmp/*/node.pid; do
        if [ -f "$pid_file" ]; then
            kill -9 $(cat "$pid_file") 2>/dev/null || true
            rm -f "$pid_file"
        fi
    done
}
```

**New:**
```bash
cleanup() {
    modal local killall-nodes --force || true
}
```

Simpler, more reliable, and better error handling!

