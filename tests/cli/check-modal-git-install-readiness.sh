#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPO_URL="${MODAL_ONBOARDING_GIT_URL:-file://$ROOT_DIR}"
REPO_REV="${MODAL_ONBOARDING_GIT_REV:-}"
FEATURES="${MODAL_ONBOARDING_FEATURES:-contract-onboarding}"
PROFILE="${MODAL_ONBOARDING_PROFILE:-debug}"
HELP_SURFACE="${MODAL_HELP_SURFACE:-lean}"

case "$PROFILE" in
  debug)
    CARGO_INSTALL_PROFILE_ARGS=(--debug)
    ;;
  release)
    CARGO_INSTALL_PROFILE_ARGS=()
    ;;
  *)
    echo "unsupported MODAL_ONBOARDING_PROFILE: $PROFILE" >&2
    echo "expected: debug or release" >&2
    exit 2
    ;;
esac

INSTALL_ROOT="$(mktemp -d)"
TEMP_CARGO_HOME="$(mktemp -d)"
TEMP_CARGO_TARGET_DIR="$(mktemp -d)"
REV_ARGS=()
cleanup() {
  rm -rf "$INSTALL_ROOT" "$TEMP_CARGO_HOME" "$TEMP_CARGO_TARGET_DIR"
}
trap cleanup EXIT

if [[ -n "$REPO_REV" ]]; then
  REV_ARGS=(--rev "$REPO_REV")
fi

CARGO_HOME="$TEMP_CARGO_HOME" CARGO_TARGET_DIR="$TEMP_CARGO_TARGET_DIR" \
  cargo install \
    --git "$REPO_URL" \
    "${REV_ARGS[@]}" \
    modal \
    --no-default-features \
    --features "$FEATURES" \
    --root "$INSTALL_ROOT" \
    --locked \
    "${CARGO_INSTALL_PROFILE_ARGS[@]}"

MODAL_BIN="$INSTALL_ROOT/bin/modal"
if [[ ! -x "$MODAL_BIN" ]]; then
  echo "git install did not produce executable modal at $MODAL_BIN" >&2
  exit 1
fi

MODAL_BIN="$MODAL_BIN" MODAL_HELP_SURFACE="$HELP_SURFACE" \
  "$ROOT_DIR/tests/cli/check-modal-help-surface.sh"

if [[ -x "${MODALITY_BIN:-}" ]]; then
  MODAL_BIN="$MODAL_BIN" MODALITY_BIN="$MODALITY_BIN" \
    "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
else
  cat <<EOF
first-contract git-install smoke skipped: MODALITY_BIN not supplied

Pass a built language CLI to verify the installed modal binary against the full
first-contract path:
  MODALITY_BIN=/path/to/modality $0
EOF
fi

if [[ -n "$REPO_REV" ]]; then
  echo "modal git install readiness check passed: $REPO_URL#$REPO_REV"
else
  echo "modal git install readiness check passed: $REPO_URL"
fi
