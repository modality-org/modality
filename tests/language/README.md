# Language Examples

This directory contains examples demonstrating the Modality language parser and CLI tools.

## Examples

- `01-simple-models/` - Basic models, diagrams, and formula checking
- `02-formulas/` - Formula syntax: modal operators, temporal operators, fixed points (mu-calculus)
- `03-first-contract/` - Parser-backed source of truth for the canonical first-contract onboarding path
- `check-first-contract-synthesize-cli.sh` - Real CLI smoke for synthesizing the first-contract witness model from its rule with `--verify` when a `modality` binary is available
- `check-rule-synthesize-cli.sh` - Real CLI smoke for synthesizing a signed-`POST` witness model from a rule formula block with `--verify` and a review bundle when a `modality` binary is available
- `check-model-lint-cli.sh` - Real CLI smoke for the vacuous `[+ACTION] true` lint warning in standalone formulas and rule formula blocks when a `modality` binary is available

## Running Tests

```bash
# Run the source-built onboarding smoke test
./run-onboarding-tests.sh

# Reuse a built or cached modality binary instead of cargo run
MODALITY_BIN=/path/to/modality ./run-onboarding-tests.sh

# Run the model lint CLI smoke against a built or cached modality binary
MODALITY_BIN=/path/to/modality ./check-model-lint-cli.sh

# Run the first-contract synthesis CLI smoke against a built or cached binary
MODALITY_BIN=/path/to/modality ./check-first-contract-synthesize-cli.sh

# Run the signed-POST rule synthesis CLI smoke against a built or cached binary
MODALITY_BIN=/path/to/modality ./check-rule-synthesize-cli.sh

# Run formula tests
cd 02-formulas && ./run-tests.sh

# Or run Rust integration tests
cd ../../rust && cargo test -p modality-lang
```
