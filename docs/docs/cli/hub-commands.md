---
sidebar_position: 7
title: Hub Commands
---

# Hub Commands (`modal hub`)

The contract hub is a collaborative server for multi-party contracts. It provides HTTP-based access to contracts, handles authentication, and validates commits against contract rules.

## Server Management

### Start

```bash
modal hub start [OPTIONS]
```

Start the contract hub server.

**Options:**
| Option | Description |
|--------|-------------|
| `--detach`, `-d` | Run in background |
| `--port <PORT>` | Listen port (default: 8080) |
| `--host <HOST>` | Bind address (default: 0.0.0.0) |
| `--data <PATH>` | Data directory |

**Example:**
```bash
modal hub start --detach --port 8080
```

### Stop

```bash
modal hub stop
```

Gracefully stop the running hub server.

### Status

```bash
modal hub status
```

Show hub server status, including:
- Running state
- Port and bind address
- Number of contracts
- Connected clients

## Identity Management

### Register

```bash
modal hub register [OPTIONS]
```

Register your identity with the hub. This creates an account using your ed25519 identity key.

**Options:**
| Option | Description |
|--------|-------------|
| `--passfile <PATH>` | Identity passfile |
| `--hub <URL>` | Hub URL |
| `--name <NAME>` | Display name |

**Example:**
```bash
modal hub register --passfile alice.passfile --hub http://localhost:8080
```

## Contract Management

### Create

```bash
modal hub create <NAME> [OPTIONS]
```

Create a new contract on the hub.

**Options:**
| Option | Description |
|--------|-------------|
| `--description <DESC>` | Contract description |
| `--passfile <PATH>` | Creator identity |
| `--hub <URL>` | Hub URL |
| `--public` | Make contract publicly readable |

**Example:**
```bash
modal hub create "Escrow Contract" \
  --description "3-party escrow for service delivery" \
  --passfile alice.passfile
```

### Grant Access

```bash
modal hub grant <CONTRACT_ID> [OPTIONS]
```

Grant access to a contract for another identity.

**Options:**
| Option | Description |
|--------|-------------|
| `--identity <ID>` | Identity to grant (ed25519:...) |
| `--role <ROLE>` | Role: `reader`, `writer`, `admin` |
| `--passfile <PATH>` | Your identity (must be admin) |

**Roles:**
| Role | Permissions |
|------|-------------|
| `reader` | Pull commits, read state |
| `writer` | Push commits, pull, read |
| `admin` | Grant/revoke access, all writer permissions |

**Example:**
```bash
modal hub grant abc123 \
  --identity ed25519:xyz789... \
  --role writer \
  --passfile alice.passfile
```

### Revoke Access

```bash
modal hub revoke <CONTRACT_ID> [OPTIONS]
```

Revoke access from an identity.

**Options:**
| Option | Description |
|--------|-------------|
| `--identity <ID>` | Identity to revoke |
| `--passfile <PATH>` | Your identity (must be admin) |

### List Contracts

```bash
modal hub list [OPTIONS]
```

List contracts you have access to.

**Options:**
| Option | Description |
|--------|-------------|
| `--hub <URL>` | Hub URL |
| `--passfile <PATH>` | Your identity |

### Contract Info

```bash
modal hub info <CONTRACT_ID> [OPTIONS]
```

Get information about a specific contract.

## Push and Pull

Once a hub remote is configured, use standard contract commands:

```bash
# Add hub as remote
modal c remote add origin http://hub.example.com/contracts/abc123

# Push commits
modal c push origin --sign alice.passfile

# Pull commits
modal c pull origin
```

Or use the full URL directly:

```bash
modal c push http://hub.example.com/contracts/abc123 --sign alice.passfile
modal c pull http://hub.example.com/contracts/abc123
```

## Authentication

The hub uses two-tier ed25519 authentication:

1. **Identity Key** — Your long-term identity (in passfile)
2. **Access Key** — Session-based, rotatable token

When you register or authenticate, the hub issues an access token. This token is stored locally and used for subsequent requests.

```bash
# Authenticate (if token expired)
modal hub auth --passfile alice.passfile --hub http://localhost:8080
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MODAL_HUB_URL` | Default hub URL |
| `MODAL_PASSFILE` | Default passfile for authentication |

## Example Workflow

```bash
# 1. Start the hub
modal hub start --detach

# 2. Register your identity
modal hub register --passfile alice.passfile

# 3. Create a contract
modal hub create "My Contract" --passfile alice.passfile
# Returns: Contract ID abc123

# 4. Grant access to collaborator
modal hub grant abc123 --identity ed25519:bob... --role writer

# 5. Clone locally and work
modal c pull http://localhost:8080/contracts/abc123
cd abc123
modal c checkout

# 6. Make changes and push
modal c set /data/value.text "hello"
modal c commit --all --sign alice.passfile -m "Update value"
modal c push http://localhost:8080/contracts/abc123
```
