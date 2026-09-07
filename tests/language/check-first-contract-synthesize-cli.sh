#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODALITY_BIN="${MODALITY_BIN:-$ROOT_DIR/rust/target/debug/modality}"

if [[ ! -x "$MODALITY_BIN" ]]; then
  cat <<EOF
first-contract synthesize CLI smoke skipped: modality binary not found at $MODALITY_BIN

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

RULE="$TMP_DIR/authorized.modality"
MODEL="$TMP_DIR/default.modality"
REVIEW_BUNDLE="$TMP_DIR/authorized-review.md"
LINT_OUT="$TMP_DIR/lint.out"
SYNTH_OUT="$TMP_DIR/synthesize.out"
VALIDATE_OUT="$TMP_DIR/validate.out"
MERMAID_OUT="$TMP_DIR/mermaid.out"
VIEW_OUT="$TMP_DIR/view.out"

cat >"$RULE" <<'EOF'
export default rule {
  starting_at $PARENT
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
EOF

"$MODALITY_BIN" model lint "$RULE" >"$LINT_OUT" 2>&1
grep -q "1 formula(s) lint-clean" "$LINT_OUT"

"$MODALITY_BIN" model synthesize \
  --rule "$RULE" \
  --verify \
  --review-bundle "$REVIEW_BUNDLE" \
  -o "$MODEL" >"$SYNTH_OUT" 2>&1

required_synthesis_patterns=(
  "Synthesizing from rule file:"
  "Verifying synthesized model against 1 formula(s)"
  'F1 `default_rule` satisfied'
  "default_rule"
  "Synthesis review bundle written to"
)

for pattern in "${required_synthesis_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$SYNTH_OUT"; then
    echo "first-contract synthesis output is missing: $pattern" >&2
    cat "$SYNTH_OUT" >&2
    exit 1
  fi
done

required_model_patterns=(
  "model Contract"
  "q0 --> q1"
  "q1 --> q1: +signed_by(/parties/alice.id)"
)

for pattern in "${required_model_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MODEL"; then
    echo "first-contract synthesized witness is missing: $pattern" >&2
    cat "$MODEL" >&2
    exit 1
  fi
done

if grep -Fq "+POST" "$MODEL"; then
  echo "first-contract synthesized witness still includes +POST" >&2
  cat "$MODEL" >&2
  exit 1
fi

if grep -Fq "+MODEL" "$MODEL"; then
  echo "first-contract synthesized witness still includes +MODEL" >&2
  cat "$MODEL" >&2
  exit 1
fi

if grep -Fq "/parties/bob.id" "$MODEL"; then
  echo "first-contract synthesized witness still includes Bob" >&2
  cat "$MODEL" >&2
  exit 1
fi

required_review_patterns=(
  "# Modality Synthesis Review Bundle"
  "## Rule File"
  "default_rule"
  "## Extracted Facts"
  '`-signed_by(/parties/alice.id)`'
  '`-signed_by(/parties/bob.id)`'
  "## Verifier Result"
  "Status: passed (\`--verify\`)"
  "## Witness Model"
  "q0 --> q1"
  "q1 --> q1: +signed_by(/parties/alice.id)"
  "## Assumptions"
  "## Known Gaps"
)

for pattern in "${required_review_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$REVIEW_BUNDLE"; then
    echo "first-contract synthesis review bundle is missing: $pattern" >&2
    cat "$REVIEW_BUNDLE" >&2
    exit 1
  fi
done

if grep -Fq "+POST" "$REVIEW_BUNDLE"; then
  echo "first-contract synthesis review bundle still includes +POST" >&2
  cat "$REVIEW_BUNDLE" >&2
  exit 1
fi

if grep -Fq "q1 --> q1: +signed_by(/parties/bob.id)" "$REVIEW_BUNDLE"; then
  echo "first-contract synthesis review bundle still includes Bob's live transition" >&2
  cat "$REVIEW_BUNDLE" >&2
  exit 1
fi

"$MODALITY_BIN" model validate "$MODEL" --verbose >"$VALIDATE_OUT" 2>&1

required_validation_patterns=(
  "Contract is valid!"
  "All properties are predicates or commit method labels (verifier-observed)."
  "Transitions: 2"
)

for pattern in "${required_validation_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$VALIDATE_OUT"; then
    echo "first-contract synthesized witness validation output is missing: $pattern" >&2
    cat "$VALIDATE_OUT" >&2
    exit 1
  fi
done

"$MODALITY_BIN" model mermaid "$MODEL" >"$MERMAID_OUT" 2>&1

required_mermaid_patterns=(
  "stateDiagram-v2"
  "q0 --> q1"
  '"+signed_by(/parties/alice.id)"'
)

for pattern in "${required_mermaid_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MERMAID_OUT"; then
    echo "first-contract synthesized witness mermaid output is missing: $pattern" >&2
    cat "$MERMAID_OUT" >&2
    exit 1
  fi
done

if grep -Fq "+POST" "$MERMAID_OUT"; then
  echo "first-contract synthesized witness mermaid still includes +POST" >&2
  cat "$MERMAID_OUT" >&2
  exit 1
fi

if grep -Fq "+MODEL" "$MERMAID_OUT"; then
  echo "first-contract synthesized witness mermaid still includes +MODEL" >&2
  cat "$MERMAID_OUT" >&2
  exit 1
fi

if grep -Fq "/parties/bob.id" "$MERMAID_OUT"; then
  echo "first-contract synthesized witness mermaid still includes Bob" >&2
  cat "$MERMAID_OUT" >&2
  exit 1
fi

"$MODALITY_BIN" model view "$MODEL" --no-open >"$VIEW_OUT" 2>&1

if ! grep -q "^Wrote " "$VIEW_OUT"; then
  echo "first-contract model view did not write a temp HTML file" >&2
  cat "$VIEW_OUT" >&2
  exit 1
fi
VIEW_HTML="$(sed -n 's/^Wrote //p' "$VIEW_OUT")"
required_view_patterns=(
  "mermaid.esm.min.mjs"
  "registerLayoutLoaders"
  "mermaid-layout-elk"
  "stateDiagram-v2"
  "part flow"
  "q0 --&gt; q1"
  "+signed_by(/parties/alice.id)"
)
for pattern in "${required_view_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$VIEW_HTML"; then
    echo "first-contract model view HTML is missing: $pattern" >&2
    cat "$VIEW_HTML" >&2
    exit 1
  fi
done
if grep -Fq "+POST" "$VIEW_HTML"; then
  echo "first-contract model view HTML still includes +POST" >&2
  cat "$VIEW_HTML" >&2
  exit 1
fi
if grep -Fq "/parties/bob.id" "$VIEW_HTML"; then
  echo "first-contract model view HTML still includes Bob" >&2
  cat "$VIEW_HTML" >&2
  exit 1
fi
rm -f "$VIEW_HTML"

echo "first-contract synthesize CLI smoke passed"
