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
| Trust | Hub operator | Network consensus |

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
# The first push names the hub URL and saves it as the `origin` remote
modal c push --remote http://127.0.0.1:8080/contracts/my-contract
modal c push
modal c pull
```

Credentials in `.modal-hub/credentials.json` are optional on the Rust hub.
They are used by the JavaScript hub in `services/contract-hub`. The Rust hub
accepts unauthenticated push/pull on localhost.

## Multi-Party Collaboration

### Alice publishes

```bash
modal c push --remote http://127.0.0.1:8080/contracts/escrow-with-bob
```

### Bob clones and contributes

```bash
# A new copy in ./escrow-with-bob, with the hub saved as `origin`
modal c pull http://127.0.0.1:8080/contracts/escrow-with-bob
cd escrow-with-bob
modal c set-named-id /parties/bob.id ../bob.passfile
modal c commit --all --sign ../bob.passfile -m "Bob joins"
modal c push
```

## Chain Sync (Decentralized)

For trustless operation, sync to the chain instead:

```bash
modal c push --remote-name chain --remote /ip4/<sequencer>/tcp/<port>/ws/p2p/12D3KooW...
modal c push --remote-name chain
```

Chain commits are validated by consensus — no single party can censor or tamper.

## See also

- [Hub Commands](../cli/hub-commands)
- [Hub REST API](../reference/hub-rest-api)
- [RFC 8555 ACME autoformalization example](../../experiments/ietf-autoformalization/rfc8555-acme/)
