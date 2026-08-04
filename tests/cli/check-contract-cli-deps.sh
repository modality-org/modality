#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

FORBIDDEN_DEPS_RE='^(libp2p|rocksdb|librocksdb-sys|zstd-sys)$'

matches="$(
  cd "$ROOT_DIR/rust"
  cargo tree -p modal-cli-contract --no-default-features --edges normal --prefix none \
    | sed 's/ .*//' \
    | sort -u \
    | grep -E "$FORBIDDEN_DEPS_RE" || true
)"

if [[ -n "$matches" ]]; then
  echo "modal-cli-contract default dependency tree includes onboarding-heavy deps:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "contract CLI default dependency guard passed"
