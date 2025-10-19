# Network Node Noop Mode

The network node supports a "noop" (no-operation) mode that boots up but doesn't perform any network operations. This mode is useful for testing autoupgrade functionality without running the full node operations.

## Usage

Run a noop node using the CLI:

```bash
modality node run-noop --config /path/to/config.json
```

## Configuration

The noop mode uses the same configuration format as regular nodes, but with the `noop_mode` field set to `true`:

```json
{
  "passfile_path": "../../passfiles/node1.mod_passfile",
  "storage_path": "../../../tmp/storage/node1",
  "autoupgrade_enabled": true,
  "autoupgrade_git_repo": "https://github.com/modality-org/modality",
  "autoupgrade_git_branch": "devnet",
  "autoupgrade_check_interval_secs": 300,
  "noop_mode": true
}
```

### Configuration Fields

- **`noop_mode`** (optional, boolean): Enable noop mode. When `true`, the node will only run autoupgrade and skip all network operations.

## What Noop Mode Does

1. **Boots Up**: Initializes the node with the provided configuration
2. **Starts Autoupgrade**: If `autoupgrade_enabled` is `true`, starts the autoupgrade background task
3. **Periodic Status**: Logs status messages every 5 minutes showing autoupgrade status
4. **Waits for Shutdown**: Responds to Ctrl+C or shutdown signals gracefully

## What Noop Mode Does NOT Do

- **No Networking**: Doesn't start networking, consensus, or mining tasks
- **No Peer Connections**: Doesn't connect to bootstrappers or other peers
- **No Gossip**: Doesn't participate in gossip protocols
- **No Mining**: Doesn't mine blocks
- **No Consensus**: Doesn't participate in consensus protocols

## Use Cases

### Testing Autoupgrade

The primary use case is testing the autoupgrade functionality:

```bash
# Start a noop node with autoupgrade enabled
modality node run-noop --config node1-noop.json

# The node will:
# 1. Boot up
# 2. Start checking for updates every 5 minutes (or configured interval)
# 3. Log status messages
# 4. Automatically upgrade when new commits are detected
```

### Development and Debugging

- **Isolated Testing**: Test autoupgrade without network interference
- **Resource Monitoring**: Monitor autoupgrade behavior without network overhead
- **Configuration Validation**: Verify autoupgrade configuration without full node startup

## Example Configuration

See `fixtures/network-node-configs/devnet1/node1-noop.json` for a complete example configuration.

## Logs

The noop mode logs its activity:

- **INFO**: Initial startup message
- **INFO**: Periodic status messages (every 5 minutes)
- **INFO**: Autoupgrade activity (if enabled)
- **INFO**: Shutdown messages

Example log output:
```
INFO Starting noop node with config: node1-noop.json
INFO Starting node in noop mode - only autoupgrade will be active
INFO Autoupgrade enabled: checking https://github.com/modality-org/modality branch 'devnet' every 300s
INFO Current commit on branch 'devnet': abc123...
INFO Noop node running - autoupgrade active: true
INFO Noop node running - autoupgrade active: true
```

## Comparison with Regular Nodes

| Feature | Regular Node | Noop Node |
|---------|-------------|-----------|
| Bootup | ✅ | ✅ |
| Autoupgrade | ✅ | ✅ |
| Networking | ✅ | ❌ |
| Consensus | ✅ | ❌ |
| Mining | ✅ | ❌ |
| Gossip | ✅ | ❌ |
| Resource Usage | High | Low |

## Integration with Autoupgrade

The noop mode is specifically designed to work seamlessly with the autoupgrade feature:

- **Same Configuration**: Uses identical autoupgrade configuration as regular nodes
- **Same Behavior**: Autoupgrade works exactly the same way
- **Isolated Testing**: Perfect for testing autoupgrade without network complexity
- **Quick Validation**: Fast way to verify autoupgrade configuration changes

This makes it an ideal tool for development, testing, and validation of the autoupgrade system.
