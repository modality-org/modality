#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FEATURES="${MODAL_ONBOARDING_FEATURES:-contract-onboarding}"
OUTPUT_FILE="$(mktemp)"

cleanup() {
  rm -f "$OUTPUT_FILE"
  if [[ -n "${TEMP_CARGO_TARGET_DIR:-}" ]]; then
    rm -rf "$TEMP_CARGO_TARGET_DIR"
  fi
}
trap cleanup EXIT

if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
  TEMP_CARGO_TARGET_DIR="$(mktemp -d)"
  export CARGO_TARGET_DIR="$TEMP_CARGO_TARGET_DIR"
fi

set +e
(
  cd "$ROOT_DIR/rust"
  cargo package \
    -p modal \
    --no-default-features \
    --features "$FEATURES" \
    --no-verify \
    --allow-dirty
) >"$OUTPUT_FILE" 2>&1
status=$?
set -e

if [[ "$status" -eq 0 ]]; then
  echo "modal package readiness check passed"
  exit 0
fi

if grep -Eq 'no matching package named `modal-cli-|no matching package named `modality|no matching package named `modal-common' "$OUTPUT_FILE"; then
  cat <<EOF
modal package readiness blocked: modal depends on workspace crates that are not available from the registry yet.

Run with CARGO_TARGET_DIR=/path/to/tmp to keep package artifacts off the repo target.
EOF
  exit 0
fi

cat "$OUTPUT_FILE" >&2
exit "$status"
