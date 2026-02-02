---
sidebar_position: 2
title: Contract Commands
---

# Contract Commands (`modal contract` / `modal c`)

## Create a Contract

```bash
modal contract create
modal c create
```

Creates a new contract in the current directory. Initializes `.contract/` for tracking commits.

## Checkout State

```bash
modal c checkout
```

Extracts the current state from commits to the `state/` directory for editing.

## Set State Values

```bash
# Set a value at a path
modal c set /path/to/file.txt "content"

# Set from a file
modal c set /data/config.json --file ./local-config.json
```

## Commit Changes

```bash
# Commit all changes (state + rules)
modal c commit --all -m "Description"

# Commit with signature
modal c commit --all --sign alice.passfile -m "Signed commit"

# Commit only state changes
modal c commit --state -m "State update"

# Commit an action
modal c commit --action '{"type":"DEPOSIT","amount":100}' --sign alice.passfile
```

## View History

```bash
# Show commit log
modal c log

# Show detailed log
modal c log --verbose
```

## Check Status

```bash
modal c status
modal status  # shortcut when in contract directory
```

## View Differences

```bash
modal c diff
```

## Push to Network/Hub

```bash
# Push to a hub
modal c push http://hub.example.com/contracts/my-contract

# Push to chain validators
modal c push /ip4/validator.modality.network/...
```

## Pull from Network/Hub

```bash
modal c pull http://hub.example.com/contracts/my-contract
```

## Get Contract Info

```bash
# Get contract ID
modal c id

# Get current commit ID  
modal c commit-id

# Get state value
modal c get /parties/alice.id
```
