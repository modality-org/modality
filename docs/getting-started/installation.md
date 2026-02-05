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

# Build
cargo build --release

# Add to path
export PATH="$PATH:$(pwd)/target/release"

# Verify installation
modal --version
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
