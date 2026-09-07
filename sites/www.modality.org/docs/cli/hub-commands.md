---
sidebar_position: 7
title: Hub Commands
---

# Hub Commands (`modal hub`)

Run the contract hub server from the full Rust wrapper. These commands are
available in full builds, not in the lean first-contract onboarding wrapper.

The current CLI surface exposes server startup only. Contract exchange happens
through the hub HTTP API and the contract remote commands that can talk to HTTP
hub URLs. Older sketches that mentioned `modal hub register`, `create`,
`grant`, `revoke`, `status`, or `auth` do not match the current wrapper.

## Start

```bash
modal hub start [OPTIONS]
```

Start a REST hub and, by default, a JSON-RPC compatibility endpoint. The server
stores contracts under the configured data directory and logs the REST, RPC, and
health endpoints when it starts.

**Options:**
| Option | Description |
|--------|-------------|
| `--host <HOST>` | Bind address; defaults to `0.0.0.0` |
| `--port <PORT>` | REST API port; defaults to `8080` |
| `--rpc-port <RPC_PORT>` | JSON-RPC port; defaults to `3000`; use `0` to disable RPC |
| `--data-dir <DATA_DIR>` | Data directory for stored contracts; defaults to `.hub` |
| `--cors <BOOL>` | Enable browser CORS headers; defaults to `true` |

**Examples:**
```bash
modal hub start --data-dir .hub
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0
```

## REST API

The startup command serves these REST routes:

| Route | Method | Purpose |
|-------|--------|---------|
| `/health` | `GET` | Health check |
| `/contracts` | `POST` | Create a contract |
| `/contracts/synthesize` | `POST` | Synthesize a contract draft |
| `/contracts/:id` | `GET` | Get contract metadata and state |
| `/contracts/:id/state` | `GET` | Get the materialized contract state |
| `/contracts/:id/log` | `GET` | Get the commit log; accepts `limit` and `offset` query parameters |
| `/contracts/:id/commits` | `POST` | Submit a commit |
| `/contracts/:id/commits/:hash` | `GET` | Get one commit |
| `/templates` | `GET` | List built-in templates |
| `/templates/:id` | `GET` | Get one template |

For local checks:

```bash
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/templates
```

## Contract Remotes

For command-line contract work, configure an HTTP hub URL through the contract
remote commands and then use `modal c push` or `modal c pull`:

```bash
modal c remote add origin http://127.0.0.1:8080/contracts/<contract-id>
modal c push origin --sign ~/.modality/alice.mod_passfile
modal c pull origin
```

HTTP hub remotes read credentials from `.modal-hub/credentials.json` by default,
or from the `--hub-creds <HUB_CREDS>` option on `modal c push` and `modal c
pull`. The current `modal hub` command group does not create that credentials
file.
