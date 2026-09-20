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
  "source line numbers and shape labels"
  "\`path-write template\` for traceability"
  "Does it flag malformed signed \`Source fact:\` shapes"
  "review warning: malformed source fact"
  "Does the Review Checklist say source capture, clause trace, parser-backed"
  "Does the Review Checklist summarize whether source facts were preserved"
  "many source facts were preserved"
  "many malformed source facts were flagged"
  "how many external"
  "Does the Source Assumptions section preserve any reviewer-supplied"
  "\`External assumption:\` lines as out-of-proof evidence boundaries, with source"
  "line numbers and assumption-boundary labels"
  "Prompt-to-facts trace: not automatic"
  "Did \`--verify\` accept the witness model?"
  "always(!+POST | <+signed_by(/users/reviewer.id)> true)"
  "Review Checklist with \`Verifier result:"
  "Source facts preserved: yes"
  "Source facts preserved count: 2"
  "Malformed source facts flagged: 1"
  "External assumptions preserved: yes"
  "External assumptions preserved count: 1"
  "Source Clause Trace"
  "This trace is preserved for review"
  "Source Facts"
  "Source fact: +sets(/posts/{post_id}/body)"
  "preserved with source line numbers and source-fact shape"
  "Malformed signed source facts"
  "should not treat it as structured"
  "or prove them"
  "Source Assumptions"
  "External assumption: signature verification and path"
  "identity evidence come from commit data."
  "\`commit evidence boundary\`"
  "\`external-world boundary\`"
  "lines as explicit review boundaries: synthesis preserves them"
  "prove them. External-world dependencies"
  "## No-Witness Bundle"
  "no satisfying witness was found by bounded μ-calculus search"
  "A Review Checklist with \`Verifier result: failed\`, source-fact preservation"
  "source-fact count"
  "malformed-source-fact count"
  "external-assumption count"
  "assumption-boundary labels"
  "candidate witness model that failed verification"
  "Any \`External assumption:\` lines supplied with the original source"
  "Any \`Source fact:\` lines supplied with the original source"
  "source-fact shape labels, including malformed-source warnings"
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
