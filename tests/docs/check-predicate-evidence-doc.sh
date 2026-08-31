#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/reference/standard-predicates.md"
LANG_DOC="$ROOT_DIR/docs/language/predicates.md"

required_patterns=(
  "## Current Local Evidence Matrix"
  "| \`+POST\`, \`+MODEL\`, and other method labels | Pending commit body methods | Checked on the pending commit |"
  "| \`signed_by(/path.id)\` | Pending commit signatures plus the public key string at \`/path.id\` in accepted state | Reads previously committed state, not values written by the same commit |"
  "| \`any_signed(/path)\` | Pending commit signatures plus every accepted-state \`*.id\` file at \`/path\` or descendants | At least one listed identity must sign |"
  "| \`all_signed(/path)\` | Pending commit signatures plus every accepted-state \`*.id\` file at \`/path\` or descendants | The path must contain at least one identity, and every listed identity must sign |"
  "| \`threshold(\"n\", /path)\` | Pending commit signatures plus every accepted-state \`*.id\` file at \`/path\` or descendants | At least \`n\` unique listed identities must sign |"
  "| \`modifies(/path)\` | Pending commit body paths | Matches \`/path\` itself or descendants such as \`/path/alice.id\` |"
  "| \`post_to_path(/path)\` | Pending commit body methods and paths | Matches a \`POST\` action to \`/path\` itself or a descendant |"
  "| \`has_property(/path, \"a.b\")\` | Accepted-state JSON at \`/path\` | Reads previously committed JSON and follows dot-separated object keys |"
  "## Implementation Status"
  "| \`signed_by\`, \`any_signed\`, \`all_signed\`, \`threshold\`, \`modifies\`, \`post_to_path\`, \`has_property\` | Enforced | Derived from pending signatures, accepted state, pending methods, pending paths, and accepted-state JSON |"
  "| \`timestamp_valid\` | Unit-tested extension module only | Implemented in \`modal-wasm-validation\`; not yet replay evidence for the local first-contract validator |"
  "| \`before\`, \`after\`, state predicates, hash predicates, \`oracle_attests\`, and \`wasm\` | Not first-contract-local yet | Intended extension vocabulary; treat as external or future predicate checks unless a validator path explicitly documents support |"
  "### post_to_path"
  "Ignores non-\`POST\` actions, even when they write under the same path"
  "Returns true if any \`POST\` action targets the path itself or a descendant"
  "Does not match sibling paths that merely share a string prefix"
  "The \`timestamp_valid\` extension module compares an input timestamp with the"
  "replay must define where the trusted clock value"
  "The \`modal-wasm-validation\` crate also has unit-tested state-inspection modules"
  "The local validator now derives \`has_property(/path, \"a.b\")\`"
  "same pending commit"
  "Treat other state predicate inputs as explicit JSON"
  "The local validator now derives \`post_to_path(/path)\`"
  "\`has_property(/path, \"a.b\")\` from"
  "WASM-style predicate-test inputs remain"
  "Verifies n-of-m signatures from the accepted identities under a path."
  "Does not count identities from sibling paths that merely share a string prefix"
  "Counts each authorized public key at most once"
  "Ignores commit signatures from keys that are not listed under the path"
  "Rejection output reports the authorized signature count, accepted member count,"
  "WASM predicates are intended custom predicate modules. They are not part of the"
  "Does not see identity files written by the same pending commit"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$DOC"; then
    echo "standard predicate reference is missing evidence text: $pattern" >&2
    exit 1
  fi
done

WASM_README="$ROOT_DIR/rust/modal-wasm-validation/src/predicates/README.md"
wasm_patterns=(
  "These modules are locally unit-tested predicate evaluators."
  "not, by"
  "themselves, evidence that the local first-contract validator can derive each"
  "first-contract evidence boundary, use \`docs/reference/standard-predicates.md\`."
  "The checked object is explicit predicate-test input"
  "\`has_property(/path, \"a.b\")\` directly from accepted-state JSON"
  "previously committed state"
  "The \"current time\" is \`context.timestamp\`"
  "document the trusted clock source"
  "The \`modal-cli-contract\` local model-governance path also derives"
  "\`post_to_path(/path)\` directly from the pending commit body"
  "matching \`POST\`"
)

for pattern in "${wasm_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$WASM_README"; then
    echo "wasm predicate README is missing evidence-boundary text: $pattern" >&2
    exit 1
  fi
done

language_patterns=(
  "This page names the language vocabulary."
  "The currently verified local"
  "first-contract path is narrower: method labels, \`signed_by\`, \`any_signed\`,"
  "\`all_signed\`, \`threshold\`, \`modifies\`, \`post_to_path\`, and \`has_property\` are"
  "[standard predicate evidence matrix](../reference/standard-predicates.md)"
  "Do not treat the future vocabulary below as runtime evidence until a validator"
  "Oracle, time, comparison, hash,"
  "and most WASM predicates are extension vocabulary in the local first-contract"
)

for pattern in "${language_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LANG_DOC"; then
    echo "language predicate reference is missing evidence-boundary text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$DOC"; then
  echo "standard predicate reference should avoid formula implication sugar" >&2
  exit 1
fi

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LANG_DOC"; then
  echo "language predicate reference should avoid formula implication sugar" >&2
  exit 1
fi

echo "predicate evidence doc check passed"
