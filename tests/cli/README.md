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
output. The status check asserts that both JSON and text output report the
replayed governing model state as `q1`, while the log check asserts that the
signed onboarding commit exposes Alice's signer ID and the commit message in
both JSON and text output. Together these cover the visible authority evidence
and human-readable contract context a new user sees when they run `modal c
status` and `modal c log`. It then commits a signed post-bootstrap state
update, asserts that status/log now expose the third committed entry while the
governing model state remains `q1`, and attempts an unsigned one, asserting
that the governing model rejects the unsigned path with the closest candidate
transition and missing `signed_by` predicate diagnostics.

To check the installed or source-built help surface before running a flow:

```bash
MODAL_BIN=/path/to/modal tests/cli/check-modal-help-surface.sh
```

The check first asserts that `modal --version` identifies the wrapper, then
checks the selected command surface. The default check is the lean onboarding
surface. Use `MODAL_HELP_SURFACE=full` when testing a full wrapper build. The
check covers both `modal contract --help` and the documented `modal c --help`
alias used by the first-contract guide. It also checks the first-contract
subcommand help pages for the documented identity and commit flags: `modal id
create --path`, `modal id get --path`, `modal c commit` with `--all`, `--sign`,
and `--message`, `modal c status` and `modal c log` inspection flags, plus
`modal c set-named-id`.

Use `cargo build -p modal` when you are testing the full wrapper surface,
including node, network, hub, chain, predicate, and program commands.

To test another binary:

```bash
MODAL_BIN=/path/to/modal tests/cli/run-first-contract-cli-smoke.sh
```

## Contract Evolution

```bash
cd rust
cargo build -p modal --no-default-features --features contract-onboarding
cd ..
tests/cli/run-contract-evolution-cli-smoke.sh
```

The evolution smoke creates a V1 contract, proves unsigned post-bootstrap
updates are rejected, appends a signed accumulated rule, rejects a replacement
model that reintroduces unsigned steady-state posts, accepts a replacement
model that adds Bob as another authorized signer, and then proves Bob-signed
V2 updates are accepted while unsigned V2 updates are still rejected.
