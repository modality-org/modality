---
sidebar_position: 2
title: Installation
---

# Installation

## Prerequisites

- Git
- Rust toolchain (for building from source)

## Install from Source

Build the lean onboarding wrapper first if your goal is the first-contract
local flow:

```bash
# Clone the repo
git clone https://github.com/modality-org/modality.git
cd modality/rust

# Build the first-contract onboarding CLI
cargo build --release -p modal --no-default-features --features contract-onboarding

# Add to path
export PATH="$PATH:$(pwd)/target/release"

# Verify installation
modal --help
```

The lean wrapper includes `modal contract` and `modal id` commands without the
network, hub, node, predicate, or program surfaces.

Build the full wrapper when you need those broader command groups:

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

# Contract commands (lean default: no libp2p, wasmtime, or modality-lang)
cargo build -p modal-cli-contract

# Contract commands with all optional deps (P2P push/pull, model status, WASM upload)
cargo build -p modal-cli-contract --features full

# Program commands with WASM validation
cargo build -p modal-cli-program --features full

# Network info only (very lean: modal-networks + clap)
cargo build -p modal-cli-net

# Node management
cargo build -p modal-cli-node

# Predicate, program, chain, or network commands
cargo build -p modal-cli-predicate
cargo build -p modal-cli-program
cargo build -p modal-cli-chain
cargo build -p modal-cli-net

# Lean first-contract wrapper
cargo build -p modal --no-default-features --features contract-onboarding

# Full CLI (for integration testing)
cargo build -p modal
```

## Verify Installation

```bash
modal --help
modality model --help
```

Use `modal` for contract logs, identities, commits, status, and the first-contract
local flow. Use `modality` for model and rule authoring tasks such as
`modality model synthesize`, `modality model validate`, and `modality model lint`.
A successful onboarding install should make both command surfaces visible before
you start the first-contract guide.

For the lean onboarding wrapper, `modal --help` should show the first-contract
command surface, including:

```
contract   Contract related commands
id         ID and key related commands
passfile   Passfile related commands
status     Show status
commit     Commit changes
set        Set a state file value
```

You should not see the full runtime command groups such as `hub`, `node`,
`predicate`, `program`, or `chain` unless you built the full wrapper.

For the full wrapper, `modal --help` should also include broader runtime
commands such as:

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
