#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/reference/verifier-rejections.md"

required_patterns=(
  "# Verifier Rejection Explanations"
  "explain that rejection from the same model"
  "current governing-model state"
  "closest candidate transition"
  "predicates that failed"
  "Similar transitions from other states"
  'current states {"q1"}'
  "Candidate transitions: none from current states"
  "non-current transition from q1 to q2 [+POST]; current states: q0; failed predicates: none"
  "still be unavailable"
  "current witness state"
  "missing +signed_by(/parties/alice.id)"
  "missing +signed_by(/parties/bob.id)"
  'missing +threshold("2", /treasury/signers)'
  "forbidden -modifies(/members) matched"
  "tests/cli/run-first-contract-cli-smoke.sh"
  "model-governance unit tests"
  "Model-Replacement Rule Failures"
  "failed anchor state"
  "satisfying states in the candidate model"
  "recursive formula counterexample"
  "Action-modal witnesses"
  "Fixed-point unfolding witnesses"
  "fixed-point witness set"
  "model-checking counterexample"
  "They do not prove that an"
  "external party should have signed"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "verifier rejection doc is missing: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- '(^|[^[:alnum:]_])(->|implies)([^[:alnum:]_]|$)' "$DOC"; then
  echo "verifier rejection doc should avoid implication sugar" >&2
  exit 1
fi

echo "verifier rejection doc check passed"
