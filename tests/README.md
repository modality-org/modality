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
commitment-versus-enabledness trap warning, runs the `modality model lint` smoke
when a `modality` binary is present, and checks that both the default contract
CLI dependency tree and the lean `modal` onboarding wrapper avoid
onboarding-heavy network/storage/compression deps. It also runs the
first-contract CLI wrapper smoke when `rust/target/debug/modal` exists, or when
`MODAL_BIN=/path/to/modal` points at another built `modal` binary. When a
binary is present, it also checks that the real `modal --version` output
identifies the wrapper and that the `modal --help` surface matches the selected
onboarding shape: lean builds must expose the first-contract commands and omit
full runtime groups, while `MODAL_ONBOARDING_FEATURES=full` expects those
runtime groups to be present. The same real binary also runs the contract
evolution smoke, which verifies an additive rule commit, rejected bad witness
replacement, accepted V2 witness replacement, and Bob-signed successor update.

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
first-contract language smoke, avoiding `cargo run` in low-disk no-build runs:

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
