#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/contract-commands.md"

required_patterns=(
  "# Contract Commands (\`modal contract\` / \`modal c\`)"
  "modal c create [OPTIONS]"
  "\`--dir <DIR>\` | Directory path where the contract will be created"
  "\`--output <FORMAT>\` | Output format: \`text\` or \`json\`"
  "modal c checkout [OPTIONS]"
  "\`--dir <DIR>\` | Contract directory (defaults to current directory)"
  "modal c set [OPTIONS] <PATH> <VALUE>"
  "modal c set-named-id [OPTIONS] <PATH> <NAME>"
  "passfile path or a passfile name"
  "modal c set-named-id /parties/alice.id alice.passfile"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "contract command reference is missing current help-surface text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "modal c set-named-id <PATH> --named <NAME>" \
  "modal c set-named-id /parties/alice.id --named alice" \
  "\`--name <NAME>\` | Contract name" \
  "\`--template <TEMPLATE>\` | Initialize from template" \
  "\`--file <FILE>\` | Read value from file" \
  "\`--type <TYPE>\` | Explicit path type" \
  "\`--commit <HASH>\` | Checkout specific commit" \
  "\`--force\` | Overwrite local changes"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "contract command reference still contains stale help-surface text: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "contract command doc check passed"
