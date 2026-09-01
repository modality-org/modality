#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/reference/contract-evolution.md"

required_patterns=(
  "Modality contracts evolve by appending commits. They do not edit old terms in"
  "Rules are the authority. Models are witnesses"
  "\`MODEL\` commits replace the witness model only when the old accepted model"
  "The initial setup commit is accepted because it can take \`q0 -> q1\`"
  "an unsigned \`POST\` is rejected"
  "q1 -> q1 [+RULE +signed_by(/parties/alice.id)]"
  "[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)"
  "allows this separate rule commit"
  "A later \`MODEL\` commit is accepted only if both checks pass:"
  "The old accepted model has a matching \`+MODEL\` transition"
  "The candidate model satisfies the accumulated rule and can replay the"
  "This replacement is rejected because it exposes an unsigned steady-state"
  "replacement is not mutation"
  "Party changes should be ordinary state changes guarded by the current model"
  "active -> active [+POST +any_signed(/members) -modifies(/members)]"
  "active -> active [+POST +modifies(/members) +all_signed(/members)]"
  "active -> active [+MODEL +all_signed(/members)]"
  "Alice alone can"
  "add Bob while she is the only accepted member"
  "Bob can then append an ordinary"
  "Alice alone cannot replace the witness model after Bob is accepted"
  "Alice alone cannot add \`/members/carol.id\`"
  "Both rejected"
  "missing +all_signed(/members)"
  "Until that path is runnable in the contract CLI"
  "rules keep accumulating"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$DOC"; then
    echo "contract evolution reference is missing text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$DOC"; then
  echo "contract evolution reference should avoid formula implication sugar" >&2
  exit 1
fi

echo "contract evolution doc check passed"
