#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
MODEL="$SCRIPT_DIR/first-contract.modality"
MODALITY_BIN="${MODALITY_BIN:-}"

if [[ -n "$MODALITY_BIN" ]]; then
  if [[ ! -x "$MODALITY_BIN" ]]; then
    echo "MODALITY_BIN is not executable: $MODALITY_BIN" >&2
    exit 1
  fi
  output="$("$MODALITY_BIN" model validate "$MODEL" --verbose)"
else
  output="$(cd "$ROOT_DIR/rust" && cargo run -q -p modality -- model validate "$MODEL" --verbose)"
fi

printf '%s\n' "$output"

if ! grep -q "Transitions: 4" <<<"$output"; then
  echo "expected first-contract fixture to validate with 4 transitions" >&2
  exit 1
fi

if ! grep -q "Parts: 1" <<<"$output"; then
  echo "expected first-contract fixture to validate with 1 part" >&2
  exit 1
fi

if ! grep -q "Contract is valid!" <<<"$output"; then
  echo "expected first-contract fixture to pass validation" >&2
  exit 1
fi

if ! grep -q "All properties are predicates (verifiable)." <<<"$output"; then
  echo "expected first-contract fixture to pass predicate-only validation" >&2
  exit 1
fi
