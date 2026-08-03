# CLI Smokes

These scripts exercise user-facing CLI flows that are too broad for a single
crate unit test.

## First Contract

```bash
cd rust
cargo build -p modal
cd ..
tests/cli/run-first-contract-cli-smoke.sh
```

The smoke uses the source-built `modal` binary to create a contract, create
Alice and Bob passfiles, write their `.id` files into contract state, commit
the state with Alice's signature, and inspect status plus log output.

To test another binary:

```bash
MODAL_BIN=/path/to/modal tests/cli/run-first-contract-cli-smoke.sh
```
