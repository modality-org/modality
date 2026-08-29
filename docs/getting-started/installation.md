---
sidebar_position: 2
title: Installation
---

# Installation

## Prerequisites

- Git
- Rust toolchain (for building from source)

The repository pins the Rust compiler in `rust/rust-toolchain.toml`. Before
measuring source-build onboarding, confirm Cargo is using that pinned toolchain
and that the locked dependency graph resolves with it:

```bash
cd modality/rust
rustup show active-toolchain
cargo metadata --locked --no-deps --format-version 1
```

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
exactly those five entries in the emitted order,
and checks that the manifest covers exactly `bin/modal`, `README.txt`, and
`PROVENANCE.txt` plus `EVIDENCE-BUNDLE.txt`. The provenance file records the
source revision, version, profile, features, platform, and expected help
surface. The archive producer now fails before emitting release evidence when
the source revision is not a lowercase hex commit token, so `unknown` or
hand-written revision notes cannot become the advertised archive provenance.
The evidence manifest names the replayable evidence bundle, artifact,
version, source revision, profile, feature set, exact help surface, binary,
provenance file, checksum file, and post-unpack checks.
With `MODAL_ONBOARDING_ARTIFACT_SMOKE=1`, the download verifier now requires a
same-revision `MODALITY_BIN`, checks the unpacked help surface, and runs the
first-contract CLI smoke against the unpacked `modal` binary. The language CLI
revision may be the exact provenance revision or a longer matching hex prefix
for the same commit. Set
`MODAL_ONBOARDING_ARCHIVE_EXPECT_REV=<commit>` when release evidence must fail
if the built `modal` binary is stale or came from a different source revision.
The `.github/workflows/onboarding-release-archive.yml` workflow wires this into
GitHub Actions without publishing anything: it builds `modality`, builds the
lean release `modal` wrapper with `contract-onboarding`, runs the archive
readiness check with the expected source revision, then uploads the verified
replayable archive bundle, detached tarball checksum, and
`VERIFY-DOWNLOAD.txt` verification recipe as a workflow artifact. The
recipe names, in order, the artifact, exact downloaded directory entries,
expected source revision, expected profile, expected feature set, expected help surface, and verifier command.
The workflow summary prints the exact `gh run download` command for that run and the
matching `MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=<commit>` replay verifier
command, plus the optional `MODAL_ONBOARDING_ARTIFACT_SMOKE=1` replay command
that checks the downloaded binary's version, recorded help surface, and
first-contract path when `MODALITY_BIN=/path/to/modality` was built from the
same source revision. The
uploaded Actions artifact and the replay summary both use the
`modal-linux-x86_64-release-archive-<source-revision>` artifact name, so the
download command stays tied to the exact source revision under test. The
summary also includes a local replay block that creates an `artifact_dir`,
downloads the named artifact there, runs the pinned verifier against that
directory, and then shows the optional smoke replay against the same directory.
The same summary records the exact Actions artifact name, the GitHub artifact
digest emitted by the `upload-artifact` `artifact-digest` output, and fails if that digest is missing.
It
also records the exact detached tarball checksum line
that the local replay verifies, so a manual release-candidate run keeps the
platform archive identity visible next to the Modality archive identity and
replay commands.
Run `workflow_dispatch` manually with this guarded handoff so an empty or
wrong-revision run lookup stops before any artifact is trusted:

```bash
source_rev="$(git rev-parse HEAD)"
gh workflow run "Onboarding Release Archive" --ref main
run_id="$(gh run list --workflow "Onboarding Release Archive" --branch main --event workflow_dispatch --commit "$source_rev" --limit 1 --json databaseId,headSha --jq '.[0] | select(.headSha == "'"$source_rev"'") | .databaseId')"
test -n "$run_id"
gh run watch "$run_id" --exit-status
```

Tag a commit as `modal-v*` when you want tag-scoped release evidence. The
workflow summary is the release-candidate handoff surface: it names the source revision,
exact Actions artifact, GitHub artifact digest, detached tarball checksum, exact
download command, pinned verifier command, and optional smoke replay command for
that run.
For checkpoint review, treat the minimum evidence bundle as complete only when
the handoff includes the exact source revision, matching workflow run id, exact
Actions artifact name, GitHub artifact digest, detached tarball checksum line,
downloaded-artifact verifier result, and, when first-contract replay is claimed,
the same-revision `MODALITY_BIN` smoke replay result.
After downloading that artifact, verify it before unpacking or trusting the
binary:

```bash
tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir
```

