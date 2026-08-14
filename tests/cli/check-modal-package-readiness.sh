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

selected_direct_workspace_deps() {
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq unavailable; direct dependency list skipped"
    return 0
  fi

  (
    cd "$ROOT_DIR"
    cargo metadata \
      --format-version=1 \
      --no-default-features \
      --features "$FEATURES" \
      --manifest-path rust/modal/Cargo.toml
  ) | jq -r '
    .resolve.root as $root
    | .resolve.nodes[]
    | select(.id == $root)
    | .deps[]
    | select(.pkg | startswith("path+file:"))
    | .pkg
    | capture("/(?<name>[^/#]+)#(?<version>[^#]+)$")
    | "\(.name) \(.version)"
  ' | sort -u
}

selected_workspace_package_closure() {
  (
    cd "$ROOT_DIR/rust"
    cargo tree \
      -p modal \
      --no-default-features \
      --features "$FEATURES" \
      --edges normal \
      --prefix none
  ) | sed 's/ .*//' | sort -u | grep -E '^(modal|modality)' || true
}

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
  direct_deps="$(selected_direct_workspace_deps)"
  package_closure="$(selected_workspace_package_closure)"
  cat <<EOF
modal package readiness blocked: modal depends on workspace crates that are not available from the registry yet.

Selected direct workspace dependencies:
$direct_deps

Selected workspace package closure:
$package_closure

Run with CARGO_TARGET_DIR=/path/to/tmp to keep package artifacts off the repo target.
EOF
  exit 0
fi

cat "$OUTPUT_FILE" >&2
exit "$status"
