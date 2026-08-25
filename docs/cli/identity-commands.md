---
sidebar_position: 3
title: Identity Commands
---

# Identity Commands (`modal id`)

## Create a New Identity

```bash
# Create with default path
modal id create --path alice.passfile

# Create with passfile encryption
modal id create --path alice.passfile --encrypt

# Create in the standard passfile directory
modal id create --name alice
```

`modal id create` writes a passfile. Use `--path <PATH>` for an explicit
location, `--dir <DIR>` for an output directory, or `--name <NAME>` for the
standard `~/.modality/<name>.mod_passfile` location.

## Derive Sub-Identity

```bash
modal id derive --mnemonic "abandon abandon ..." --path alice-escrow.passfile
```

Derives a keypair from a BIP39 mnemonic seed phrase. Useful for:
- Contract-specific keys
- Rotating access keys
- Hierarchical key management

## Get Public ID

```bash
modal id get --path alice.passfile
# Output: ed25519:abc123...
```

## Passfile Operations

### Encrypt a Passfile

```bash
modal passfile encrypt --path alice.passfile
```

### Decrypt a Passfile

```bash
modal passfile decrypt --path alice.passfile
```

## Best Practices

1. **Protect your passfiles** — They contain your private keys
2. **Use password encryption** for long-term storage
3. **Derive sub-keys** for different contracts
4. **Back up your identity key** — It represents your identity
