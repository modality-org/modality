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

Local source builds and temporary Cargo-root installs are verified by the
onboarding smokes. Git URL installs are measured by
`tests/cli/check-modal-git-install-readiness.sh`, which installs `modal` into a
temporary Cargo root, checks the installed help surface, and runs the
first-contract CLI smoke when a built `modality` binary is supplied.
Set `MODAL_ONBOARDING_GIT_REV=<commit>` to pin the exact Git revision under
test for release checklists or CI evidence.
Release-archive-shaped binary bundles are measured by
`tests/cli/check-modal-release-archive-readiness.sh`, which creates a
`modal-<version>-<os>-<arch>-<profile>.tar.gz` containing `bin/modal` and
`README.txt` plus `PROVENANCE.txt`, `EVIDENCE-BUNDLE.txt`, and `SHA256SUMS`,
unpacks it, verifies the checksum manifest, checks that the archive contains
exactly those five entries,
and checks that the manifest covers exactly `bin/modal`, `README.txt`, and
`PROVENANCE.txt` plus `EVIDENCE-BUNDLE.txt`. The provenance file records the
source revision, version, profile, features, platform, and expected help
surface. The evidence manifest names the replayable evidence bundle, artifact,
source revision, binary, provenance file, checksum file, and post-unpack checks.
It checks the unpacked help surface and runs the first-contract CLI smoke when a
built `modality` binary is supplied. Set
`MODAL_ONBOARDING_ARCHIVE_EXPECT_REV=<commit>` when release evidence must fail
if the built `modal` binary is stale or came from a different source revision.
The `.github/workflows/onboarding-release-archive.yml` workflow wires this into
GitHub Actions without publishing anything: it builds `modality`, builds the
lean release `modal` wrapper with `contract-onboarding`, runs the archive
readiness check with the expected source revision, then uploads the verified
replayable archive bundle, detached tarball checksum, and
`VERIFY-DOWNLOAD.txt` verification recipe as a workflow artifact.
Run it manually with
`workflow_dispatch`, or tag a commit as `modal-v*` when you want tag-scoped
release evidence.
After downloading that artifact, verify it before unpacking or trusting the
binary:

```bash
tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir
```

The download check expects exactly one `modal-*.tar.gz` plus its matching
`.sha256` sidecar plus exactly one `VERIFY-DOWNLOAD.txt` recipe, rejects any
other top-level entries in the downloaded artifact directory, requires those
entries to be regular non-symlink files, verifies the detached checksum first,
requires the detached checksum sidecar to name exactly that one archive,
then rechecks the exact archive members, internal checksum manifest, executable `bin/modal`,
regular non-symlink unpacked files with expected payload modes (`bin/modal` as
`0755`; text and checksum files as `0644`), provenance, source revision, replayable
evidence bundle marker, and the recipe's archive, checksum, revision, and verifier command.
Set
`MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=<commit>` when a downloaded artifact must
fail unless its internal provenance matches one exact source revision.
The uploaded `VERIFY-DOWNLOAD.txt` repeats the expected source revision and the
exact verifier command so the artifact directory remains self-describing after
download.
External crates.io-style packaging is tracked separately:
`tests/cli/check-modal-package-readiness.sh` reports the current blocker until
the workspace CLI crates that `modal` depends on are available from the
registry. Its blocker output names the selected direct workspace dependencies
(`modal-cli-contract`, `modal-common`, and `modality`) and the selected package
closure (`modal-cli-common`, `modal-cli-contract`, `modal-common`, `modality`,
and `modality-lang`) that must be covered by an external package or installer
plan.

For the language CLI, `modality --help` should only expose the model command
group, and `modality model --help` should expose the parser and review tools:

```
model      Model related commands
check      Check a formula against a model
synthesize Synthesize a model from a template
validate   Validate a contract model
lint       Lint governance formulas
```

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
