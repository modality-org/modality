# Modality Examples

## Onboarding Smokes

Run the current first-contract onboarding checks from the repository root:

```bash
tests/run-onboarding-smokes.sh
```

This always runs the parser-backed language smoke and checks that the default
contract CLI dependency tree avoids onboarding-heavy storage/compression deps.
It also runs the first-contract CLI wrapper smoke when `rust/target/debug/modal`
exists, or when `MODAL_BIN=/path/to/modal` points at another built `modal`
binary.

To measure the full source-built wrapper path from the same entry point, ask the
smoke to build the binary when it is missing:

```bash
MODAL_ONBOARDING_BUILD=1 tests/run-onboarding-smokes.sh
```

The smoke checks for at least 1 GiB of free disk before invoking Cargo so local
onboarding failures report the resource problem before incremental build output
fills the filesystem. Override the guard with `MODAL_ONBOARDING_MIN_KB` only for
known no-build diagnostic runs.

After `cargo build` within `/rust`, you can use this directory to locally try out the `modality` command.

Alternatively, you can also use `modality-js` for the javascript implementation of the cli.

Be sure to approve direnv to add the debug build to your PATH within this directory.

Files in the playground subdirectory are ignored. The other directories are scripts showing exemplary usage.
