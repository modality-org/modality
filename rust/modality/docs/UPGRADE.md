# Modality Auto-Upgrade

The Modality CLI includes a built-in auto-upgrade feature that allows you to easily update to the latest version without needing to reinstall.

## Usage

### Basic Upgrade

Upgrade to the latest testnet version:

```bash
modality upgrade
```

This will:
1. Detect your platform automatically
2. Fetch the latest version from `http://packages.modality.org`
3. Download the appropriate binary
4. Replace your current binary with the new one
5. Confirm the upgrade was successful

### Advanced Options

#### Upgrade from mainnet

```bash
modality upgrade --branch mainnet
```

#### Upgrade to a specific version

```bash
modality upgrade --version 20251018_182116-3a00ac0
```

#### Force upgrade (skip confirmation)

```bash
modality upgrade --force
```

#### Use a custom package server

```bash
modality upgrade --base-url http://my-custom-server.com
```

## How It Works

The upgrade command:

1. **Detects Platform**: Automatically identifies your OS and architecture
   - Linux x86_64
   - Linux ARM64
   - macOS Intel
   - macOS Apple Silicon
   - Windows x86_64

2. **Fetches Manifest**: Downloads the `manifest.json` from the package server to get version information

3. **Downloads Binary**: Fetches the appropriate binary for your platform

4. **Self-Replacement**: Uses the `self-replace` crate to safely replace the running binary

5. **Verification**: Confirms the upgrade was successful

## Example Session

```bash
$ modality upgrade

🚀 Modality Upgrade

🖥️  Platform: darwin-aarch64
📍 Current binary: /Users/username/.modality/bin/modality
📡 Fetching manifest from: http://packages.modality.org/testnet/latest/manifest.json
📦 Latest version: 20251018_182116-3a00ac0
🌿 Branch: testnet
🔖 Commit: 3a00ac0

⚠️  About to upgrade to version 20251018_182116-3a00ac0
   Binary: http://packages.modality.org/testnet/latest/binaries/darwin-aarch64/modality

Continue? [y/N]: y
⬇️  Downloading: http://packages.modality.org/testnet/latest/binaries/darwin-aarch64/modality
✅ Downloaded successfully

🔄 Replacing binary...
✅ Upgrade complete!

🎉 Modality has been upgraded to version 20251018_182116-3a00ac0
   Run 'modality --version' to verify
```

## Automated Upgrades

You can automate upgrades in scripts:

```bash
# Upgrade without confirmation
modality upgrade --force

# Or with error handling
if modality upgrade --force; then
    echo "Upgrade successful"
else
    echo "Upgrade failed"
    exit 1
fi
```

## Troubleshooting

### Permission Denied

If you get a permission error, make sure you have write access to the binary location:

```bash
ls -la $(which modality)
```

### Binary Location

The upgrade command will replace the binary at its current location. If you installed using the install script, it's typically at:
- `~/.modality/bin/modality`

If you compiled from source, it might be elsewhere in your PATH.

### Network Issues

If the download fails, check:
1. Your internet connection
2. The package server is accessible: `curl http://packages.modality.org/testnet/latest/manifest.json`
3. Your platform is supported

## Security

- The upgrade command uses **HTTP** by default (not HTTPS) to match the package distribution setup
- Binaries are verified by checking they can execute before replacement
- The old binary is only replaced if the download is successful
- Temporary files are cleaned up automatically

## Integration with Installation

The upgrade feature works seamlessly with the one-line installer:

```bash
# Initial install
curl -fsSL http://packages.modality.org/testnet/latest/install.sh | sh

# Later, upgrade
modality upgrade
```

No need to reinstall or manage paths manually!

