#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$SCRIPT_DIR/../../../rust"
MODEL="$SCRIPT_DIR/first-contract.modality"

output="$(cd "$RUST_DIR" && cargo run -q -p modality -- model validate "$MODEL" --verbose)"

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
