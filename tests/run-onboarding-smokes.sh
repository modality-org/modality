#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODAL_BIN="${MODAL_BIN:-$ROOT_DIR/rust/target/debug/modal}"
MIN_FREE_KB="${MODAL_ONBOARDING_MIN_KB:-1048576}"

available_kb="$(df -Pk "$ROOT_DIR" | awk 'NR == 2 { print $4 }')"
if [[ "$available_kb" -lt "$MIN_FREE_KB" ]]; then
  cat >&2 <<EOF
onboarding smokes need at least ${MIN_FREE_KB} KiB free in $ROOT_DIR
available: ${available_kb} KiB

Free disk space, or lower the preflight only for a known no-build run:
  MODAL_ONBOARDING_MIN_KB=$available_kb $0
EOF
  exit 1
fi

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
