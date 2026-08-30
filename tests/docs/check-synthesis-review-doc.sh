#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/reference/synthesis-review.md"

required_patterns=(
  "# Synthesis Review Bundles"
  "modality model synthesize"
  "--verify"
  "--review-bundle"
  "--source-file"
  "Which action labels and predicate calls were extracted by the parser?"
  "Which reviewer-authored source clause, prompt, or protocol text was preserved?"
  "Does the Review Checklist say source capture, clause trace, parser-backed"
  "Did \`--verify\` accept the witness model?"
  "always(!+POST | <+signed_by(/users/reviewer.id)> true)"
  "Review Checklist with \`Verifier result:"
  "Source Clause Trace"
  "This trace is preserved for review"
  "## No-Witness Bundle"
  "no satisfying witness was found by the current synthesis heuristics"
  "A Review Checklist with \`Verifier result: failed\`."
  "candidate witness model that failed verification"
  "Assumptions and known gaps"
  "bounded heuristic search path"
  "It is a review artifact"
  "Confirm the rule source is the text the reviewer intended to check."
  "Read the verifier error before changing the rule"
  "unsupported synthesis pattern instead of an impossible contract"
  "rule impossible_contract"
  "state that \`--verify\` failed"
  "useful negative result"
  "does not automatically prove that natural-language intent"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "synthesis review doc is missing: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- '(^|[^[:alnum:]_])(->|implies)([^[:alnum:]_]|$)' "$DOC"; then
  echo "synthesis review doc should avoid implication sugar" >&2
  exit 1
fi

echo "synthesis review doc check passed"
