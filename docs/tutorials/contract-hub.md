---
sidebar_position: 4
title: Contract Hub
---

# Contract Hub Tutorial

Push and pull Modality contracts through the **Rust hub** (`modal hub start`)
or a **chain** remote (decentralized). A JavaScript hub remains under
`services/contract-hub` when a JS-hosted API is needed.

## Hub vs Chain

| Feature | Hub | Chain |
|---------|-----|-------|
| URL format | `http://...` | `/ip4/.../p2p/...` |
| Validation | Server-side (`modality-lang`) | Consensus |
| Speed | Fast | Depends on network |
| Trust | Hub operator | Validators |

## Quick Start

### 1. Start the Hub

```bash
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0 --data-dir .hub
```

The current `modal hub` command group starts the server only. See
[Hub Commands](../cli/hub-commands).

### 2. Create a contract (optional)

```bash
curl -s -X POST http://127.0.0.1:8080/contracts \
  -H 'content-type: application/json' \
  -d '{"template":"escrow"}'
```

`modal c push` will also create the remote contract id if it does not exist.

## Push/Pull Workflow

```bash
modal c remote add origin http://127.0.0.1:8080/contracts/my-contract
modal c push origin
modal c pull origin
```

Credentials in `.modal-hub/credentials.json` are optional on the Rust hub.
They are used by the JavaScript hub in `services/contract-hub`. The Rust hub
accepts unauthenticated push/pull on localhost.

## Multi-Party Collaboration

### Alice publishes

```bash
modal c remote add origin http://127.0.0.1:8080/contracts/escrow-with-bob
modal c push origin
```

### Bob clones and contributes

```bash
modal c clone http://127.0.0.1:8080/contracts/escrow-with-bob
modal c set-named-id /parties/bob.id ./bob.mod_passfile
modal c commit --all --sign bob.mod_passfile -m "Bob joins"
modal c push origin
```

## Chain Sync (Decentralized)

For trustless operation, sync to the chain instead:

```bash
modal c remote add chain /ip4/validator.modality.network/tcp/4001/p2p/12D3KooW...
modal c push chain
```

Chain commits are validated by consensus — no single party can censor or tamper.

## See also

- [Hub Commands](../cli/hub-commands)
- [Hub REST API](../reference/hub-rest-api)
- [RFC 8555 ACME autoformalization example](../../experiments/ietf-autoformalization/rfc8555-acme/)
