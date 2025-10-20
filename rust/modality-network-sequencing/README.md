# modality-network-sequencing

Sequencing and chain observation for Modality network consensus.

## Overview

This package provides the core functionality for sequencer nodes in the Modality network. Sequencers are a second class of consensus nodes that observe mining events and maintain the canonical chain without participating in mining themselves.

## Architecture

Sequencers have several key responsibilities:

1. **Chain Observation**: Listen to mining block gossip events and track the canonical chain
2. **Fork Choice**: Apply cumulative difficulty-based fork choice rules (implemented in the gossip handler)
3. **Consensus Participation**: Participate in the consensus protocol using the observed mining chain
4. **Data Serving**: Maintain a full datastore and serve block data to other nodes

## Key Components

### ChainObserver

The `ChainObserver` struct provides an API for tracking the canonical mining chain:

```rust
use modality_network_sequencing::ChainObserver;

let observer = ChainObserver::new(datastore);
observer.initialize().await?;

// Get current chain tip
let tip = observer.get_chain_tip().await;

// Get canonical blocks
let blocks = observer.get_all_canonical_blocks().await?;
```

## Usage

Sequencer nodes are started using the CLI:

```bash
modality net run-sequencer --dir /path/to/node/dir
```

Or with a specific config file:

```bash
modality net run-sequencer --config /path/to/config.json
```

## Differences from Miners

| Feature | Miners | Sequencers |
|---------|--------|------------|
| Mine blocks | ✅ Yes | ❌ No |
| Listen to mining gossip | ✅ Yes | ✅ Yes |
| Maintain canonical chain | ✅ Yes | ✅ Yes |
| Participate in consensus | ❌ No | ✅ Yes |
| Full datastore | ✅ Yes | ✅ Yes |

## Development

Run tests:

```bash
cargo test -p modality-network-sequencing
```

