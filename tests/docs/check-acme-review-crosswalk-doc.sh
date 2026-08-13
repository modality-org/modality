#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CROSSWALK="$ROOT_DIR/experiments/ietf-autoformalization/rfc8555-acme/review-benchmark/path-write-crosswalk.md"

required_patterns=(
  "ACME Finalize Review Benchmark Crosswalk"
  'two RFC 8555 source clauses'
  '`+ACME_FINALIZE_ORDER`'
  '`+ACME_CREATE_ORDER`'
  '`+sets(/order/status.text, "processing")`'
  '`+sets(/order/status.text, "pending")`'
  '`+signed_by(/users/account_holder.id)`'
  '`only_holder_finalizes`'
  '`only_holder_creates_order`'
  '`finalize_requires_ready`'
  '`finalize_requires_authorization`'
  '`finalize_requires_order`'
  'RFC section 7.1.4 source clause'
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
