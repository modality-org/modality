# Modality Examples

## Onboarding Smokes

Run the current first-contract onboarding checks from the repository root:

```bash
tests/run-onboarding-smokes.sh
```

This always runs the parser-backed language smoke, checks that the standard
predicate reference preserves the local evidence-source matrix, checks that the
contract evolution reference preserves the accumulated-rule and witness
replacement model, checks that the rule syntax reference preserves the
commitment-versus-enabledness trap warning, checks that the first-contract guide
adds rules, synthesizes the witness model with `--verify`, validates it with
`modality model validate`, and then commits, checks that the installation guide
keeps the `modal` contract CLI and `modality` model CLI split explicit, checks
that the real `modality` model help surface matches the documented language CLI
when a language binary is present, checks that the verifier rejection reference
preserves current-state, closest-candidate, and missing-predicate diagnostics,
checks that the synthesis review reference explains passed bundles and no-witness
bundles, checks that the ACME review benchmark crosswalk keeps the abstract
`newOrder`, authorization-validation, and finalize fixture distinct from the
path-write corpus, runs the first-contract rule-to-witness synthesis smoke,
validates the synthesized witness, runs a signed-`POST` rule synthesis smoke,
validates that synthesized witness, runs the unsatisfied-rule no-witness
diagnostic bundle check, runs the `modality model lint` smoke when a `modality`
binary is present, and checks that
both the default contract CLI dependency tree and the lean `modal` onboarding
wrapper avoid onboarding-heavy network/storage/compression deps. It also runs the
first-contract CLI wrapper smoke when both `rust/target/debug/modal` and
`rust/target/debug/modality` exist, or when `MODAL_BIN=/path/to/modal` and
`MODALITY_BIN=/path/to/modality` point at built binaries. When a `modal`
binary is present, it also checks that the real `modal --version` output
identifies the wrapper and that the `modal --help` surface matches the selected
onboarding shape: lean builds must expose the first-contract commands and omit
full runtime groups, while `MODAL_ONBOARDING_FEATURES=full` expects those
runtime groups to be present. The first-contract CLI smoke synthesizes the
governing witness with `--verify`, validates that generated model, and only
then commits it through `modal`. The same real `modal` binary also runs the
contract evolution smoke, which verifies an additive rule commit, rejected bad
witness replacement, accepted V2 witness replacement, and Bob-signed successor
update.
The language CLI bundle also runs an ACME RFC 8555 review-benchmark smoke that
preserves source clauses, extracted action and signature facts, verifier
status, explicit external assumptions, and known gaps in a review bundle while
guarding that the benchmark rule uses explicit Boolean syntax instead of
implication sugar.
Set `MODAL_ONBOARDING_PACKAGE_CHECK=1` to also run the package-readiness probe
for the lean wrapper. That probe runs Cargo's package preparation and reports
the current external-package blocker when `modal` still depends on workspace
CLI crates that are not available from the registry.
Set `MODAL_ONBOARDING_GIT_INSTALL_CHECK=1` to also install the lean wrapper
from a Git URL into a temporary Cargo root, check the installed `modal` help
surface, and run the first-contract CLI smoke when `MODALITY_BIN` points at a
built language CLI. By default this uses `file://$PWD` so local and CI runs can
exercise the same Cargo Git-install resolver without publishing crates; set
`MODAL_ONBOARDING_GIT_URL=https://github.com/modality-org/modality.git` when
checking the public repository path, and set `MODAL_ONBOARDING_GIT_REV=<commit>`
when the install evidence must be pinned to an exact revision.
Set `MODAL_ONBOARDING_ARCHIVE_CHECK=1` to also create a
release-archive-shaped tarball from a built or installed lean wrapper, unpack
it, check the unpacked `modal` help surface, and run the first-contract CLI
smoke when `MODALITY_BIN` points at a built language CLI.

To measure the full source-built wrapper path from the same entry point, ask the
smoke to build the binary when it is missing:

