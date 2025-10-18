# Network Node Autoupgrade

The network node supports automatic upgrading by downloading pre-built binaries from a package server. When enabled, the node will periodically check for new versions and automatically upgrade itself by downloading and replacing the binary.

## Configuration

Add the following fields to your node configuration JSON file:

```json
{
  "autoupgrade_enabled": true,
  "autoupgrade_base_url": "http://packages.modality.org",
  "autoupgrade_branch": "testnet",
  "autoupgrade_check_interval_secs": 3600
}
```

### Configuration Options

- **`autoupgrade_enabled`** (optional, boolean): Enable or disable the autoupgrade feature. Default: `false`
- **`autoupgrade_base_url`** (optional, string): The base URL of the package server. Default: `http://packages.modality.org`
- **`autoupgrade_branch`** (optional, string): The branch to upgrade from (e.g., "testnet" or "mainnet"). Default: `testnet`
- **`autoupgrade_check_interval_secs`** (optional, number): How often to check for updates in seconds. Default: `3600` (1 hour)

## How It Works

1. **Periodic Checks**: The node spawns a background task that periodically checks the package server for new versions by fetching the manifest

2. **Detecting Updates**: The autoupgrade task compares the latest version in the manifest with the version at startup. If they differ, an update is available.

3. **Downloading Updates**: When a new version is detected, the node:
   - Fetches the manifest from `{base_url}/{branch}/latest/manifest.json`
   - Determines the appropriate binary for the current platform
   - Downloads the binary from `{base_url}/{branch}/latest/binaries/{platform}/modality`

4. **Binary Replacement**: After successful download:
   - The current executable is replaced with the newly downloaded binary
   - A new process is spawned with the same command-line arguments
   - The current process exits gracefully

5. **Seamless Restart**: The node restarts with the new version, maintaining the same configuration

## Requirements

- **Network Access**: The node must be able to reach the package server URL
- **Write Permissions**: The node must have permission to replace its own binary

## Platform Support

The autoupgrade feature automatically detects your platform and downloads the appropriate binary:
- Linux x86_64
- Linux ARM64 (aarch64)
- macOS Intel (x86_64)
- macOS Apple Silicon (aarch64)
- Windows x86_64

## Migration from Registry-Based Autoupgrade

If you were previously using the registry-based autoupgrade (with `autoupgrade_registry_url`), please update your configuration to use the new binary-based approach:

**Old configuration:**
```json
{
  "autoupgrade_enabled": true,
  "autoupgrade_registry_url": "http://packages.modality.org/testnet/latest/cargo-registry/index/"
}
```

**New configuration:**
```json
{
  "autoupgrade_enabled": true,
  "autoupgrade_base_url": "http://packages.modality.org",
  "autoupgrade_branch": "testnet"
}
```

The old `autoupgrade_registry_url` field is deprecated but still supported for backward compatibility.

## Example Configuration

See `fixtures/network-node-configs/devnet1/node1-with-autoupgrade.json` for a complete example.

## Security Considerations

- **Trust the Package Server**: Only configure autoupgrade with package servers you trust, as the node will automatically download and execute binaries from that source
- **HTTPS**: Ensure the package server URL uses HTTPS to prevent man-in-the-middle attacks
- **Access Control**: Consider the security implications of automatic updates in your deployment environment
- **Binary Verification**: Future versions may include signature verification for downloaded binaries

## Troubleshooting

### Autoupgrade Not Working

1. Check that `autoupgrade_enabled` is set to `true`
2. Verify the package server URL is accessible: `curl {base_url}/{branch}/latest/manifest.json`
3. Check the logs for autoupgrade-related messages
4. Ensure the node has network access to the package server

### Download Fails

If binary download fails, check:
- Network connectivity to the package server
- The package server has binaries available for your platform
- Sufficient disk space in the temporary directory
- No firewall rules blocking access to the package server

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

- **INFO**: Initial setup, upgrade detection, download progress, binary replacement
- **DEBUG**: Periodic check messages
- **ERROR**: Problems with manifest fetch, download failures, or binary replacement issues

Look for log messages starting with "Autoupgrade" to track the feature's behavior.

## Package Server Format

The autoupgrade system expects the package server to provide:
- A `manifest.json` file at `{base_url}/{branch}/latest/manifest.json`
- Binary files at paths specified in the manifest

Example manifest structure:
```json
{
  "version": "20251018_182116-3a00ac0",
  "git_branch": "testnet",
  "git_commit": "3a00ac0",
  "packages": {
    "binaries": {
      "darwin-aarch64": {
        "name": "modality",
        "path": "binaries/darwin-aarch64/modality",
        "platform": "darwin",
        "arch": "aarch64"
      },
      "linux-x86_64": {
        "name": "modality",
        "path": "binaries/linux-x86_64/modality",
        "platform": "linux",
        "arch": "x86_64"
      }
    }
  }
}
```

