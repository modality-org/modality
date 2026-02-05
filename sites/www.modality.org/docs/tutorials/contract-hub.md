---
sidebar_position: 4
title: Contract Hub
---

# Contract Hub Tutorial

Push and pull Modality contracts to a **hub** (centralized) or **chain** (decentralized).

## Hub vs Chain

| Feature | Hub | Chain |
|---------|-----|-------|
| URL format | `http://...` | `/ip4/.../p2p/...` |
| Auth | ed25519 keypairs | Node identity |
| Validation | Server-side | Consensus |
| Speed | Fast | Depends on network |
| Trust | Hub operator | Validators |

## Quick Start

### 1. Start the Hub

```bash
modal hub start --detach
modal hub status
```

### 2. Register Your Identity

```bash
modal hub register
```

This saves credentials to `.modal-hub/credentials.json`.

### 3. Create a Contract on Hub

```bash
modal hub create "My Contract" --description "Description here"
```

## Push/Pull Workflow

### Add a Remote

```bash
modal c remote add hub http://localhost:3100/contracts/my-contract
```

### Push Changes

```bash
modal c push hub
```

### Pull Changes

```bash
modal c pull hub
```

## Multi-Party Collaboration

### Alice Creates the Contract

```bash
modal hub create "Escrow with Bob"
# Output: Contract ID: con_abc123

modal c remote add hub http://localhost:3100/contracts/con_abc123
modal c push hub
```

### Alice Grants Bob Access

```bash
modal hub grant con_abc123 --identity bob_id_xyz --role writer
```

### Bob Clones and Contributes

```bash
# Bob clones
modal c clone http://localhost:3100/contracts/con_abc123

# Bob adds his changes
modal c set /parties/bob.id $(modal id get --path ./bob.passfile)
modal c commit --all --sign bob.passfile -m "Bob joins"
modal c push hub
```

## Access Control

| Role | Permissions |
|------|-------------|
| `owner` | Full control (push, grant, delete) |
| `writer` | Push commits |
| `reader` | Pull only |

### Grant Access

```bash
modal hub grant <contract_id> --identity <id> --role writer
```

### Revoke Access

```bash
modal hub revoke <contract_id> --identity <id>
```

## Two-Tier Authentication

The hub uses two ed25519 keypairs:

- **Identity Key** — Long-lived, represents your permanent identity
- **Access Key** — Short-lived, rotatable session key

If an access key is compromised, you can revoke it without changing your identity.

## Chain Sync (Decentralized)

For trustless operation, sync to the chain instead:

```bash
# Add chain remote
modal c remote add chain /ip4/validator.modality.network/tcp/4001/p2p/12D3KooW...

# Push to chain
modal c push chain
```

Chain commits are validated by consensus — no single party can censor or tamper.
