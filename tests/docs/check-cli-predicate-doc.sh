#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/predicate-commands.md"

required_patterns=(
  "# Predicate Commands (\`modal predicate\`)"
  "modal predicate list [OPTIONS]"
  "\`--contract-id <CONTRACT_ID>\` | Contract ID to list predicates from"
  "amount_in_range"
  "has_property"
  "timestamp_valid"
  "post_to_path"
  "modal predicate info <NAME> [OPTIONS]"
  "\`--contract-id <CONTRACT_ID>\` | Contract ID to read predicate metadata from"
  "modal predicate test <NAME> [OPTIONS]"
  "\`--args <ARGS>\` | Arguments as a JSON string"
  "\`--block-height <BLOCK_HEIGHT>\`"
  "\`--timestamp <TIMESTAMP>\`"
  "modal predicate test signed_by --args"
  "modal predicate create [OPTIONS]"
  "\`--dir <DIR>\` | Directory to create the predicate project in"
  "\`--name <NAME>\` | Predicate name"
  "modal predicate create --dir ./predicates/kyc --name kyc_verified"
  "simulated predicate testing"
  "standard predicate reference"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "predicate command reference is missing current full-wrapper text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "\`--builtin\`" \
  "\`--custom\`" \
  "\`--data <JSON>\`" \
  "\`--file <FILE>\`" \
  "\`--verbose\`" \
  "\`--path <PATH>\` | Output directory" \
  "\`--lang <LANG>\`" \
  "modal predicate create kyc_verified --path" \
  "modal predicate test threshold --data" \
  "modal predicate test oracle_attests --file" \
  "threshold        n-of-m" \
  "oracle_attests" \
  "hash_matches"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "predicate command reference still contains stale full-wrapper text: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "predicate command doc check passed"