The download check expects exactly one `modal-*.tar.gz` plus its matching
`.sha256` sidecar plus exactly one `VERIFY-DOWNLOAD.txt` recipe, rejects any
other top-level entries in the downloaded artifact directory, requires those
entries to be regular non-symlink files with canonical `0644` modes, verifies
the detached checksum first,
requires the detached checksum sidecar to be one canonical SHA-256 line naming
exactly that one archive,
then rechecks the exact archive members in the emitted order, internal checksum
manifest entries in the emitted order, executable `bin/modal`,
regular non-symlink unpacked files with expected payload modes (`bin/modal` as
`0755`; text and checksum files as `0644`), provenance metadata, source revision,
single provenance marker, replayable evidence bundle marker, and the recipe's
archive, checksum, revision, and verifier command.
The producer smoke also mutates the downloaded archive, checksum sidecar, and
recipe into directories, symlinks, or non-canonical modes, and renames the
checksum sidecar without renaming the archive, so top-level payload checks and
sidecar pairing checks stay covered by executable negative evidence.
The archive filename must match the version, OS, architecture, and profile
recorded in provenance, so a consistently renamed tarball, sidecar, and recipe
still fails before the binary is trusted.
The provenance file must keep exactly one value for version, profile, features,
help surface, OS, and architecture. OS and architecture values must stay
archive-safe lowercase platform tokens. The profile must be one of the supported build profiles
(`debug` or `release`), and the feature set must be one of the supported wrapper feature sets
(`contract-onboarding` or `full`), so stale, partial, or unsupported provenance fails
before the binary is trusted.
The source revision must also be a lowercase hex commit token, so `unknown` or
hand-edited revision notes fail even when the checksums and replay recipe are
rebuilt consistently around them.
The archive producer also fails before emitting release evidence when the OS or
architecture provenance is not an archive-safe lowercase platform token, so
unsafe platform metadata cannot be advertised and then left for the downloaded
artifact verifier to catch later.
The archive producer also fails before emitting release evidence when the build
profile is not one of the supported values (`debug` or `release`), so ad-hoc
profile labels cannot become installer provenance even when `MODAL_BIN` points
at an explicit binary.
The archive producer also fails before emitting release evidence when the
advertised help surface or wrapper feature set is not one of the supported
values, so experimental labels cannot be published as replayable installer
provenance.
The provenance marker must also appear exactly once, so hand-merged provenance
preambles fail before any field values are trusted.
If the provenance version string carries an embedded revision marker, that
revision must match the single source revision recorded by provenance, so a
consistently renamed bundle with stale version metadata still fails before the
binary is trusted.
The help surface recorded in provenance must also be one of the supported
surfaces (`lean` or `full`), and optional smoke replay checks that the unpacked
binary reports the exact version named by provenance before checking that exact
help surface instead of assuming a default. A consistently edited README,
provenance file, and recipe that invent a new surface still fails before the
binary is trusted.
The README artifact marker must appear exactly once, and the README must repeat
the same version, source revision, profile, feature set, and help surface as
provenance, so stale human-facing bundle notes fail before the binary is
trusted, including stale version notes.
Those README metadata fields must also be single-valued, so
hand-edited notes with both current and stale values fail before the binary is
trusted.
The evidence bundle marker must appear exactly once. The evidence bundle must
also keep naming the same archive artifact, version, profile, feature set, and
exact help surface as provenance plus the checked binary, provenance file,
checksum manifest, and post-unpack smoke checks, so stale or hand-edited bundles
cannot omit or drift from the replay ingredients while preserving checksums.
Those evidence manifest fields must also be single-valued, so hand-merged
manifests with both current and stale replay ingredients fail before the binary
is trusted.
The recipe's artifact section must name exactly the one downloaded archive, so
missing artifact metadata fails before unpacking; stale or hand-edited extra artifact names fail before unpacking.
The recipe title must also appear exactly once as the first line, so stale
notes inserted before it or hand-merged recipe preambles fail before the binary
is trusted.
The provenance file must name exactly one source revision, so ambiguous or
hand-merged provenance fails before the binary is trusted.
It also checks that the recipe names exactly the expected downloaded directory
entries in the emitted order: the archive, its `.sha256` sidecar, and
`VERIFY-DOWNLOAD.txt`.
The recipe's expected source revision section must name exactly the same single
revision as the unpacked provenance, so missing revision metadata or stale
duplicate revisions fail before the binary is trusted.
The recipe's expected profile and feature-set sections must also name exactly
the same single values as the unpacked provenance.
The recipe's expected help surface section must name exactly the same single
help surface as the unpacked provenance, so a stale lean-versus-full replay
recipe fails before the binary is trusted.
Those recipe sections must stay in the emitted order, so a hand-shuffled recipe
fails even when each section still carries the expected value.
Set
`MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=<commit>` when a downloaded artifact must
fail unless its internal provenance matches one exact source revision.
Set `MODAL_ONBOARDING_ARTIFACT_SMOKE=1` only when a same-revision
`MODALITY_BIN` is available; the verifier now rejects missing `MODALITY_BIN`
rather than silently downgrading the requested first-contract smoke to
archive-only verification, and the producer smoke now proves that a
non-executable `MODALITY_BIN` is rejected before any replay can pass.
Unsupported smoke flag values now fail, too, instead of silently downgrading to archive-only
verification, so a mistyped replay request cannot look like a successful
archive-only check.
The uploaded `VERIFY-DOWNLOAD.txt` repeats the expected source revision and the
exact two-command verification section, plus the optional `MODAL_ONBOARDING_ARTIFACT_SMOKE=1`
and `MODALITY_BIN=/path/to/modality` same-revision replay environment, so the artifact
directory remains self-describing after download. The detached checksum command,
exact verifier command, smoke replay environment, and smoke replay description
must each appear exactly once, and the verification section must stay present
with no extra command added, so hand-merged recipes with missing, stale
duplicate, or extra replay commands fail before the binary is trusted. The
smoke replay note must remain the exact final three-line trailer, so a stale
inserted line cannot split the optional smoke instructions while preserving both
required strings. After those targeted checks pass, the full recipe must still
match the canonical emitted text exactly, so hand-inserted prose between
otherwise valid sections fails before the binary is trusted.
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
