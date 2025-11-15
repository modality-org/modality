# Command Rename: `modal nodes` → `modal local nodes`

## Change Summary

The `modal nodes` command has been renamed to `modal local nodes` to better reflect its purpose as a local development tool.

## Rationale

- **Semantic Clarity**: The command discovers nodes running on the local machine, making `local` a more accurate namespace
- **Namespace Organization**: Creates a dedicated `local` command group for development-related commands
- **Future Extensibility**: Opens the door for additional local development commands:
  - `modal local clean` - Clean all local test nodes
  - `modal local restart-all` - Restart all local nodes
  - `modal local status` - Show status of all local nodes

## Updated Command

**Before:**
```bash
modal nodes [OPTIONS]
```

**After:**
```bash
modal local nodes [OPTIONS]
```

## Usage Examples

```bash
# Find all running local nodes
modal local nodes

# Show verbose output with full paths
modal local nodes --verbose

# Count running nodes
modal local nodes | grep "^PID:" | wc -l

# Find a specific node
modal local nodes | grep "node1"
```

## Files Modified

1. **rust/modal/src/main.rs**
   - Added `LocalCommands` enum with `Nodes` subcommand
   - Added `Commands::Local` variant
   - Moved `Nodes` from top-level command to `Local` subcommand group

2. **examples/network/test-nodes-command.sh**
   - Updated all references to use `modal local nodes`

3. **docs/node-management-commands.md**
   - Updated all command references and examples

4. **NODE_MANAGEMENT_COMMANDS.md**
   - Updated all command references and examples

## Migration

For any scripts or documentation using the old command:

**Find and replace:**
- `modal nodes` → `modal local nodes`
- `$MODAL_BIN nodes` → `$MODAL_BIN local nodes`

## Help Text

```bash
$ modal local --help
Local development commands

Usage: modal local <COMMAND>

Commands:
  nodes  Find all running modal node processes
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## Testing

Run the test script to verify:
```bash
bash examples/network/test-nodes-command.sh
```

## Future Enhancements

The `modal local` namespace can be extended with additional development commands:

- `modal local clean` - Remove all local test node directories
- `modal local restart-all` - Restart all discovered nodes
- `modal local kill-all` - Kill all discovered nodes
- `modal local logs <node>` - Tail logs of a specific node
- `modal local status` - Show detailed status of all local nodes
- `modal local network` - Display topology of running local nodes

