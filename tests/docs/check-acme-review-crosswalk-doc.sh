#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CROSSWALK="$ROOT_DIR/experiments/ietf-autoformalization/rfc8555-acme/review-benchmark/path-write-crosswalk.md"

required_patterns=(
  "ACME Finalize Review Benchmark Crosswalk"
  'four RFC 8555 source clauses'
  '`+ACME_FINALIZE_ORDER`'
  '`+ACME_CREATE_ORDER`'
  '`+ACME_VALIDATE_AUTHORIZATION`'
  '`+ACME_ISSUE_CERTIFICATE`'
  '`+sets(/order/status.text, "processing")`'
  '`+sets(/order/status.text, "pending")`'
  '`+sets(/challenge/status.text, "valid")`'
  '`+sets(/order/status.text, "valid")`'
  '`+signed_by(/users/account_holder.id)`'
  '`+signed_by(/users/certificate_authority.id)`'
  '`only_holder_finalizes`'
  '`only_holder_creates_order`'
  '`only_ca_validates_authorization`'
  '`only_ca_issues_certificate`'
  '`finalize_requires_ready`'
  '`finalize_requires_authorization`'
  '`finalize_requires_order`'
  '`issuance_requires_finalize`'
  'RFC section 7.1.4 source clause'
  'RFC section 7.1.5 source clause'
  'RFC section 8 source clause'
  'universal per-edge action guards'
  'order and challenge status closed enums'
  'External assumption only'
  'not mechanically translated yet'
  'abstract source-clause review layer'
  'should not be treated as evidence'
  'full ACME path-write model was'
  'synthesized end to end'
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$CROSSWALK"; then
    echo "ACME review crosswalk doc is missing: $pattern" >&2
    exit 1
  fi
done

echo "ACME review crosswalk doc check passed"
