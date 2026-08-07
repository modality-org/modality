# CLI Smokes

These scripts exercise user-facing CLI flows that are too broad for a single
crate unit test.

## First Contract

```bash
cd rust
cargo build -p modal --no-default-features --features contract-onboarding
cd ..
tests/cli/run-first-contract-cli-smoke.sh
```

The smoke uses the source-built lean onboarding `modal` binary to create a
contract, create Alice and Bob passfiles, write their `.id` files into contract
state, commit the state with Alice's signature, and inspect status plus log
output.

To check the installed or source-built help surface before running a flow:

```bash
MODAL_BIN=/path/to/modal tests/cli/check-modal-help-surface.sh
```

The default check is the lean onboarding surface. Use
`MODAL_HELP_SURFACE=full` when testing a full wrapper build.
The check covers both `modal contract --help` and the documented
`modal c --help` alias used by the first-contract guide. It also checks the
first-contract subcommand help pages for the documented identity and commit
flags: `modal id create --path`, `modal id get --path`, `modal c commit`
with `--all`, `--sign`, and `--message`, plus `modal c set-named-id`.

Use `cargo build -p modal` when you are testing the full wrapper surface,
including node, network, hub, chain, predicate, and program commands.

To test another binary:

```bash
MODAL_BIN=/path/to/modal tests/cli/run-first-contract-cli-smoke.sh
```
