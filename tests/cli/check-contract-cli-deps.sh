#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODAL_ONBOARDING_FEATURES="${MODAL_ONBOARDING_FEATURES:-contract-onboarding}"

FORBIDDEN_DEPS_RE='^(libp2p|rocksdb|librocksdb-sys|zstd-sys)$'

contract_matches="$(
  cd "$ROOT_DIR/rust"
  cargo tree -p modal-cli-contract --no-default-features --edges normal --prefix none \
    | sed 's/ .*//' \
    | sort -u \
    | grep -E "$FORBIDDEN_DEPS_RE" || true
)"

if [[ -n "$contract_matches" ]]; then
  echo "modal-cli-contract default dependency tree includes onboarding-heavy deps:" >&2
  echo "$contract_matches" >&2
  exit 1
fi

modal_matches="$(
  cd "$ROOT_DIR/rust"
  cargo tree -p modal --no-default-features --features "$MODAL_ONBOARDING_FEATURES" --edges normal --prefix none \
    | sed 's/ .*//' \
    | sort -u \
    | grep -E "$FORBIDDEN_DEPS_RE" || true
)"

if [[ -n "$modal_matches" ]]; then
  echo "modal $MODAL_ONBOARDING_FEATURES dependency tree includes onboarding-heavy deps:" >&2
  echo "$modal_matches" >&2
  exit 1
fi

echo "contract CLI default dependency guard passed"