```bash
MODAL_ONBOARDING_BUILD=1 tests/run-onboarding-smokes.sh
```

The first-contract path also depends on the `modality` language CLI for lint,
synthesis, review-bundle generation, and validation. Build both CLIs from the
same entry point when measuring a fresh first-contract run:

```bash
MODALITY_ONBOARDING_BUILD=1 MODAL_ONBOARDING_BUILD=1 tests/run-onboarding-smokes.sh
```

The smoke checks for at least 1 GiB of free disk before invoking Cargo so local
onboarding failures report the resource problem before incremental build output
fills the filesystem. Override the guard with `MODAL_ONBOARDING_MIN_KB` only for
known no-build diagnostic runs. Build mode uses `contract-onboarding` by
default; set `MODAL_ONBOARDING_FEATURES=full` only when measuring the full
network/hub wrapper. If `CARGO_TARGET_DIR` points at a temporary target, the
smoke looks for both default binaries under that same target directory. Set
`MODAL_HELP_SURFACE=lean|full` only when checking an explicit `MODAL_BIN` whose
feature shape differs from
`MODAL_ONBOARDING_FEATURES`.

To run the lint smoke against a cached language CLI without rebuilding, pass
`MODALITY_BIN`. The same binary is also used for the parser-backed
first-contract language smoke, the standalone first-contract rule-to-witness
synthesis smoke plus its review bundle, the signed-`POST` rule synthesis plus
review-bundle and no-witness diagnostic smoke, the ACME RFC 8555
review-benchmark smoke, and, when a `modal` binary is present, to synthesize the
first-contract witness model from the rule with `--verify`, write the review
bundle, and only then let the contract CLI smoke commit it. This avoids
`cargo run` in low-disk no-build runs:

```bash
MODALITY_BIN=/path/to/modality tests/run-onboarding-smokes.sh
```

To measure the lean release-profile wrapper, set `MODAL_ONBOARDING_PROFILE`:

```bash
MODALITY_ONBOARDING_BUILD=1 MODAL_ONBOARDING_BUILD=1 MODAL_ONBOARDING_PROFILE=release tests/run-onboarding-smokes.sh
```

The default profile is `debug`, which builds and smokes
`rust/target/debug/modal`. The `release` profile builds and smokes
`rust/target/release/modal` with the same lean onboarding features.

To measure the install shape rather than a direct target binary, install the
lean wrapper into a temporary Cargo root and smoke that installed `modal`:

```bash
MODAL_ONBOARDING_INSTALL=1 tests/run-onboarding-smokes.sh
```

With the default `debug` profile this uses `cargo install --debug` so local
iteration stays fast. The smoke asserts that `cargo install` produced an
executable `bin/modal` in the temporary Cargo root and runs the help-surface and
first-contract checks against that installed binary. Add
`MODAL_ONBOARDING_PROFILE=release` when measuring the release-profile installed
wrapper.

```bash
MODAL_ONBOARDING_INSTALL=1 MODAL_ONBOARDING_PROFILE=release tests/run-onboarding-smokes.sh
```

To measure crates.io-style package readiness for the lean wrapper without
claiming that external distribution is finished, run:

```bash
MODAL_ONBOARDING_PACKAGE_CHECK=1 tests/run-onboarding-smokes.sh
```

The package-readiness probe accepts either a successful Cargo package
preparation or the current known blocker: `modal` still references workspace
CLI crates that are not published to the registry. While blocked, it prints the
selected direct workspace dependencies and the selected workspace package
closure so the external package or installer work has a concrete crate list.

To measure Git-install readiness for the lean wrapper without waiting on
registry publication, run:

```bash
MODAL_ONBOARDING_GIT_INSTALL_CHECK=1 tests/run-onboarding-smokes.sh
```

