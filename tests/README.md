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

To measure the full source-built wrapper path from the same entry point, ask the
smoke to build the binary when it is missing:

```bash
MODAL_ONBOARDING_BUILD=1 tests/run-onboarding-smokes.sh
```

The smoke checks for at least 1 GiB of free disk before invoking Cargo so local
onboarding failures report the resource problem before incremental build output
fills the filesystem. Override the guard with `MODAL_ONBOARDING_MIN_KB` only for
known no-build diagnostic runs. Build mode uses `contract-onboarding` by
default; set `MODAL_ONBOARDING_FEATURES=full` only when measuring the full
network/hub wrapper. Set `MODAL_HELP_SURFACE=lean|full` only when checking an
explicit `MODAL_BIN` whose feature shape differs from
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
MODAL_ONBOARDING_BUILD=1 MODAL_ONBOARDING_PROFILE=release tests/run-onboarding-smokes.sh
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

After `cargo build` within `/rust`, you can use this directory to locally try out the `modality` command.

Alternatively, you can also use `modality-js` for the javascript implementation of the cli.

Be sure to approve direnv to add the debug build to your PATH within this directory.

Files in the playground subdirectory are ignored. The other directories are scripts showing exemplary usage.
