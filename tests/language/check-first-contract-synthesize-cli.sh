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
SYNTH_OUT="$TMP_DIR/synthesize.out"
VALIDATE_OUT="$TMP_DIR/validate.out"

cat >"$RULE" <<'EOF'
export default rule {
  starting_at $PARENT
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
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
  'F1 `default_rule` satisfied'
  "default_rule"
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
  "q0 --> q1: +POST +MODEL"
  "q1 --> q1: +POST +signed_by(/parties/alice.id)"
  "q1 --> q1: +POST +signed_by(/parties/bob.id)"
  "q1 --> q1: +MODEL +signed_by(/parties/alice.id)"
)

for pattern in "${required_model_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MODEL"; then
    echo "first-contract synthesized witness is missing: $pattern" >&2
    cat "$MODEL" >&2
    exit 1
  fi
done

"$MODALITY_BIN" model validate "$MODEL" --verbose >"$VALIDATE_OUT" 2>&1

required_validation_patterns=(
  "Contract is valid!"
  "All properties are predicates or commit method labels (verifier-observed)."
  "Transitions: 4"
)

for pattern in "${required_validation_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$VALIDATE_OUT"; then
    echo "first-contract synthesized witness validation output is missing: $pattern" >&2
    cat "$VALIDATE_OUT" >&2
    exit 1
  fi
done

echo "first-contract synthesize CLI smoke passed"
