#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/language/rule-syntax.md"

required_patterns=(
  "### Commitment Versus Enabledness"
  "<+PAY> true"
  "[<+PAY>] true"
  "[+PAY] +signed_by(/parties/alice.id)"
  "Avoid \`[+PAY] true\` as a guard."
  "it does not prove \`PAY\`"
  "\`PAY\` is committed"
  "Run \`modality model lint <file>\`"
  "\`modality/vacuous-box-guard\`"
  "\`modality/implication-sugar\`"
  "rewrite them to explicit Boolean form"
  "Prefer explicit boolean form for conditional rules:"
  "!+modifies(/members) | +all_signed(/members)"
  "onboarding examples avoid it"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$DOC"; then
    echo "rule syntax reference is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- 'φ[[:space:]]*->[[:space:]]*ψ| implies ' "$DOC"; then
  echo "rule syntax reference should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

echo "language traps doc check passed"
