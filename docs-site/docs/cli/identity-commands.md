---
sidebar_position: 3
title: Identity Commands
---

# Identity Commands (`modal id`)

## Create a New Identity

```bash
# Create with default path
modal id create --path alice.passfile

# Create with password protection
modal id create --path alice.passfile --password
```

## Derive Sub-Identity

```bash
modal id derive --path alice.passfile --sub "escrow-key" --output alice-escrow.passfile
```

Derives a deterministic sub-key from your main identity. Useful for:
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
modal passfile encrypt --path alice.passfile --password
```

### Decrypt a Passfile

```bash
modal passfile decrypt --path alice.passfile.enc --password
```

## Best Practices

1. **Protect your passfiles** — They contain your private keys
2. **Use password encryption** for long-term storage
3. **Derive sub-keys** for different contracts
4. **Back up your identity key** — It represents your identity
