---
sidebar_position: 2
title: Installation
---

# Installation

## Prerequisites

- Git
- Rust toolchain (for building from source)

## Install from Source

```bash
# Clone the repo
git clone https://github.com/modality-org/modality.git
cd modality/rust

# Build the full CLI
cargo build --release -p modal

# Add to path
export PATH="$PATH:$(pwd)/target/release"

# Verify installation
modal --version
```

## Development Builds

The Modal CLI is split into domain crates for faster incremental builds. When working on a specific area, build only that crate:

```bash
cd modality/rust

# Hub server work
cargo build -p modal-cli-hub

# Contract commands
cargo build -p modal-cli-contract

# Node management
cargo build -p modal-cli-node

# Predicate, program, chain, or network commands
cargo build -p modal-cli-predicate
cargo build -p modal-cli-program
cargo build -p modal-cli-chain
cargo build -p modal-cli-net

# Full CLI (for integration testing)
cargo build -p modal
```

## Verify Installation

```bash
modal --help
```

You should see the available commands:

```
modal - Modality CLI

USAGE:
    modal <COMMAND>

COMMANDS:
    contract   Contract management
    id         Identity management
    predicate  Predicate operations
    node       Network node operations
    hub        Contract hub operations
    help       Print help
```
