# Modality Examples

## Onboarding Smokes

Run the current first-contract onboarding checks from the repository root:

```bash
tests/run-onboarding-smokes.sh
```

This always runs the parser-backed language smoke and checks that both the
default contract CLI dependency tree and the lean `modal` onboarding wrapper
avoid onboarding-heavy network/storage/compression deps. It also runs the
first-contract CLI wrapper smoke when `rust/target/debug/modal` exists, or when
`MODAL_BIN=/path/to/modal` points at another built `modal` binary.

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
network/hub wrapper.

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
iteration stays fast. Add `MODAL_ONBOARDING_PROFILE=release` when measuring the
release-profile installed wrapper.

```bash
MODAL_ONBOARDING_INSTALL=1 MODAL_ONBOARDING_PROFILE=release tests/run-onboarding-smokes.sh
```

After `cargo build` within `/rust`, you can use this directory to locally try out the `modality` command.

Alternatively, you can also use `modality-js` for the javascript implementation of the cli.

Be sure to approve direnv to add the debug build to your PATH within this directory.

Files in the playground subdirectory are ignored. The other directories are scripts showing exemplary usage.
