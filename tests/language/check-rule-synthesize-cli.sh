#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODALITY_BIN="${MODALITY_BIN:-$ROOT_DIR/rust/target/debug/modality}"

if [[ ! -x "$MODALITY_BIN" ]]; then
  cat <<EOF
rule synthesize CLI smoke skipped: modality binary not found at $MODALITY_BIN

Build it first:
  cd "$ROOT_DIR/rust"
  cargo build -p modality

Or pass an existing binary:
  MODALITY_BIN=/path/to/modality $0
EOF
  exit 0
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

RULE="$TMP_DIR/post-requires-reviewer.modality"
SOURCE="$TMP_DIR/post-requires-reviewer-source.txt"
MODEL="$TMP_DIR/post-requires-reviewer-model.modality"
REVIEW_BUNDLE="$TMP_DIR/post-requires-reviewer-review.md"
SYNTH_OUT="$TMP_DIR/synthesize.out"
VALIDATE_OUT="$TMP_DIR/validate.out"
UNSAT_RULE="$TMP_DIR/unsatisfied-rule.modality"
UNSAT_REVIEW_BUNDLE="$TMP_DIR/unsatisfied-rule-review.md"
UNSAT_OUT="$TMP_DIR/unsatisfied-rule.out"

cat >"$RULE" <<'EOF'
rule post_requires_reviewer {
  formula {
    always([+POST] true -> <+signed_by(/users/reviewer.id)> true)
  }
}
EOF

cat >"$SOURCE" <<'EOF'
F1: Every accepted post move must have reviewer signature evidence attached.
External assumption: signature verification and path identity evidence come from commit data.
EOF

"$MODALITY_BIN" model synthesize \
  --rule "$RULE" \
  --source-file "$SOURCE" \
  --verify \
  --review-bundle "$REVIEW_BUNDLE" \
  -o "$MODEL" >"$SYNTH_OUT" 2>&1

required_synthesis_patterns=(
  "Synthesizing from rule file:"
  "Verifying synthesized model against 1 formula(s)"
  'F1 `post_requires_reviewer` satisfied'
  "post_requires_reviewer"
  "Synthesis review bundle written to"
)

for pattern in "${required_synthesis_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$SYNTH_OUT"; then
    echo "rule synthesis output is missing: $pattern" >&2
    cat "$SYNTH_OUT" >&2
    exit 1
  fi
done

required_model_patterns=(
  "model Contract"
  "q0 --> q1: +POST +signed_by(/users/reviewer.id)"
)

for pattern in "${required_model_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$MODEL"; then
    echo "rule synthesized witness is missing: $pattern" >&2
    cat "$MODEL" >&2
    exit 1
  fi
done

required_review_patterns=(
  "# Modality Synthesis Review Bundle"
  "## Original Source"
  "- Input: \`--source-file $SOURCE\`"
  "Every accepted post move must have reviewer signature evidence attached."
  "## Rule File"
  "post_requires_reviewer"
  "## Extracted Facts"
  "Action labels:"
  '`+POST`'
  "Predicate calls:"
  '`+signed_by(/users/reviewer.id)`'
  "## Source Clause Trace"
  "F1 source clause: Every accepted post move must have reviewer signature evidence attached."
  "## Review Checklist"
  "Original source captured: yes"
  "Source-clause trace present: yes"
  "Parser-backed formulas: 1"
  "Verifier result: passed"
  "## Verifier Result"
  "Status: passed (\`--verify\`)"
  "## Witness Model"
  "model Contract"
  "## Assumptions"
  "## Known Gaps"
)

for pattern in "${required_review_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$REVIEW_BUNDLE"; then
    echo "rule synthesis review bundle is missing: $pattern" >&2
    cat "$REVIEW_BUNDLE" >&2
    exit 1
  fi
done

"$MODALITY_BIN" model validate "$MODEL" --verbose >"$VALIDATE_OUT" 2>&1

required_validation_patterns=(
  "Contract is valid!"
  "All properties are predicates or commit method labels (verifier-observed)."
  "Transitions:"
)

for pattern in "${required_validation_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$VALIDATE_OUT"; then
    echo "rule synthesized witness validation output is missing: $pattern" >&2
    cat "$VALIDATE_OUT" >&2
    exit 1
  fi
done

cat >"$UNSAT_RULE" <<'EOF'
rule impossible_contract {
  formula {
    false
  }
}
EOF

if "$MODALITY_BIN" model synthesize \
  --rule "$UNSAT_RULE" \
  --verify \
  --review-bundle "$UNSAT_REVIEW_BUNDLE" >"$UNSAT_OUT" 2>&1; then
  echo "unsatisfied rule synthesis unexpectedly succeeded" >&2
  cat "$UNSAT_OUT" >&2
  exit 1
fi

required_unsat_patterns=(
  "Synthesizing from rule file:"
  "Verifying synthesized model against 1 formula(s)"
  "Synthesis failure review bundle written to"
  "No satisfying witness found by current synthesis heuristics"
  "verifier rejected the synthesized candidate"
)

for pattern in "${required_unsat_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$UNSAT_OUT"; then
    echo "unsatisfied rule synthesis output is missing: $pattern" >&2
    cat "$UNSAT_OUT" >&2
    exit 1
  fi
done

required_unsat_review_patterns=(
  "# Modality Synthesis Review Bundle"
  "## Rule File"
  "impossible_contract"
  "## Extracted Facts"
  "## Review Checklist"
  "Original source captured: no"
  "Source-clause trace present: no"
  "Parser-backed formulas: 1"
  "Verifier result: failed"
  "## Verifier Result"
  "Status: failed (\`--verify\`)"
  "no satisfying witness was found by the current synthesis heuristics"
  "## Candidate Witness Model"
  "model Contract"
  "## Assumptions"
  "## Known Gaps"
  "bounded heuristic search path"
)

for pattern in "${required_unsat_review_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$UNSAT_REVIEW_BUNDLE"; then
    echo "unsatisfied rule synthesis review bundle is missing: $pattern" >&2
    cat "$UNSAT_REVIEW_BUNDLE" >&2
    exit 1
  fi
done

echo "rule synthesize CLI smoke passed"
