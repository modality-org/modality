# Agent development rules

## Rust validation before a code commit is ready

- During implementation, keep feedback fast: run `cargo check -p <edited-crate>` and `cargo clippy -p <edited-crate> -- -D warnings -A clippy::too_many_arguments` for each edited Rust crate (or pass multiple `-p` flags). Include affected dependents when an API change could break them.
- Before calling a Rust code commit ready, run the full workspace gates from `rust/` and make both pass:
  - `RUSTFLAGS="-D warnings" cargo check --all` (the CI form of `cargo check --all`)
  - `RUSTFLAGS="-D warnings" cargo check --all --tests --examples` (test and example code must compile too)
  - `cargo clippy --all -- -D warnings -A clippy::too_many_arguments`
- Do not treat the scoped checks as a substitute for the full gates. Fix new diagnostics and rerun the failing gate before finishing.
- The argument-count lint is intentionally exempt; do not refactor functions solely to satisfy `clippy::too_many_arguments`.
