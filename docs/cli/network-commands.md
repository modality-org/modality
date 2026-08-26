---
sidebar_position: 6
title: Network Commands
---

# Network Commands (`modal net` / `modal network`)

Query configured Modality networks and inspect full-wrapper network runtime data.
These commands are available in the full Rust wrapper, not in the lean
first-contract onboarding wrapper.

## Network Info

```bash
modal net info [NETWORK]
```

Display configured bootstrapper and DNS information for a named network.
`NETWORK` is positional and defaults to `mainnet`.

**Arguments:**
| Argument | Description |
|----------|-------------|
| `NETWORK` | Network name, such as `mainnet`, `testnet`, or a configured devnet |

**Example output:**
```
Modality Network Information

Network Name:     mainnet
Description:      ...
Bootstrappers:    2

Bootstrapper Addresses:
  /ip4/boot1.modality.network/tcp/9000/p2p/12D3Koo...
  /ip4/boot2.modality.network/tcp/9000/p2p/12D3Koo...

DNS Record:
  _dnsaddr.mainnet.modality.network
```

## Network Storage

```bash
modal net storage --config <CONFIG> [OPTIONS]
```

Inspect the datastore named by a node `config.json` and show canonical miner
block statistics.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to the node configuration file |
| `--detailed` | Show a detailed block list |
| `--epoch <EPOCH>` | Filter blocks by epoch |
| `--limit <LIMIT>` | Maximum detailed blocks to show; defaults to `10` |

## Mining Commands

### Sync Mining Data

```bash
modal net mining sync --config <CONFIG> --target <MULTIADDR> [OPTIONS]
```

Sync miner blocks from a specified node.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to the local node configuration file |
| `--target <MULTIADDR>` | Source node multiaddress |
| `--mode <MODE>` | Sync mode: `all`, `epoch`, or `range`; defaults to `all` |
| `--epoch <EPOCH>` | Epoch number, required when `--mode epoch` |
| `--from-index <INDEX>` | Start block index, required when `--mode range` |
| `--to-index <INDEX>` | End block index, required when `--mode range` |
| `--format <FORMAT>` | Output format: `summary` or `json`; defaults to `summary` |
| `--persist` | Persist synced blocks to the local datastore |

## Local Development

### List Local Nodes

```bash
modal local nodes [OPTIONS]
```

Find all running modal node processes.

**Options:**
| Option | Description |
|--------|-------------|
| `--verbose` / `-v` | Show verbose output with full paths |
| `--network <FILTER>` | Filter by network config path; supports trailing `*` wildcards |
| `--devnet` | Shorthand for `--network devnet*` |
| `--dir <DIR>` | Only show nodes in this directory or its subdirectories |

### Kill All Nodes

```bash
modal local killall-nodes [OPTIONS]
modal killall  # shortcut
```

Kill all running modal node processes.

**Options:**
| Option | Description |
|--------|-------------|
| `--force` / `-f` | Use `SIGKILL` instead of graceful shutdown |
| `--dry-run` | Show what would be killed without killing processes |
| `--network <FILTER>` | Filter by network config path; supports trailing `*` wildcards |
| `--devnet` | Shorthand for `--network devnet*` |
| `--dir <DIR>` | Only kill nodes in this directory or its subdirectories |

## Chain Commands

### Validate Chain

```bash
modal chain validate [OPTIONS]
```

Run local chain validation tests. Without `--test`, it runs all validation
tests against an in-memory datastore unless `--datastore` is supplied.

**Options:**
| Option | Description |
|--------|-------------|
| `--test <TEST>` / `-t <TEST>` | Test to run: `fork`, `gap`, `missing-parent`, `integrity`, `promotion`, or `duplicate-canonical`; may be repeated |
| `--datastore <PATH>` / `-d <PATH>` | Existing datastore directory; otherwise uses in-memory storage |
| `--json` | Output results as JSON |

### Heal Chain

```bash
modal chain heal --datastore <PATH> [OPTIONS]
```

Detect duplicate canonical blocks in a datastore and optionally mark duplicates
as orphaned.

**Options:**
| Option | Description |
|--------|-------------|
| `--datastore <PATH>` / `-d <PATH>` | Datastore directory to inspect or heal |
| `--dry-run` | Show what would be done |

## Quick Run Commands

Shortcuts for running different node types:

```bash
# Run a miner
modal run miner --dir ./my-node

# Run a validator
modal run validator --dir ./my-node

# Run an observer
modal run observer --dir ./my-node
```

These are equivalent to `modal node run-miner`, `modal node run-validator`, etc.
