# Modal

`modal` is a binary alias of the `modality` CLI. It uses the same command
tree and feature flags; the process name stays `modal`.

## Installation

```bash
cargo install --path .
```

Lean onboarding surface:

```bash
cargo build --release -p modal --no-default-features --features contract-onboarding
```

Full network surface (default):

```bash
cargo build --release -p modal
```
