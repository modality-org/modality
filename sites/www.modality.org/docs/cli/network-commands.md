---
sidebar_position: 6
title: Network Commands
---

# Network Commands (`modal net` / `modal network`)

Query and interact with Modality networks.

## Network Info

```bash
modal net info [OPTIONS]
```

Display information about a Modality network.

**Options:**
| Option | Description |
|--------|-------------|
| `--network <NAME>` | Network name (mainnet/testnet) |
| `--peer <ADDR>` | Query specific peer |

**Example output:**
```
NETWORK: mainnet
  Chain height: 1,234,567
  Active validators: 42
  Active miners: 128
  Contracts: 5,432
  
BOOTSTRAP PEERS:
  /ip4/boot1.modality.network/tcp/9000/p2p/12D3Koo...
  /ip4/boot2.modality.network/tcp/9000/p2p/12D3Koo...
```

## Network Storage

```bash
modal net storage [OPTIONS]
```

Inspect network datastore and show statistics.

**Options:**
| Option | Description |
|--------|-------------|
| `--node <PATH>` | Node directory |
| `--verbose` | Show detailed breakdown |

## Mining Commands

### Sync Mining Data

```bash
modal net mining sync [OPTIONS]
```

Sync miner blocks from a specified node.

**Options:**
| Option | Description |
|--------|-------------|
| `--from <PEER>` | Source peer address |
| `--node <PATH>` | Local node directory |

## Local Development

### List Local Nodes

```bash
modal local nodes
```

Find all running modal node processes.

### Kill All Nodes

```bash
modal local killall-nodes
modal killall  # shortcut
```

Kill all running modal node processes.

## Chain Commands

### Validate Chain

```bash
modal chain validate [OPTIONS]
```

Validate the local chain for consistency.

**Options:**
| Option | Description |
|--------|-------------|
| `--node <PATH>` | Node directory |
| `--from <HEIGHT>` | Start height |
| `--to <HEIGHT>` | End height |
| `--verbose` | Show validation details |

### Heal Chain

```bash
modal chain heal [OPTIONS]
```

Attempt to repair chain inconsistencies.

**Options:**
| Option | Description |
|--------|-------------|
| `--node <PATH>` | Node directory |
| `--backup` | Create backup before healing |
| `--dry-run` | Show what would be done |

## Quick Run Commands

Shortcuts for running different node types:

```bash
# Run a miner
modal run miner --path ./my-node

# Run a validator
modal run validator --path ./my-node

# Run an observer
modal run observer --path ./my-node
```

These are equivalent to `modal node run-miner`, `modal node run-validator`, etc.
