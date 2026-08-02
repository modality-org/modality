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
