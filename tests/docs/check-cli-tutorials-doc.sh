#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

docs=(
  "$ROOT_DIR/docs/tutorials/oracle-escrow.md"
  "$ROOT_DIR/docs/tutorials/multi-party-contract.md"
  "$ROOT_DIR/docs/tutorials/multisig-treasury.md"
)

required_patterns=(
  "modal c set-named-id /users/buyer.id buyer"
  "modal c set-named-id /users/seller.id seller"
  "modal c set-named-id /oracles/delivery.id delivery_oracle"
  "modal c set-named-id /users/alice.id alice"
  "modal c set-named-id /users/bob.id bob"
  "modal c set-named-id /treasury/alice.id alice"
  "modal c set-named-id /treasury/bob.id bob"
  "modal c set-named-id /treasury/carol.id carol"
  "modal c commit --all --sign buyer"
  "modal c commit --all --sign seller"
  "modal c commit --all --sign delivery_oracle"
  "modal c commit --all --sign alice"
  "modal c commit --all --sign bob"
  "modal c commit --all --sign carol"
)

for pattern in "${required_patterns[@]}"; do
  found=0
  for doc in "${docs[@]}"; do
    if grep -Fq -- "$pattern" "$doc"; then
      found=1
      break
    fi
  done

  if [[ "$found" -ne 1 ]]; then
    echo "CLI tutorial docs are missing current named-id setup text: $pattern" >&2
    exit 1
  fi
done

for doc in "${docs[@]}"; do
  if grep -Fq -- "set-named-id " "$doc" && grep -Fq -- "--named" "$doc"; then
    echo "CLI tutorial still contains stale set-named-id --named syntax: $doc" >&2
    exit 1
  fi

  if grep -Eq -- 'modal c (set-named-id|commit) .*~/.modality/' "$doc"; then
    echo "CLI tutorial still uses a home passfile path instead of an identity name: $doc" >&2
    exit 1
  fi
done

echo "CLI tutorial docs check passed"
