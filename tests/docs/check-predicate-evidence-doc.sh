#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/reference/standard-predicates.md"

required_patterns=(
  "## Current Local Evidence Matrix"
  "| \`+POST\`, \`+MODEL\`, and other method labels | Pending commit body methods | Checked on the pending commit |"
  "| \`signed_by(/path.id)\` | Pending commit signatures plus the public key string at \`/path.id\` in accepted state | Reads previously committed state, not values written by the same commit |"
  "| \`any_signed(/path)\` | Pending commit signatures plus every accepted-state \`*.id\` file under \`/path\` | At least one listed identity must sign |"
  "| \`all_signed(/path)\` | Pending commit signatures plus every accepted-state \`*.id\` file under \`/path\` | The directory must contain at least one identity, and every listed identity must sign |"
  "| \`threshold(\"n\", /path)\` | Pending commit signatures plus every accepted-state \`*.id\` file under \`/path\` | At least \`n\` unique listed identities must sign |"
  "| \`modifies(/path)\` | Pending commit body paths | Matches \`/path\` itself or descendants such as \`/path/alice.id\` |"
  "## Implementation Status"
  "| \`signed_by\`, \`any_signed\`, \`all_signed\`, \`threshold\`, \`modifies\` | Enforced | Derived from pending signatures, accepted state, and modified paths |"
  "| \`before\`, \`after\`, state predicates, hash predicates, \`oracle_attests\`, and \`wasm\` | Not first-contract-local yet | Intended extension vocabulary; treat as external or future predicate checks unless a validator path explicitly documents support |"
  "Verifies n-of-m signatures from the accepted identities under a path."
  "Counts each authorized public key at most once"
  "Ignores commit signatures from keys that are not listed under the path"
  "WASM predicates are intended custom predicate modules. They are not part of the"
  "Does not see identity files written by the same pending commit"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$DOC"; then
    echo "standard predicate reference is missing evidence text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$DOC"; then
  echo "standard predicate reference should avoid formula implication sugar" >&2
  exit 1
fi

echo "predicate evidence doc check passed"
