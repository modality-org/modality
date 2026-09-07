---
sidebar_position: 4
title: Node Commands
---

# Node Commands (`modal node`)

Manage network nodes in the full Rust wrapper. These commands are available in
full builds, not in the lean first-contract onboarding wrapper.

Most node commands resolve configuration from `--config <CONFIG>` or from
`--dir <DIR>/config.json`. If neither flag is supplied, commands that operate on
one node default to the current directory.

## Create

```bash
modal node create [OPTIONS]
```

Create a node directory with `config.json` and `node.modal_passfile`.

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Node directory to create; defaults to the current directory when no `config.json` exists |
| `--node-id <NODE_ID>` | Existing peer ID to record; otherwise a new identity is generated |
| `--data-dir <DATA_DIR>` | Data directory written into config; defaults to `./data` |
| `--bootstrappers <ADDRS>` | Comma-separated bootstrapper multiaddrs |
| `--network <NETWORK>` | Network preset such as `testnet`, `devnet1`, `devnet2`, or `devnet3` |
| `--testnet` | Enable the testnet preset |
| `--from-config <CONFIG>` | Merge settings from an existing config file |
| `--from-passfile <PASSFILE>` | Import an existing node identity passfile |
| `--from-template <TEMPLATE>` | Load a bundled template such as `devnet1/node1` |
| `--use-mnemonic` | Generate or import the node key from a BIP39 mnemonic |
| `--mnemonic-words <WORDS>` | Mnemonic word count; defaults to `12` |
| `--mnemonic-phrase <PHRASE>` | Existing mnemonic phrase to import |
| `--account <INDEX>` | BIP44 account index; defaults to `0` |
| `--change <INDEX>` | BIP44 change index; defaults to `0` |
| `--index <INDEX>` | BIP44 address index; defaults to `0` |
| `--passphrase <PASSPHRASE>` | Optional BIP39 passphrase |
| `--no-store-mnemonic` | Do not store the mnemonic in the passfile |
| `--logs-enabled <BOOL>` | Enable or disable file logging |
| `--log-level <LEVEL>` | `error`, `warn`, `info`, `debug`, or `trace`; defaults to `info` |
| `--bootup-enabled <BOOL>` | Enable or disable bootup tasks |
| `--bootup-minimum-genesis-timestamp <TIMESTAMP>` | Minimum genesis timestamp for pruning |
| `--bootup-prune-old-genesis-blocks <BOOL>` | Enable pruning of old genesis blocks |
| `--enable-autoupgrade` | Enable autoupgrade |
| `--autoupgrade-base-url <URL>` | Autoupgrade base URL |
| `--autoupgrade-branch <BRANCH>` | Autoupgrade branch |
| `--autoupgrade-check-interval-secs <SECS>` | Autoupgrade check interval |

**Creates:**
```text
node/
|-- config.json
|-- node.modal_passfile
|-- data/
`-- logs/
```

## Lifecycle

### Start

```bash
modal node start [OPTIONS]
```

Start a node in the background. The command writes `node.pid` in the node
directory.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--node-type <TYPE>` | `miner`, `observer`, `validator`, or `server`; otherwise resolved from config |

### Stop, Restart, Kill, and PID

```bash
modal node stop [OPTIONS]
modal node restart [OPTIONS]
modal node kill [OPTIONS]
modal node pid [OPTIONS]
```

**Shared options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |

`stop`, `restart`, and `kill` also accept `--force` / `-f` to use `SIGKILL`
instead of graceful `SIGTERM`. `restart` accepts `--node-type <TYPE>` with the
same values as `start`.

## Foreground Run Commands

```bash
modal node run [OPTIONS]
modal node run-miner [OPTIONS]
modal node run-validator [OPTIONS]
modal node run-observer [OPTIONS]
modal node run-noop [OPTIONS]
```

All foreground run commands accept:

| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |

`modal node run` additionally accepts `--enable-consensus`, which is deprecated;
prefer config-driven node roles.

The top-level quick-run aliases use the same options:

```bash
modal run miner --dir ./my-node
modal run validator --dir ./my-node
modal run observer --dir ./my-node
```

## Information

### Info and Stats

```bash
modal node info [OPTIONS]
modal node stats [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--verbose` / `-v` | Show extended information |
| `--sample-recent-blocks <COUNT>` | For `stats`, number of recent blocks to sample; defaults to `1000` |

### Address

```bash
modal node address [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--one` / `-1` | Show only one address |
| `--prefer-public` | Prefer public IP addresses |
| `--prefer-local` | Prefer loopback or local IP addresses |

### Inspect

```bash
modal node inspect [COMMAND] [KEY|INDEX] [OPTIONS]
```

Inspect local datastore state. `COMMAND` may be `general`, `mining`, `blocks`,
`block <INDEX>`, or `datastore-get <KEY>`. `--level <LEVEL>` remains available
for backward compatibility with `general`, `mining`, and `blocks`.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--level <LEVEL>` | Backward-compatible inspection level |

### Compare

```bash
modal node compare <PEER> [OPTIONS]
```

Compare the local chain with a remote peer ID or full multiaddr.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--timeout-secs <SECS>` | Timeout for network requests; defaults to `30` |
| `--precise` | Find the exact fork point with binary search |

### Logs

```bash
modal node logs [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--lines <COUNT>` / `-n <COUNT>` | Number of lines to show; defaults to `50` |
| `--follow` / `-f` | Follow log output |
| `--offline` | Show logs even when the node is not running |

## Network Operations

### Ping

```bash
modal node ping --target <MULTIADDR> [OPTIONS]
```

Ping a remote Modality node.

**Options:**
| Option | Description |
|--------|-------------|
| `--target <MULTIADDR>` | Peer multiaddr to ping |
| `--times <COUNT>` | Number of ping attempts; defaults to `1` |
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |

### Sync

```bash
modal node sync [OPTIONS]
```

Sync blockchain data from bootstrappers configured for the local node.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--block-height-minus <COUNT>` | Stop this many blocks before the highest known block height; defaults to `10` |
| `--max-peers <COUNT>` | Maximum peers to attempt; defaults to `5` |
| `--timeout-secs <SECS>` | Timeout per peer sync attempt; defaults to `30` |

## Maintenance

### Config

```bash
modal node config [OPTIONS]
```

View or modify `config.json`.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--show` | Show current configuration |
| `--set-listeners <ADDRS>` | Replace listener addresses with a comma-separated list |
| `--add-listener <ADDR>` | Add one listener address |
| `--remove-listener <ADDR>` | Remove one listener address |
| `--set-bootstrappers <ADDRS>` | Replace bootstrappers with a comma-separated list |
| `--add-bootstrapper <ADDR>` | Add one bootstrapper address |
| `--remove-bootstrapper <ADDR>` | Remove one bootstrapper address |
| `--replace-ip <FROM=TO>` | Replace IP addresses in listeners and bootstrappers |
| `--enable-autoupgrade` | Enable autoupgrade |
| `--disable-autoupgrade` | Disable autoupgrade |
| `--merge-in <FILE>` | Merge settings from a JSON file |
| `--dry-run` | Show merge changes without modifying the config |

### Clear

```bash
modal node clear [OPTIONS]
modal node clear-storage [OPTIONS]
```

Clear both storage and logs, or only storage.

**Options:**
| Option | Description |
|--------|-------------|
| `--config <CONFIG>` | Path to `config.json` |
| `--dir <DIR>` | Node directory containing `config.json` |
| `--yes` / `-y` | Skip the confirmation prompt |
