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
MODEL="$TMP_DIR/post-requires-reviewer-model.modality"
SYNTH_OUT="$TMP_DIR/synthesize.out"
VALIDATE_OUT="$TMP_DIR/validate.out"

cat >"$RULE" <<'EOF'
rule post_requires_reviewer {
  formula {
    always([+POST] true -> <+signed_by(/users/reviewer.id)> true)
  }
}
EOF

"$MODALITY_BIN" model synthesize \
  --rule "$RULE" \
  --verify \
  -o "$MODEL" >"$SYNTH_OUT" 2>&1

required_synthesis_patterns=(
  "Synthesizing from rule file:"
  "Verifying synthesized model against 1 formula(s)"
  'F1 `post_requires_reviewer` satisfied'
  "post_requires_reviewer"
)

for pattern in "${required_synthesis_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$SYNTH_OUT"; then
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
  if ! grep -Fq "$pattern" "$MODEL"; then
    echo "rule synthesized witness is missing: $pattern" >&2
    cat "$MODEL" >&2
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
  if ! grep -Fq "$pattern" "$VALIDATE_OUT"; then
    echo "rule synthesized witness validation output is missing: $pattern" >&2
    cat "$VALIDATE_OUT" >&2
    exit 1
  fi
done

echo "rule synthesize CLI smoke passed"
