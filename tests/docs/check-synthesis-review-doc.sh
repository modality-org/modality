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
  "Does the Source Facts section preserve any reviewer-supplied \`Source fact:\`"
  "concrete \`+sets(...)\` path-write expectations"
  "source line numbers for traceability"
  "Does the Review Checklist say source capture, clause trace, parser-backed"
  "Does the Source Assumptions section preserve any reviewer-supplied"
  "\`External assumption:\` lines as out-of-proof evidence boundaries, with source"
  "Prompt-to-facts trace: not automatic"
  "Did \`--verify\` accept the witness model?"
  "always(!+POST | <+signed_by(/users/reviewer.id)> true)"
  "Review Checklist with \`Verifier result:"
  "Source Clause Trace"
  "This trace is preserved for review"
  "Source Facts"
  "Source fact: +sets(/posts/{post_id}/body)"
  "preserved with source line numbers for review"
  "does not infer or prove them"
  "Source Assumptions"
  "signature verification and path identity evidence come from commit data."
  "synthesis preserves them, but does not prove them."
  "## No-Witness Bundle"
  "no satisfying witness was found by bounded μ-calculus search"
  "A Review Checklist with \`Verifier result: failed\`."
  "candidate witness model that failed verification"
  "Any \`External assumption:\` lines supplied with the original source"
  "Any \`Source fact:\` lines supplied with the original source"
  "Assumptions and known gaps"
  "bounded explicit-state μ-calculus search"
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
