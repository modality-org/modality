#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

docs=(
  "$ROOT_DIR/docs/tutorials/oracle-escrow.md"
  "$ROOT_DIR/docs/tutorials/multi-party-contract.md"
  "$ROOT_DIR/docs/tutorials/multisig-treasury.md"
)

required_patterns=(
  "modal c set-named-id /users/buyer.id ~/.modality/buyer.mod_passfile"
  "modal c set-named-id /users/seller.id ~/.modality/seller.mod_passfile"
  "modal c set-named-id /oracles/delivery.id ~/.modality/delivery_oracle.mod_passfile"
  "modal c set-named-id /users/alice.id ~/.modality/alice.mod_passfile"
  "modal c set-named-id /users/bob.id ~/.modality/bob.mod_passfile"
  "modal c set-named-id /treasury/alice.id ~/.modality/alice.mod_passfile"
  "modal c set-named-id /treasury/bob.id ~/.modality/bob.mod_passfile"
  "modal c set-named-id /treasury/carol.id ~/.modality/carol.mod_passfile"
  "modal c commit --all --sign ~/.modality/buyer.mod_passfile"
  "modal c commit --all --sign ~/.modality/seller.mod_passfile"
  "modal c commit --all --sign ~/.modality/delivery_oracle.mod_passfile"
  "modal c commit --all --sign ~/.modality/alice.mod_passfile"
  "modal c commit --all --sign ~/.modality/bob.mod_passfile"
  "modal c commit --all --sign ~/.modality/carol.mod_passfile"
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

  if grep -Eq -- 'modal c commit .*--sign (alice|bob|carol|buyer|seller|delivery_oracle)( |$)' "$doc"; then
    echo "CLI tutorial still signs with a bare identity name instead of a passfile path: $doc" >&2
    exit 1
  fi
done

echo "CLI tutorial docs check passed"
