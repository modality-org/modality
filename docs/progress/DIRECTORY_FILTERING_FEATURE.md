# Directory Filtering for Node Management Commands

## Summary

Added `--dir` option to both `modal local nodes` and `modal local killall-nodes` commands, enabling selective operations on nodes based on their directory location. This allows filtering to only nodes within a specified directory tree (recursively).

## Changes

### 1. Command Line Interface

Added new `--dir` option to:
- `modal local nodes --dir <PATH>` - Show only nodes in the specified directory or subdirectories
- `modal local killall-nodes --dir <PATH>` - Kill only nodes in the specified directory or subdirectories

### 2. Implementation

**File: `rust/modal/src/cmds/local/killall_nodes.rs`**
- Added `dir: Option<PathBuf>` field to `Opts` struct
- Added `filter_nodes_by_directory()` function that uses path canonicalization
- Filters nodes by checking if their canonical path starts with the specified directory's canonical path

**File: `rust/modal/src/cmds/local/nodes.rs`**
- Added `dir: Option<PathBuf>` field to `Opts` struct
- Added public `filter_nodes_by_directory()` function (reused by killall_nodes)
- Filters nodes using the same path canonicalization approach

### 3. Path Canonicalization

The implementation uses `fs::canonicalize()` to:
- Handle relative paths (e.g., `.`, `./examples/network`)
- Resolve symlinks
- Ensure reliable path comparison using `Path::starts_with()`

### 4. Example Updates

Updated cleanup scripts in `examples/network/` to use the new command:

**Updated files:**
- `examples/network/miner-gossip-race/00-clean.sh`
- `examples/network/miner-gossip-race/test.sh`
- `examples/network/miner-gossip/test.sh`
- `examples/network/05-mining/test.sh`
- `examples/network/wasm-in-contract/test.sh`

**Old pattern:**
```bash
pkill -9 -f "modal node run-miner.*miner-gossip" 2>/dev/null || true
```

**New pattern:**
```bash
modal local killall-nodes --dir . --force 2>/dev/null || true
```

### 5. Documentation Updates

Updated documentation to include the new `--dir` option:
- `docs/node-management-commands.md` - Added `--dir` option documentation and examples
- `examples/network/README.md` - Updated cleanup instructions
- `examples/network/QUICK_REFERENCE.md` - Updated troubleshooting section
- `examples/network/CI-CD-GUIDE.md` - Updated CI cleanup patterns
- `examples/network/08-network-partition/README.md` - Updated troubleshooting

## Usage Examples

### Basic Usage

```bash
# Show only nodes in current directory tree
modal local nodes --dir .

# Kill only nodes in current directory tree
modal local killall-nodes --dir . --force

# Show nodes in specific directory
modal local nodes --dir ./examples/network

# Dry run to see what would be killed
modal local killall-nodes --dir . --dry-run
```

### Combined with Other Filters

```bash
# Kill only devnet nodes in current directory
modal local killall-nodes --network "devnet*" --dir . --force

# Show verbose info for nodes in specific directory
modal local nodes --dir ./tmp --verbose
```

### In Test/Cleanup Scripts

```bash
#!/usr/bin/env bash
cd $(dirname -- "$0")

# Clean up any previous test runs in this directory tree
modal local killall-nodes --dir . --force 2>/dev/null || true
sleep 1

# ... rest of test script
```

## Benefits

1. **Scoped Cleanup**: Tests can clean up only their own nodes without affecting other running nodes
2. **Safety**: Reduces risk of accidentally killing unrelated nodes when multiple developers/tests are running
3. **Simplicity**: Single command replaces complex `pkill` patterns with specific path matching
4. **Cross-platform**: Works consistently on all platforms (unlike `pkill` patterns)
5. **Composable**: Can be combined with `--network` filter for even more precise control

## Use Cases

### 1. Local Development
```bash
# Kill all nodes in your test directory
cd examples/network/my-test
modal local killall-nodes --dir . --force
```

### 2. CI/CD
```bash
# Cleanup only CI test nodes
modal local killall-nodes --dir /tmp/ci-tests --force
```

### 3. Multi-User Environments
```bash
# Each user works in their own directory
cd ~/my-tests
modal local killall-nodes --dir . --force  # Won't affect other users
```

### 4. Test Isolation
```bash
# Each test directory cleans up only its nodes
cd examples/network/miner-gossip-race
./00-clean.sh  # Uses: modal local killall-nodes --dir .
```

## Technical Details

### Path Matching Algorithm

1. Canonicalize the filter directory path (resolves `.`, `..`, symlinks)
2. For each discovered node:
   - Try to canonicalize the node's directory path
   - Check if the node's canonical path starts with the filter's canonical path
   - Include node in results if it matches

### Error Handling

- If the specified directory doesn't exist, `fs::canonicalize()` will return an error
- If a node's directory can't be canonicalized (e.g., deleted), the node is excluded from results
- All errors are properly propagated up to the user with helpful messages

### Recursive Behavior

The directory filtering is inherently recursive because it uses `Path::starts_with()`:
- `--dir .` matches `.`, `./tmp`, `./tmp/node1`, `./tmp/node1/storage`, etc.
- `--dir /path/to/tests` matches all subdirectories under `/path/to/tests`

## Testing

The feature can be tested with:

```bash
# Build the CLI
cd rust && cargo build --package modal

# Verify help text includes --dir option
modal local nodes --help
modal local killall-nodes --help

# Test with actual nodes (requires running nodes)
cd examples/network/miner-gossip-race
./01-run-miner1.sh  # Start a test node
modal local nodes --dir .  # Should show the node
modal local killall-nodes --dir . --dry-run  # Should show what would be killed
modal local killall-nodes --dir . --force  # Actually kill it
```

## Notes

- The `--dir` filter can be combined with `--network` filter for more precise control
- Use `--dry-run` to preview what would be killed before actually killing
- The filter works recursively through all subdirectories
- Relative paths (like `.`) are supported and recommended for test scripts


