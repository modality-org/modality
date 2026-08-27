---
sidebar_position: 1
title: Overview
---

# CLI Reference

The `modal` command-line tool is your interface to Modality.

## Installation

```bash
# First-contract onboarding wrapper
cd rust
cargo build --release -p modal --no-default-features --features contract-onboarding

# Language/model CLI used by the first-contract guide
cargo build --release -p modality
```

The lean onboarding wrapper exposes the contract and identity surfaces needed by
the first-contract guide: `modal contract`, `modal c`, `modal id`,
`modal passfile`, `modal status`, `modal pull`, `modal commit`, `modal diff`,
`modal set`, `modal repost`, `modal add-rule`, and `modal download`. It omits the
runtime-heavy hub, node, network, predicate, program, chain, local, run,
`killall`, and upgrade surfaces.

Build the full wrapper with `cargo build --release -p modal` when you need those
broader command groups. Use `modality` for model and rule authoring tasks such
as `modality model lint`, `modality model synthesize`, and
`modality model validate`.

## Command Groups

| Command | Alias | Description |
|---------|-------|-------------|
| `modal contract` | `modal c` | Contract management (create, commit, push, pull) |
| `modal id` | `modal identity` | Identity management (create, derive, get) |
| `modal passfile` | — | Passfile encryption/decryption |
| `modal hub` | — | Contract hub server and collaboration; full wrapper only |
| `modal predicate` | — | Predicate listing and testing; full wrapper only |
| `modal program` | — | Program management; full wrapper only |
| `modal node` | — | Network node operations; full wrapper only |
| `modal net` | `modal network` | Network information; full wrapper only |
| `modal local` | — | Local development utilities; full wrapper only |
| `modal run` | — | Quick node runners; full wrapper only |
| `modal chain` | — | Chain validation; full wrapper only |

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

# Pull commits (shortcut for modal contract pull)
modal pull http://hub.example.com/contracts/my-contract

# Show uncommitted changes (shortcut for modal contract diff)
modal diff

# Repost state from another contract
modal repost source-contract-id /source/path /local/path

# Add a rule to the current contract
modal add-rule rules/member-protection.modality

# Download a packed contract file
modal download http://hub.example.com/contracts/my-contract.pack

# Kill all local nodes (full wrapper only)
modal killall

# Upgrade to latest version (full wrapper only)
modal upgrade
```

## Quick Reference

```bash
# Identity
modal id create --path alice.passfile
modal id get --path alice.passfile

# Contract workflow
modal c create
modal c set-named-id /parties/alice.id alice.passfile
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
| `modal contract pull` | `modal pull` |
| `modal contract commit` | `modal commit` |
| `modal contract diff` | `modal diff` |
| `modal contract set` | `modal set` |
| `modal contract repost` | `modal repost` |
| `modal contract add-rule` | `modal add-rule` |
| `modal contract download` | `modal download` |
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
