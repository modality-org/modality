---
sidebar_position: 4
title: Node Commands
---

# Node Commands (`modal node`)

Manage network nodes — create, run, and monitor.

## Create

```bash
modal node create [OPTIONS]
```

Create a new node directory with configuration.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory (default: `./node`) |
| `--network <NET>` | Network (mainnet/testnet) |
| `--name <NAME>` | Node name |

**Creates:**
```
node/
├── config.json       # Node configuration
├── node.passfile     # Node identity
├── data/             # Chain data
└── logs/             # Log files
```

## Lifecycle Commands

### Start

```bash
modal node start [OPTIONS]
```

Start a node in the background.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory |
| `--detach` | Run in background (default) |

### Stop

```bash
modal node stop [OPTIONS]
```

Gracefully stop a running node.

### Restart

```bash
modal node restart [OPTIONS]
```

Restart a running node.

### Kill

```bash
modal node kill [OPTIONS]
```

Forcefully kill a node process.

### PID

```bash
modal node pid [OPTIONS]
```

Display the PID of a running node.

## Run Commands (Foreground)

Run nodes in the foreground (useful for debugging).

### Run Miner

```bash
modal node run-miner [OPTIONS]
```

Run a mining node that participates in block production.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory |
| `--threads <N>` | Mining threads |

### Run Validator

```bash
modal node run-validator [OPTIONS]
```

Run a validator node (observes and validates, doesn't mine).

### Run Observer

```bash
modal node run-observer [OPTIONS]
```

Run an observer node (read-only, syncs chain).

### Run Noop

```bash
modal node run-noop [OPTIONS]
```

Run a minimal node (only auto-upgrade, no network).

## Information Commands

### Info

```bash
modal node info [OPTIONS]
```

Display information about a node.

**Example output:**
```
NODE: my-node
  Status: running
  PID: 12345
  Network: mainnet
  Chain height: 1,234,567
  Peers: 12
  Uptime: 3d 4h 12m
```

### Address

```bash
modal node address [OPTIONS]
```

Display the listening addresses of a node.

**Example output:**
```
/ip4/192.168.1.100/tcp/9000/p2p/12D3KooWExample...
/ip4/127.0.0.1/tcp/9000/p2p/12D3KooWExample...
```

### Inspect

```bash
modal node inspect [OPTIONS]
```

Inspect a node's state (works for running or offline nodes).

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory |
| `--chain` | Show chain info |
| `--peers` | Show peer info |
| `--config` | Show configuration |

### Compare

```bash
modal node compare <PEER> [OPTIONS]
```

Compare local chain with a remote peer.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Local node directory |
| `--verbose` | Show block-by-block comparison |

### Logs

```bash
modal node logs [OPTIONS]
```

Tail the logs of a running node.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory |
| `--follow`, `-f` | Follow log output |
| `--lines <N>` | Number of lines to show |

### Stats

```bash
modal node stats [OPTIONS]
```

Display summary statistics from recent blocks.

## Network Operations

### Ping

```bash
modal node ping <PEER> [OPTIONS]
```

Ping a remote Modality node.

**Example:**
```bash
modal node ping /ip4/peer.modality.network/tcp/9000/p2p/12D3Koo...
```

### Sync

```bash
modal node sync [OPTIONS]
```

Sync blockchain from network peers.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory |
| `--peer <ADDR>` | Specific peer to sync from |
| `--from <HEIGHT>` | Start height |

## Maintenance

### Config

```bash
modal node config [OPTIONS]
```

View or modify node configuration.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory |
| `--get <KEY>` | Get config value |
| `--set <KEY=VALUE>` | Set config value |

### Clear

```bash
modal node clear [OPTIONS]
```

Clear both storage and logs from a node.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Node directory |
| `--confirm` | Skip confirmation prompt |

### Clear Storage

```bash
modal node clear-storage [OPTIONS]
```

Clear only storage (keep logs).

## Hub Commands

The contract hub is a collaborative server for multi-party contracts.

### Start Hub

```bash
modal hub start [OPTIONS]
```

Start the contract hub server.

**Options:**
| Option | Description |
|--------|-------------|
| `--detach` | Run in background |
| `--port <PORT>` | Listen port (default: 8080) |
| `--data <PATH>` | Data directory |

### Stop Hub

```bash
modal hub stop
```

### Hub Status

```bash
modal hub status
```

### Register Identity

```bash
modal hub register [OPTIONS]
```

Register your identity with the hub.

**Options:**
| Option | Description |
|--------|-------------|
| `--passfile <PATH>` | Identity passfile |
| `--hub <URL>` | Hub URL |

### Create Hub Contract

```bash
modal hub create <NAME> [OPTIONS]
```

Create a new contract on the hub.

**Options:**
| Option | Description |
|--------|-------------|
| `--description <DESC>` | Contract description |
| `--passfile <PATH>` | Creator identity |

### Grant Access

```bash
modal hub grant <CONTRACT_ID> [OPTIONS]
```

Grant access to a contract.

**Options:**
| Option | Description |
|--------|-------------|
| `--identity <ID>` | Identity to grant |
| `--role <ROLE>` | Role (reader/writer/admin) |
