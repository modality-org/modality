# Modality Language Examples

This directory contains examples demonstrating how to use the Modality Language Parser.

## Rust Examples

- **`parse_example.rs`** - Basic parsing example
- **`compare_parsers.rs`** - Compare hand-written vs LALRPOP parsers
- **`lalrpop_example.rs`** - LALRPOP parser usage
- **`parse_all_models.rs`** - Parse multiple models from a file
- **`mermaid_example.rs`** - Generate Mermaid diagrams
- **`simple_mermaid.rs`** - Simple Mermaid diagram generation

## WASM Examples

See the [`wasm/`](wasm/) directory for WebAssembly examples:

- **`example.html`** - Browser-based demo
- **`node-example.cjs`** - Node.js example
- **`README.md`** - Detailed WASM documentation

## Model Files

The [`models/`](models/) directory contains example Modality language files for testing.

## Running Examples

### Rust Examples

From `rust/`:

```bash
# Examples that read a file take its path
cargo run -p modality-lang --example parse_example -- modality-lang/examples/models/SimpleExamples.modality

# The others run as they are
cargo run -p modality-lang --example model_checker_demo

# Build every example
cargo build -p modality-lang --examples
```

### Contracts and demos

- **`contract-evolution.modality`**, **`evolving-dao.modality`** and
  **`full-agent-demo.modality`** - models, rules and parse-only tests; check
  a formula with `modal model check <file> -f <formula>`
- **`cli-workflow.sh`** - the agent contract commands of the `modality`
  binary built with its `contract` feature

### WASM Examples

See the [WASM examples README](wasm/README.md) for detailed instructions. 