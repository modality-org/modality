#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/identity-commands.md"

required_patterns=(
  "# Identity Commands (\`modal id\`)"
  "modal id create --path alice.passfile"
  "modal id create --path alice.passfile --encrypt"
  "modal id create --name alice"
  "\`--path <PATH>\`"
  "\`--dir <DIR>\`"
  "\`--name <NAME>\`"
  "modal id derive --mnemonic"
  "BIP39 mnemonic seed phrase"
  "modal id get --path alice.passfile"
  "modal passfile encrypt --path alice.passfile"
  "modal passfile decrypt --path alice.passfile"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "identity command reference is missing current help-surface text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "modal id create --path alice.passfile --password" \
  "modal id derive --path alice.passfile --sub" \
  "--output alice-escrow.passfile" \
  "modal passfile encrypt --path alice.passfile --password" \
  "modal passfile decrypt --path alice.passfile.enc --password"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "identity command reference still contains stale help-surface text: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "identity command doc check passed"
