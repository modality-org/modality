#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODAL_BIN="${MODAL_BIN:-$ROOT_DIR/rust/target/debug/modal}"

"$ROOT_DIR/tests/language/run-onboarding-tests.sh"
"$ROOT_DIR/tests/cli/check-contract-cli-deps.sh"

if [[ -x "$MODAL_BIN" ]]; then
  MODAL_BIN="$MODAL_BIN" "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
else
  cat <<EOF
first-contract CLI smoke skipped: modal binary not found at $MODAL_BIN

Build it first:
  cd "$ROOT_DIR/rust"
  cargo build -p modal

Or pass an existing binary:
  MODAL_BIN=/path/to/modal $0
EOF
fi
