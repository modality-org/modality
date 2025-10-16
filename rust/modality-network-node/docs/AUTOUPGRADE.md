# Network Node Autoupgrade

The network node supports automatic upgrading from a configured git branch. When enabled, the node will periodically check for new commits and automatically upgrade itself by installing the latest version using `cargo install`.

## Configuration

Add the following fields to your node configuration JSON file:

```json
{
  "autoupgrade_enabled": true,
  "autoupgrade_git_repo": "https://github.com/modality-org/modality",
  "autoupgrade_git_branch": "devnet",
  "autoupgrade_check_interval_secs": 3600
}
```

### Configuration Options

- **`autoupgrade_enabled`** (optional, boolean): Enable or disable the autoupgrade feature. Default: `false`
- **`autoupgrade_git_repo`** (required if enabled, string): The git repository URL to check for updates
- **`autoupgrade_git_branch`** (required if enabled, string): The branch to track for updates (e.g., "devnet", "main")
- **`autoupgrade_check_interval_secs`** (optional, number): How often to check for updates in seconds. Default: `3600` (1 hour)

## How It Works

1. **Periodic Checks**: The node spawns a background task that periodically checks the configured git branch for new commits using `git ls-remote`

2. **Detecting Updates**: The autoupgrade task compares the latest commit hash on the remote branch with the commit hash at startup. If they differ, an update is available.

3. **Installing Updates**: When a new commit is detected, the node:
   - Runs `cargo install --git <repo> --branch <branch> modality-network-node --force`
   - Downloads and compiles the new version
   - Verifies the installation was successful

4. **Binary Replacement**: After successful compilation:
   - The current executable is replaced with the newly compiled binary
   - A new process is spawned with the same command-line arguments
   - The current process exits gracefully

5. **Seamless Restart**: The node restarts with the new version, maintaining the same configuration

## Requirements

- **Git**: Must be installed and available in PATH
- **Cargo**: Must be installed and available in PATH (used to compile updates)
- **Network Access**: The node must be able to reach the git repository URL
- **Write Permissions**: The node must have permission to replace its own binary

## Example Configuration

See `fixtures/network-node-configs/devnet1/node1-with-autoupgrade.json` for a complete example.

## Security Considerations

- **Trust the Source**: Only configure autoupgrade with git repositories you trust, as the node will automatically download and execute code from that source
- **Branch Selection**: Use stable branches (like "devnet") rather than development branches to avoid unexpected behavior
- **Network Security**: Ensure the git repository URL uses HTTPS to prevent man-in-the-middle attacks
- **Access Control**: Consider the security implications of automatic updates in your deployment environment

## Troubleshooting

### Autoupgrade Not Working

1. Check that `autoupgrade_enabled` is set to `true`
2. Verify git is installed: `git --version`
3. Verify cargo is installed: `cargo --version`
4. Check the logs for autoupgrade-related messages
5. Ensure the node has network access to the git repository

### Installation Fails

If `cargo install` fails, check:
- Sufficient disk space for compilation
- All required system dependencies are installed
- The git repository and branch exist and are accessible

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
- **ERROR**: Problems with git access, installation failures, or binary replacement issues

Look for log messages starting with "Autoupgrade" to track the feature's behavior.

