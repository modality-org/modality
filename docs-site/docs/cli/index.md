---
sidebar_position: 1
title: Overview
---

# CLI Reference

The `modal` command-line tool is your interface to Modality.

## Command Overview

| Command | Description |
|---------|-------------|
| `modal contract` | Contract management (create, commit, push, pull) |
| `modal id` | Identity management (create, derive, get) |
| `modal predicate` | Predicate management and testing |
| `modal node` | Network node operations |
| `modal hub` | Contract hub operations |

## Quick Reference

```bash
# Create a contract
modal contract create

# Create identity
modal id create --path alice.passfile

# Commit changes
modal c commit --all --sign alice.passfile -m "message"

# Check status
modal c status

# Push to hub
modal c push http://hub.example.com/contracts/my-contract
```

## Shortcuts

| Full Command | Shortcut |
|--------------|----------|
| `modal contract` | `modal c` |
| `modal contract status` | `modal status` |
