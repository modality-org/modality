# Network Node Autoupgrade

The network node supports automatic upgrading from a configured cargo registry. When enabled, the node will periodically check for new versions and automatically upgrade itself by installing the latest version using `cargo install`.

## Configuration

Add the following fields to your node configuration JSON file:

```json
{
  "autoupgrade_enabled": true,
  "autoupgrade_registry_url": "http://packages.modality.org/testnet/latest/cargo-registry/index/",
  "autoupgrade_check_interval_secs": 3600
}
```

### Configuration Options

- **`autoupgrade_enabled`** (optional, boolean): Enable or disable the autoupgrade feature. Default: `false`
- **`autoupgrade_registry_url`** (required if enabled, string): The cargo registry URL to check for updates
- **`autoupgrade_check_interval_secs`** (optional, number): How often to check for updates in seconds. Default: `3600` (1 hour)

## How It Works

1. **Periodic Checks**: The node spawns a background task that periodically checks the configured cargo registry for new versions using `cargo search`

2. **Detecting Updates**: The autoupgrade task compares the latest version in the registry with the version at startup. If they differ, an update is available.

3. **Installing Updates**: When a new version is detected, the node:
   - Runs `cargo install --index sparse+<registry_url> modality --force`
   - Downloads and compiles the new version
   - Verifies the installation was successful

4. **Binary Replacement**: After successful compilation:
   - The current executable is replaced with the newly compiled binary
   - A new process is spawned with the same command-line arguments
   - The current process exits gracefully

5. **Seamless Restart**: The node restarts with the new version, maintaining the same configuration

## Requirements

- **Cargo**: Must be installed and available in PATH (used to search and install updates)
- **Network Access**: The node must be able to reach the cargo registry URL
- **Write Permissions**: The node must have permission to replace its own binary

## Example Configuration

See `fixtures/network-node-configs/devnet1/node1-with-autoupgrade.json` for a complete example.

## Security Considerations

- **Trust the Registry**: Only configure autoupgrade with cargo registries you trust, as the node will automatically download and execute code from that source
- **Registry Security**: Ensure the registry URL uses HTTPS to prevent man-in-the-middle attacks
- **Access Control**: Consider the security implications of automatic updates in your deployment environment

## Troubleshooting

### Autoupgrade Not Working

1. Check that `autoupgrade_enabled` is set to `true`
2. Verify cargo is installed: `cargo --version`
3. Check the logs for autoupgrade-related messages
4. Ensure the node has network access to the cargo registry
5. Verify the registry URL is correct and accessible

### Installation Fails

If `cargo install` fails, check:
- Sufficient disk space for compilation
- All required system dependencies are installed
- The cargo registry is accessible and contains the modality package
- The registry URL is correct and accessible

### Binary Replacement Fails

On some systems, replacing a running executable may require special permissions. Check:
- The node process has write permissions to its own binary location
- No file locking or antivirus software is preventing the replacement

## Disabling Autoupgrade

To disable autoupgrade:
1. Set `autoupgrade_enabled` to `false` in the configuration, or
2. Remove the autoupgrade configuration fields entirely, or
3. Restart the node after making the configuration change

## Logs

The autoupgrade feature logs its activity at various levels:

- **INFO**: Initial setup, upgrade detection, installation progress
- **DEBUG**: Periodic check messages
- **ERROR**: Problems with registry access, installation failures, or binary replacement issues

Look for log messages starting with "Autoupgrade" to track the feature's behavior.

## Registry Format

The autoupgrade system uses Cargo's sparse registry format. The registry URL should point to a directory containing:
- `config.json` - Registry configuration
- Package index files - Containing package metadata and versions

Example registry structure:
```
http://packages.modality.org/testnet/latest/cargo-registry/index/
├── config.json
├── mo/
│   └── modality/
│       └── index
└── ...
```

