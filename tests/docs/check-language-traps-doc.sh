#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/language/rule-syntax.md"
GOTCHAS_DOC="$ROOT_DIR/docs/reference/gotchas.md"
MEMBERS_ONLY_TUTORIAL="$ROOT_DIR/docs/tutorials/members-only-contract.md"

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

gotchas_required_patterns=(
  "always(!<+ADD_MEMBER> true | <+ADD_MEMBER +all_signed(/members)> true)"
  "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)"
  "\`modality/implication-sugar\`"
  "always(!<+modifies(/x)> true | <+modifies(/x) +signed_by(/admin.id)> true)"
  "always(!<+modifies(/path)> true | <+modifies(/path) +all_signed(/members)> true)"
)

for pattern in "${gotchas_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$GOTCHAS_DOC"; then
    echo "gotchas reference is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$GOTCHAS_DOC"; then
  echo "gotchas reference should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

members_only_required_patterns=(
  "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)"
  "always(!<+modifies(/config)> true | <+modifies(/config) +signed_by(/admin.id)> true)"
)

for pattern in "${members_only_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MEMBERS_ONLY_TUTORIAL"; then
    echo "members-only tutorial is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$MEMBERS_ONLY_TUTORIAL"; then
  echo "members-only tutorial should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

echo "language traps doc check passed"
