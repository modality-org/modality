#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODALITY_BIN="${MODALITY_BIN:-$ROOT_DIR/rust/target/debug/modality}"

if [[ ! -x "$MODALITY_BIN" ]]; then
  cat <<EOF
ACME review benchmark smoke skipped: modality binary not found at $MODALITY_BIN

Build it first:
  cd "$ROOT_DIR/rust"
  cargo build -p modality

Or pass an existing binary:
  MODALITY_BIN=/path/to/modality $0
EOF
  exit 0
fi

BENCH_DIR="$ROOT_DIR/experiments/ietf-autoformalization/rfc8555-acme/review-benchmark"
RULE="$BENCH_DIR/finalize-order-rule.modality"
SOURCE="$BENCH_DIR/finalize-order-source.txt"

if grep -Eq -- '->|\bimplies\b' "$RULE"; then
  echo "ACME review benchmark rule must use explicit Boolean form, not implication sugar" >&2
  cat "$RULE" >&2
  exit 1
fi

if ! grep -Fq -- 'always(!<+ACME_FINALIZE_ORDER> true | <+ACME_FINALIZE_ORDER +signed_by(/users/account_holder.id)> true)' "$RULE"; then
  echo "ACME review benchmark rule is missing the explicit finalize/signature Boolean guard" >&2
  cat "$RULE" >&2
  exit 1
fi

if ! grep -Fq -- 'always(!<+ACME_CREATE_ORDER> true | <+ACME_CREATE_ORDER +signed_by(/users/account_holder.id)> true)' "$RULE"; then
  echo "ACME review benchmark rule is missing the explicit newOrder/signature Boolean guard" >&2
  cat "$RULE" >&2
  exit 1
fi

if ! grep -Fq -- 'always(!<+ACME_VALIDATE_AUTHORIZATION> true | <+ACME_VALIDATE_AUTHORIZATION +signed_by(/users/certificate_authority.id)> true)' "$RULE"; then
  echo "ACME review benchmark rule is missing the explicit authorization-validation/CA Boolean guard" >&2
  cat "$RULE" >&2
  exit 1
fi

if ! grep -Fq -- 'always(!<+ACME_ISSUE_CERTIFICATE> true | <+ACME_ISSUE_CERTIFICATE +signed_by(/users/certificate_authority.id)> true)' "$RULE"; then
  echo "ACME review benchmark rule is missing the explicit certificate-issuance/CA Boolean guard" >&2
  cat "$RULE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

MODEL="$TMP_DIR/acme-finalize-model.modality"
REVIEW_BUNDLE="$TMP_DIR/acme-finalize-review.md"
SYNTH_OUT="$TMP_DIR/acme-finalize-synthesize.out"

"$MODALITY_BIN" model synthesize \
  --rule "$RULE" \
  --source-file "$SOURCE" \
  --verify \
  --review-bundle "$REVIEW_BUNDLE" \
  -o "$MODEL" >"$SYNTH_OUT" 2>&1

required_output_patterns=(
  "Synthesizing from rule file:"
  "Verifying synthesized model against 4 formula(s)"
  'F1.1 `acme_finalize_requires_account_holder` satisfied'
  'F1.2 `acme_new_order_requires_account_holder` satisfied'
  'F1.3 `acme_authorization_validation_requires_ca` satisfied'
  'F1.4 `acme_certificate_issuance_requires_ca` satisfied'
  "Synthesis review bundle written to"
)

for pattern in "${required_output_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$SYNTH_OUT"; then
    echo "ACME review benchmark output is missing: $pattern" >&2
    cat "$SYNTH_OUT" >&2
    exit 1
  fi
done

required_model_patterns=(
  "model Contract"
  "+ACME_FINALIZE_ORDER +signed_by(/users/account_holder.id)"
  "+ACME_CREATE_ORDER"
  "+signed_by(/users/account_holder.id)"
  "+ACME_VALIDATE_AUTHORIZATION"
  "+signed_by(/users/certificate_authority.id)"
  "+ACME_ISSUE_CERTIFICATE"
)

for pattern in "${required_model_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$MODEL"; then
    echo "ACME review benchmark witness is missing: $pattern" >&2
    cat "$MODEL" >&2
    exit 1
  fi
done

required_review_patterns=(
  "# Modality Synthesis Review Bundle"
  "## Original Source"
  "RFC 8555 section 7.4"
  "submitting a CSR to the order finalize URL"
  "RFC 8555 section 7.1.4"
  "server's newOrder resource"
  "RFC 8555 section 7.1.5"
  "marks authorization valid after successful challenge validation"
  "RFC 8555 section 8"
  "exposes it at the certificate URL"
  "External assumption: ACME account-key authentication"
  "CSR cryptographic soundness"
  "CA policy"
  "WebPKI trust"
  "DNS or HTTP domain-control validation"
  "## Rule File"
  "acme_finalize_requires_account_holder"
  "acme_new_order_requires_account_holder"
  "acme_authorization_validation_requires_ca"
  "acme_certificate_issuance_requires_ca"
  "## Extracted Facts"
  '`+ACME_FINALIZE_ORDER`'
  '`+ACME_CREATE_ORDER`'
  '`+ACME_VALIDATE_AUTHORIZATION`'
  '`+ACME_ISSUE_CERTIFICATE`'
  '`+signed_by(/users/account_holder.id)`'
  '`+signed_by(/users/certificate_authority.id)`'
  "## Source Clause Trace"
  "F1 source clause: RFC 8555 section 7.4"
  "F2 source clause: RFC 8555 section 7.1.4"
  "F3 source clause: RFC 8555 section 7.1.5"
  "F4 source clause: RFC 8555 section 8"
  "## Verifier Result"
  "Status: passed (\`--verify\`)"
  "## Witness Model"
  "## Assumptions"
  "## Known Gaps"
  "does not prove the extracted formulas capture the original intent"
)

for pattern in "${required_review_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$REVIEW_BUNDLE"; then
    echo "ACME review benchmark bundle is missing: $pattern" >&2
    cat "$REVIEW_BUNDLE" >&2
    exit 1
  fi
done

echo "ACME review benchmark smoke passed"
