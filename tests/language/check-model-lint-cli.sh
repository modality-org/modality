#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODALITY_BIN="${MODALITY_BIN:-$ROOT_DIR/rust/target/debug/modality}"

if [[ ! -x "$MODALITY_BIN" ]]; then
  cat <<EOF
model lint CLI smoke skipped: modality binary not found at $MODALITY_BIN

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

BAD_FORMULA="$TMP_DIR/vacuous-box.modality"
GOOD_FORMULA="$TMP_DIR/enabled-pay.modality"
BAD_OUT="$TMP_DIR/vacuous-box.out"
GOOD_OUT="$TMP_DIR/enabled-pay.out"

cat >"$BAD_FORMULA" <<'EOF'
formula vacuous_pay_guard {
  always([+PAY] true)
}
EOF

cat >"$GOOD_FORMULA" <<'EOF'
formula enabled_pay_guard {
  always(<+PAY> true)
}
EOF

if "$MODALITY_BIN" model lint "$BAD_FORMULA" >"$BAD_OUT" 2>&1; then
  echo "model lint should reject vacuous [+PAY] true guards" >&2
  exit 1
fi

required_bad_patterns=(
  'modality/vacuous-box-guard'
  '`[+PAY] true` is vacuous'
  'use `[<+PAY>]` (committed) or `<+PAY>` (enabled here)'
  'Formula lint failed with 1 finding'
)

for pattern in "${required_bad_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$BAD_OUT"; then
    echo "model lint vacuous-guard output is missing: $pattern" >&2
    cat "$BAD_OUT" >&2
    exit 1
  fi
done

"$MODALITY_BIN" model lint "$GOOD_FORMULA" >"$GOOD_OUT" 2>&1

if ! grep -Fq "1 formula(s) lint-clean" "$GOOD_OUT"; then
  echo "model lint should accept the enabledness form" >&2
  cat "$GOOD_OUT" >&2
  exit 1
fi

echo "model lint CLI smoke passed"
