---
sidebar_position: 4
title: Node Commands
---

# Node Commands (`modal node`)

## Create a Node

```bash
modal node create --path ./my-node
```

## Start/Stop a Node

```bash
modal node start --path ./my-node
modal node stop --path ./my-node
modal node restart --path ./my-node
```

## View Node Info

```bash
modal node info --path ./my-node
modal node address --path ./my-node
modal node logs --path ./my-node
```

## Run a Node (Foreground)

```bash
# Run as miner
modal node run-miner --path ./my-node

# Run as validator
modal node run-validator --path ./my-node

# Run as observer
modal node run-observer --path ./my-node
```

## Sync with Network

```bash
modal node sync --path ./my-node
```

## Hub Commands (`modal hub`)

### Start the Hub

```bash
modal hub start --detach
modal hub status
modal hub stop
```

### Register Identity

```bash
modal hub register
```

### Create Contract on Hub

```bash
modal hub create "Contract Name" --description "Description"
```

### Grant Access

```bash
modal hub grant <contract_id> --identity <id> --role writer
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MODAL_NODE_PATH` | Default node directory |
| `MODAL_NETWORK` | Default network (mainnet/testnet) |
| `MODAL_HUB_URL` | Default hub URL for push/pull |
