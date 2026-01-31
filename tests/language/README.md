# Language Examples

This directory contains examples demonstrating the Modality language parser and CLI tools.

## Examples

- `01-simple-models/` - Basic models, diagrams, and formula checking
- `02-formulas/` - Formula syntax: modal operators, temporal operators, fixed points (mu-calculus)

## Running Tests

```bash
# Run formula tests
cd 02-formulas && ./run-tests.sh

# Or run Rust integration tests
cd ../../rust && cargo test -p modality-lang
```
