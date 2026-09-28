#!/bin/sh
# Build the proven Lean checker, then compare it with the Rust theory on
# random label sets. Pass PT_ROUNDS / PT_SEED through the environment.
set -e
cd "$(dirname "$0")"
lake build pt-check
PT_CHECK="$PWD/.lake/build/bin/pt-check" cargo test \
  --manifest-path ../../../rust/Cargo.toml -p modality-lang --lib \
  rust_and_lean_agree -- --ignored --nocapture
