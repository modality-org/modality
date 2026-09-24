#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/reference/standard-predicates.md"
LANG_DOC="$ROOT_DIR/docs/language/predicates.md"
MODEL_GOVERNANCE="$ROOT_DIR/rust/modality-common/src/model_governance.rs"

required_patterns=(
  "## Current Local Evidence Matrix"
  "| \`+POST\`, \`+REPOST\`, \`+MODEL\`, and other method labels | Pending commit body methods | Checked on the pending commit |"
  "| \`signed_by(/path.id)\` | Pending commit signatures plus the public key string at \`/path.id\` in accepted state | Reads previously committed state, not values written by the same commit |"
  "| \`any_signed(/path)\` | Pending commit signatures plus every accepted-state \`*.id\` file at \`/path\` or descendants | At least one listed identity must sign |"
  "| \`all_signed(/path)\` | Pending commit signatures plus every accepted-state \`*.id\` file at \`/path\` or descendants | The path must contain at least one identity, and every listed identity must sign |"
  "| \`threshold(\"n\", /path)\` | Pending commit signatures plus every accepted-state \`*.id\` file at \`/path\` or descendants | At least \`n\` unique listed identities must sign |"
  "| \`modifies(/path)\` | Pending commit body paths | Matches \`/path\` itself or descendants such as \`/path/alice.id\` |"
  "| \`post_to_path(/path)\` | Pending commit body methods and paths | Matches a \`POST\` action to \`/path\` itself or a descendant |"
  "| \`has_property(/path, \"a.b\")\` | Accepted-state JSON at \`/path\` | Reads previously committed JSON and follows dot-separated object keys |"
  "| \`state_exists(/path)\` | Accepted-state path map | Checks that a path was already committed before the pending commit |"
  "| \`text_eq(/path, \"value\")\` or \`text_eq(/left, /right)\` | Accepted-state text | Compares previously committed string values or a committed string to a literal |"
  "| \`text_contains(/path, \"needle\")\`, \`text_starts_with(/path, \"prefix\")\`, and \`text_ends_with(/path, \"suffix\")\` | Accepted-state text | Checks whether a previously committed string contains, starts with, or ends with a literal substring |"
  "| \`amount_in_range(/path, \"min\", \"max\")\` | Accepted-state number | Compares a previously committed number to inclusive quoted numeric or accepted-state numeric bounds |"
  "| \`num_eq\`, \`num_gt\`, \`num_gte\`, \`num_lt\`, \`num_lte\` | Accepted-state number | Compares a previously committed number to a literal or accepted-state numeric bound |"
  "| \`bool_true(/path)\` and \`bool_false(/path)\` | Accepted-state boolean | Checks a previously committed boolean value |"
  "## Implementation Status"
  "| \`signed_by\`, \`any_signed\`, \`all_signed\`, \`threshold\`, \`modifies\`, \`post_to_path\`, \`has_property\`, \`state_exists\`, \`text_eq\`, \`text_contains\`, \`text_starts_with\`, \`text_ends_with\`, \`amount_in_range\`, \`num_eq\`, \`num_gt\`, \`num_gte\`, \`num_lt\`, \`num_lte\`, \`bool_true\`, \`bool_false\` | Enforced | Derived from pending signatures, accepted state, pending methods, pending paths, accepted-state path existence, accepted-state JSON, accepted-state text, accepted-state numbers, and accepted-state booleans |"
  "| \`timestamp_valid\` | Unit-tested extension module only | Implemented in \`modality-wasm-validation\`; not yet replay evidence for the local first-contract validator |"
  "| \`before\`, \`after\`, other state-value predicates, hash predicates, \`oracle_attests\`, and \`wasm\` | Not first-contract-local yet | Intended extension vocabulary; treat as external or future predicate checks unless a validator path explicitly documents support |"
  "## Checkpoint Review Scope"
  "covers method labels, pending signatures, accepted-state identity paths,"
  "segment-aware pending write paths, accepted-state JSON properties,"
  "accepted-state path existence, accepted-state text comparison, contains, prefix,"
  "and suffix checks, accepted-state numeric ranges, and accepted-state numeric"
  "comparisons plus accepted-state boolean checks."
  "enough to review local log conformance"
  "without depending on clocks, oracles,"
  "hash preimages, or custom WASM execution."
  "Keep deadline, oracle, hash, and broader WASM predicates out of first-contract"
  "documents the replay artifact format, trust"
  "root, and negative tests for each evidence source."
  "**Replay-bound artifact boundary:**"
  "first external evidence format to graduate from"
  "vocabulary to verifier evidence"
  "canonical bytes"
  "inside the replay bundle"
  "contract id or genesis hash, pending commit hash, predicate name, oracle path,"
  "claim, value, issuance time, and freshness or expiry policy"
  "wrong-contract, stale or future timestamp,"
  "missing commit-binding, missing or mismatched oracle path, argument mismatch,"
  "wrong accepted-state oracle key, and malformed or non-canonical artifact cases"
  "reported as checked instead of missing external evidence"
  "unit-tested"
  "\`replay_bundle_json\` input boundary"
  "exact compact canonical JSON"
  "same attestation and positive \`max_age_seconds\` freshness policy as the"
  "predicate input"
  "accepted-state oracle"
  "key as \`expected_oracle_pubkey\`"
  "Missing replay-bundle freshness"
  "policy,"
  "bundle/input"
  "freshness mismatches"
  "missing accepted-state oracle keys"
  "accepted-state oracle key mismatches"
  "mismatches, malformed JSON"
  "malformed JSON, pretty-printed or"
  "wrong"
  "predicate names, and envelope/input"
  "attestation mismatches fail before"
  "signature"
  "verification"
  "This is still"
  "extension-level evidence only"
  "validator supplies the"
  "bundle and accepted-state oracle key from replay data"
  "accepted-state oracle key"
  "expected_oracle_pubkey"
  "### post_to_path"
  "Ignores non-\`POST\` actions, even when they write under the same path"
  "Returns true if any \`POST\` action targets the path itself or a descendant"
  "Does not match sibling paths that merely share a string prefix"
  "The \`timestamp_valid\` extension module compares an input timestamp with the"
  "replay must define where the trusted clock value"
  "The \`modality-wasm-validation\` crate also has unit-tested state-inspection modules"
  "The local validator now derives \`has_property(/path, \"a.b\")\`"
  "The local validator now also derives \`state_exists(/path)\`"
  "a value written by the same pending commit is not evidence for"
  "The local validator also derives \`text_eq\` from accepted contract state."
  "see text written by the same pending commit."
  "The \`has_property\`, \`state_exists\`, \`text_eq\`, \`text_contains\`,"
  "\`text_starts_with\`, \`text_ends_with\`, \`amount_in_range\`, numeric comparison,"
  "evidence today. They read only accepted state; they"
  "path existence, text, numbers, or booleans written by the same"
  "same pending commit"
  "other state predicate inputs"
  "### amount_in_range"
  "Bounds can be quoted numeric values or paths to accepted-state numeric values."
  "### text_eq / text_contains / text_starts_with / text_ends_with"
  "contains, starts with, or ends with a literal substring"
  "text_starts_with(/status.text, \"approved\")"
  "text_ends_with(/status.text, \"reviewer\")"
  "### num_eq / num_gt / num_gte / num_lt / num_lte"
  "Checks accepted-state numeric values."
  "numeric literal or a"
  "same pending commit"
  "Checks accepted-state boolean values."
  "It does not see booleans written by the same pending commit."
  "The local validator now derives \`post_to_path(/path)\`"
  "\`has_property(/path, \"a.b\")\` from"
  "\`text_eq\`, \`text_contains\`, \`text_starts_with\`,"
  "and \`text_ends_with\` from accepted-state strings"
  "comparisons from accepted-state numbers"
  "accepted-state booleans; other WASM-style predicate-test inputs remain"
  "explicit JSON until a validator path documents their replay binding."
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

WASM_README="$ROOT_DIR/rust/modality-wasm-validation/src/predicates/README.md"
wasm_patterns=(
  "These modules are locally unit-tested predicate evaluators."
  "not, by"
  "themselves, evidence that the local first-contract validator can derive each"
  "first-contract evidence boundary, use \`docs/reference/standard-predicates.md\`."
  "The checked object is explicit predicate-test input"
  "The checked amount is explicit predicate-test input"
  "\`has_property(/path, \"a.b\")\` directly from accepted-state JSON"
  "It also derives \`state_exists(/path)\` from accepted-state path existence."
  "It also derives \`text_eq\` from accepted-state strings"
  "It also derives \`text_contains\` from accepted-state strings"
  "It also derives \`text_starts_with\` and \`text_ends_with\` from accepted-state"
  "literal prefixes or suffixes"
  "\`amount_in_range(/path, \"min\", \"max\")\` directly from accepted-state numbers"
  "It also derives \`amount_in_range\` from accepted-state numbers"
  "It also derives \`num_eq\`, \`num_gt\`, \`num_gte\`, \`num_lt\`, and \`num_lte\` from"
  "accepted-state number path."
  "It also derives \`bool_true\` and \`bool_false\` from accepted-state booleans."
  "previously committed state"
  "The \"current time\" is \`context.timestamp\`"
  "document the trusted clock source"
  "The \`oracle_attests\` evaluator is unit-tested extension code, not current local"
  "signed payload to bind the oracle key, oracle path, claim, value, contract id,"
  "pending commit hash, and timestamp"
  "When \`replay_bundle_json\` is supplied"
  "canonical \`oracle_attests\` replay-bundle envelope"
  "requires the bundle's positive \`max_age_seconds\` freshness policy to match"
  "the predicate input"
  "\`expected_oracle_pubkey\`"
  "missing accepted-state oracle keys"
  "accepted-state oracle"
  "key mismatches"
  "rejects missing"
  "replay-bundle freshness policies, bundle/input"
  "freshness mismatches, missing accepted-state oracle keys"
  "key mismatches, malformed"
  "bundle/input"
  "freshness mismatches"
  "malformed JSON, non-canonical JSON bytes, wrong predicate"
  "names, and attestations that"
  "attestations that differ from the predicate input before an oracle"
  "claim can pass"
  "rejects missing oracle-path"
  "mismatched oracle paths, missing"
  "pending-commit bindings, and mismatched pending"
  "commit hashes"
  "commit hashes"
  "supply the accepted-state"
  "oracle-key lookup from replayed state"
  "freshness policy"
  "The \`modality-cli-contract\` local model-governance path also derives"
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
  "\`all_signed\`, \`threshold\`, \`modifies\`, \`post_to_path\`, \`has_property\`,"
  "\`state_exists\`, \`text_eq\`, \`text_contains\`, \`text_starts_with\`,"
  "\`text_ends_with\`, \`amount_in_range\`, \`num_eq\`, \`num_gt\`, \`num_gte\`, \`num_lt\`,"
  "\`num_lte\`, \`bool_true\`, and \`bool_false\` are enforced from"
  "replayable commit"
  "The local validator derives \`state_exists\` from accepted-state path existence"
  "a path written by the same pending commit is not evidence for that commit."
  "The local validator derives \`bool_true\` and \`bool_false\` from accepted-state"
  "The local validator also derives \`num_eq\`, \`num_gt\`, \`num_gte\`, \`num_lt\`, and"
  "\`num_lte\` from accepted-state numbers only."
  "The local validator derives \`text_eq\`, \`text_contains\`, \`text_starts_with\`, and"
  "\`text_ends_with\` from accepted-state"
  "[standard predicate evidence matrix](../reference/standard-predicates.md)"
  "Do not treat the future vocabulary below as runtime evidence until a validator"
  "Oracle, time, hash,"
  "WASM predicates are extension vocabulary in the local"
)

for pattern in "${language_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LANG_DOC"; then
    echo "language predicate reference is missing evidence-boundary text: $pattern" >&2
    exit 1
  fi
done

local_predicate_patterns=(
  '"text_starts_with" => match (args.first(), args.get(1))'
  '"text_ends_with" => match (args.first(), args.get(1))'
  "fn state_text_starts_with"
  "fn state_text_ends_with"
  "fn enforces_text_prefix_suffix_against_accepted_state_strings"
  "accepted state text at {path} does not start with {prefix}"
  "accepted state text at {path} does not end with {suffix}"
)

for pattern in "${local_predicate_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MODEL_GOVERNANCE"; then
    echo "local predicate implementation is missing text prefix/suffix evidence: $pattern" >&2
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