The Git-install probe installs `modal` into a temporary Cargo root from
`MODAL_ONBOARDING_GIT_URL` or, by default, `file://$PWD`. Pair it with
`MODALITY_BIN=/path/to/modality` when you want the installed `modal` binary to
run the full first-contract contract smoke after the help-surface check. Set
`MODAL_ONBOARDING_GIT_REV=<commit>` to pin the installed source revision.

To measure release-archive readiness from a built lean wrapper without
publishing a registry package, run:

```bash
MODAL_ONBOARDING_BUILD=1 MODAL_ONBOARDING_ARCHIVE_CHECK=1 tests/run-onboarding-smokes.sh
```

The archive-readiness probe creates a `modal-<version>-<os>-<arch>-<profile>.tar.gz`
containing `bin/modal`, `README.txt`, `PROVENANCE.txt`, `EVIDENCE-BUNDLE.txt`,
and `SHA256SUMS`, unpacks it, verifies the checksum manifest, asserts that the
archive contains exactly those five entries in the emitted order, asserts that
the manifest covers exactly `bin/modal`, `README.txt`, `PROVENANCE.txt`, and
`EVIDENCE-BUNDLE.txt`, checks that provenance records the source revision,
profile, and help surface, checks that the evidence manifest names the
replayable bundle, artifact, source revision, and post-unpack checks, checks
that the unpacked binary reports the same version, checks the selected help
surface, and runs the first-contract CLI smoke when `MODALITY_BIN` points at a
built language CLI. Set `MODAL_ONBOARDING_ARCHIVE_DIR=/path/to/dir` when you
want to keep the generated tarball and detached `.sha256` checksum for release
inspection. The kept directory also includes `VERIFY-DOWNLOAD.txt`, which names
the exact downloaded directory entries, archive, expected source revision,
expected help surface, detached checksum check, and exact download-verifier
command section for the artifact consumer. Set
`MODAL_ONBOARDING_ARCHIVE_EXPECT_REV=<commit>` when release evidence must fail
if the built `modal` binary is stale or came from a different source revision.
The archive check also runs the downloaded-artifact verifier against the
generated archive directory, which simulates the GitHub Actions artifact
consumer path by requiring exactly one `modal-*.tar.gz`, its matching detached
`.sha256` sidecar, exactly one `VERIFY-DOWNLOAD.txt` recipe, no other
top-level artifact entries, regular non-symlink top-level files with canonical
`0644` modes, single-valued provenance whose embedded version revision matches
the source revision when present, the exact one-entry detached checksum sidecar
with a canonical SHA-256 line, internal archive members in the emitted order,
internal checksum manifest entries in the
emitted order, regular non-symlink unpacked files, executable `bin/modal`,
provenance metadata, source revision, the
expected unpacked payload modes (`bin/modal` as `0755`; text and checksum files
as `0644`), exactly one README artifact marker, README metadata matching
provenance, the single provenance marker, the single replayable evidence bundle
marker, exactly one provenance source revision, exactly one value for the
required provenance metadata fields,
lowercase hex source revision token,
archive-safe OS and architecture tokens,
one of the supported provenance profiles (`debug` or `release`), one of the
supported provenance feature sets (`contract-onboarding` or `full`), one of the
supported provenance help surfaces (`lean` or `full`),
and the recipe's archive, checksum, exact expected directory
entries, first-line single title, revision, help surface, and verifier command
section, rejecting any extra verification commands. It also requires the optional smoke
replay note to remain the exact final two-line trailer, so stale inserted text
cannot split the smoke instructions while preserving the required lines. Set
`MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=<commit>` when checking a downloaded
workflow artifact against one exact source revision. Set
`MODAL_ONBOARDING_ARTIFACT_SMOKE=1` to also check that the downloaded binary's
reported version matches provenance before replaying the selected help surface.

After `cargo build` within `/rust`, you can use this directory to locally try out the `modality` command.

Alternatively, you can also use `modality-js` for the javascript implementation of the cli.

Be sure to approve direnv to add the debug build to your PATH within this directory.

Files in the playground subdirectory are ignored. The other directories are scripts showing exemplary usage.
