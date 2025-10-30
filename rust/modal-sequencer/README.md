# modal-sequencer

Blockchain sequencer for Modality - observes the mining chain without mining blocks.

## Overview

This package provides the `Sequencer` struct that wraps `modal-observer` functionality
to track the canonical mining chain without participating in mining itself.

## Usage

```rust
use modal_sequencer::{Sequencer, SequencerConfig};
use modal_datastore::NetworkDatastore;
use std::sync::Arc;
use tokio::sync::Mutex;

// Create datastore
let datastore = Arc::new(Mutex::new(NetworkDatastore::new(storage_path).await?));

// Create and initialize sequencer
let sequencer = Sequencer::new_default(datastore).await?;
sequencer.initialize().await?;

// Get current chain tip
let tip = sequencer.get_chain_tip().await;
```

## Features

- **Chain Observation**: Uses `modal-observer` to track the canonical mining chain
- **Fork Choice**: Applies cumulative difficulty-based fork choice rules
- **No Mining**: Observes the chain without participating in mining

## Architecture

The sequencer is a lightweight wrapper around `modal-observer::ChainObserver` that
provides a simple API for nodes that want to observe the mining chain without
mining blocks themselves.

