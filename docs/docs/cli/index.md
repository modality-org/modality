---
sidebar_position: 1
title: Overview
---

# CLI Reference

The `modal` command-line tool is your interface to Modality.

## Installation

```bash
# macOS/Linux
curl -fsSL https://get.modality.org | sh

# Or build from source
cd rust && cargo build --release
```

## Command Groups

| Command | Alias | Description |
|---------|-------|-------------|
| `modal contract` | `modal c` | Contract management (create, commit, push, pull) |
| `modal id` | `modal identity` | Identity management (create, derive, get) |
| `modal passfile` | — | Passfile encryption/decryption |
| `modal predicate` | — | Predicate listing and testing |
| `modal program` | — | Program management |
| `modal node` | — | Network node operations |
| `modal net` | `modal network` | Network information |
| `modal local` | — | Local development utilities |
| `modal run` | — | Quick node runners |
| `modal chain` | — | Chain validation |

## Global Commands

```bash
# Show version
modal --version
modal -v

# Show help
modal --help
modal <command> --help

# Show status (in contract directory)
modal status

# Kill all local nodes
modal killall

# Upgrade to latest version
modal upgrade
```

## Quick Reference

```bash
# Identity
modal id create --path alice.passfile
modal id get --path alice.passfile

# Contract workflow
modal c create
modal c set /parties/alice.id --named alice
modal c commit --all --sign alice.passfile -m "Initial setup"
modal c status
modal c log

# Push/Pull
modal c push http://hub.example.com/contracts/my-contract
modal c pull http://hub.example.com/contracts/my-contract

# Predicates
modal predicate list
modal predicate info signed_by
modal predicate test signed_by --data '{"path":"/alice.id","signature":"..."}'
```

## Shortcuts

| Full Command | Shortcut |
|--------------|----------|
| `modal contract` | `modal c` |
| `modal contract status` | `modal status` |
| `modal identity` | `modal id` |
| `modal network` | `modal net` |
| `modal local killall-nodes` | `modal killall` |

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MODAL_NODE_PATH` | Default node directory | `./node` |
| `MODAL_NETWORK` | Network (mainnet/testnet) | `mainnet` |
| `MODAL_HUB_URL` | Default hub URL | — |
| `MODAL_PASSFILE` | Default passfile path | — |
