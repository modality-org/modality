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

```bash
# Run a specific example
cargo run --example parse_example

# Run all examples
cargo test --examples
```

### WASM Examples

See the [WASM examples README](wasm/README.md) for detailed instructions. 