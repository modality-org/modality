# CLI Smokes

These scripts exercise user-facing CLI flows that are too broad for a single
crate unit test.

## First Contract

```bash
cd rust
cargo build -p modal --no-default-features --features contract-onboarding
cargo build -p modality
cd ..
tests/cli/run-first-contract-cli-smoke.sh
```

The smoke uses the source-built lean onboarding `modal` binary plus the
language `modality` binary to create a contract, create Alice and Bob passfiles,
write their `.id` files into contract state, synthesize and verify the
governing witness model with a review bundle, commit the accepted artifacts with
Alice's signature, and inspect status plus log output. The status check asserts
that both JSON and text output report the replayed governing model state as
`q1`, while the log check asserts that the signed onboarding commit exposes
Alice's signer ID and the commit message in both JSON and text output. Together
these cover the visible authority evidence and human-readable contract context
a new user sees when they run `modal c status` and `modal c log`. It then
commits a signed post-bootstrap state update, asserts that status/log now expose
the third committed entry while the governing model state remains `q1`, and
attempts an unsigned one, asserting that the governing model rejects the
unsigned path with the closest candidate transition and missing `signed_by`
predicate diagnostics.

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

To check whether the lean wrapper is ready for crates.io-style package
preparation:

```bash
tests/cli/check-modal-package-readiness.sh
```

This is a readiness probe, not an install command. It passes if Cargo can
prepare the package, and it also passes with a clear message for the current
known blocker: `modal` still depends on workspace CLI crates that are not
available from the registry.

To check whether the lean wrapper can be installed from a Git URL into a
temporary Cargo root:

```bash
tests/cli/check-modal-git-install-readiness.sh
```

The Git-install readiness check defaults to `file://$PWD`, which exercises
Cargo's Git resolver without requiring registry publication. Set
`MODAL_ONBOARDING_GIT_URL=https://github.com/modality-org/modality.git` to
check the public repository path, set `MODAL_ONBOARDING_GIT_REV=<commit>` to
pin the exact Git revision under test, and pass
`MODALITY_BIN=/path/to/modality` to run the installed `modal` binary through
the full first-contract smoke.

To check whether a built lean wrapper has the shape needed for a release
archive:

```bash
MODAL_BIN=/path/to/modal tests/cli/check-modal-release-archive-readiness.sh
```

The archive-readiness check creates a temporary
`modal-<version>-<os>-<arch>-<profile>.tar.gz`, verifies that it contains
`bin/modal`, `README.txt`, `PROVENANCE.txt`, `EVIDENCE-BUNDLE.txt`, and
`SHA256SUMS` in the emitted order, unpacks it, verifies the checksum manifest,
checks the provenance source revision, archive-safe platform tokens, profile,
feature set, and help surface, checks the evidence manifest declares the
replayable bundle, exact artifact, version, source revision, profile, feature
set, help surface, and post-unpack checks, then
checks the unpacked binary version and help surface, and runs the full
first-contract smoke when `MODALITY_BIN=/path/to/modality` is supplied. Set
`MODAL_ONBOARDING_ARCHIVE_DIR=/path/to/dir` to keep the generated tarball and
detached `.sha256` checksum. The kept directory also includes
`VERIFY-DOWNLOAD.txt`, which names the exact downloaded directory entries,
archive, expected source revision, expected profile, expected feature set,
expected help surface, detached checksum check, and exact download-verifier
command for artifact consumers in the canonical emitted order. Set
`MODAL_ONBOARDING_ARCHIVE_EXPECT_REV=<commit>` when release
evidence must fail if the built `modal` binary is stale or came from a
different source revision.

To verify a downloaded GitHub Actions artifact before unpacking or trusting it:

```bash
tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir
```

The download check expects exactly one `modal-*.tar.gz` and its matching
detached `.sha256` sidecar plus exactly one `VERIFY-DOWNLOAD.txt` recipe,
rejects any other top-level artifact entries, requires those entries to be
regular non-symlink files with canonical `0644` modes, requires the detached
checksum sidecar to name exactly that one archive with one canonical SHA-256
line, verifies the sidecar first, then unpacks the archive into a temporary
directory and rechecks the
exact member list in the emitted order, internal checksum manifest entries in
the emitted order, regular non-symlink unpacked
files, executable `bin/modal`, expected unpacked payload modes (`bin/modal` as
`0755`; text and checksum files as `0644`), provenance metadata, exactly one
README artifact marker plus version, revision, profile, feature-set, and
help-surface metadata matching provenance, replayable evidence
bundle marker, and the recipe's archive, checksum, exact expected directory
entries in the emitted order, revision, profile, feature set, help surface, and
exact verification-command section.
The producer smoke covers those top-level payload checks with negative cases for
directory archive, checksum sidecar, and recipe entries; symlinked archive,
checksum sidecar, and recipe entries; plus non-canonical archive, checksum
sidecar, and recipe modes.
The archive filename must also match the version, OS, architecture, and profile
recorded in provenance, so a consistently renamed tarball, sidecar, and recipe
fails before the binary is trusted. OS and architecture values must stay
archive-safe lowercase platform tokens. The
provenance file must keep exactly one marker line, name exactly one lowercase
hex source revision token, and exactly one value for the required metadata
fields. The profile must be one of the supported build
profiles (`debug` or `release`), and the feature set must be one of the
supported wrapper feature sets (`contract-onboarding` or `full`). Its help
surface must also be one of the supported surfaces (`lean` or `full`).
If the version string carries an embedded revision marker, that revision must
match the single source revision recorded by provenance. The replayable
evidence bundle marker must appear exactly once, and evidence manifest fields
for the artifact, version, source revision, profile, feature set, help surface,
binary, provenance file, checksum manifest, and post-unpack checks must also be
single-valued. The recipe title must appear
exactly once as the first line. The
recipe source-revision and help-surface
sections must each name exactly one value matching provenance, and its detached
checksum command, exact verifier command, smoke replay environment, and smoke
replay description must each appear exactly once, and the verification-command
section must contain no extra commands. The recipe sections must also stay in
the emitted order, so hand-shuffled recipes fail even when their values still
match provenance. The recipe smoke replay note must remain the exact final
two-line trailer, so stale inserted text cannot split the optional smoke
instructions while preserving both required lines. The whole recipe must also
match the canonical emitted text exactly after the targeted checks pass, so
hand-inserted prose between otherwise valid sections is rejected. The recipe also preserves the optional
`MODAL_ONBOARDING_ARTIFACT_SMOKE=1` and `MODALITY_BIN=/path/to/modality`
environment for replaying the version, help-surface, and first-contract smokes
against the unpacked binary. Set
`MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=<commit>` when the downloaded artifact
must fail unless its provenance names one exact source revision. Set
`MODAL_ONBOARDING_ARTIFACT_SMOKE=1` to also run the version and help surface
checks, and pass `MODALITY_BIN=/path/to/modality` with that flag to run the
first-contract smoke against the downloaded binary.

To test another binary:

```bash
MODAL_BIN=/path/to/modal MODALITY_BIN=/path/to/modality tests/cli/run-first-contract-cli-smoke.sh
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
