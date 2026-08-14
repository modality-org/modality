#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/getting-started/first-contract.md"

required_patterns=(
  "## 4. Add Protection Rules"
  "## 5. Synthesize the Witness Model"
  "mkdir -p model"
  "mkdir -p review"
  "--review-bundle review/authorized.md"
  "modality model validate model/default.modality --verbose"
  "Contract is valid!"
  "Transitions: 4"
  "review"
  "the synthesized candidate"
  "parser-backed extracted facts"
  "passed verifier result"
  "known gaps"
  "## 6. Commit and Verify"
  "modal c commit --all --sign alice.mod_passfile"
  "accepted rule, witness model, and synthesis review bundle"
  "synthesis review bundle"
  "rejected"
  "does not alter those accepted artifacts"
  "## 7. Prove the Rule Is Active"
  "missing \`signed_by\`"
  "predicate diagnostics"
  "should still end at the last accepted signed update"
  "After \`modal c checkout\`"
  "state/notes.text"
  "state/unsigned.text"
  "\`rules/authorized.modality\`, \`model/default.modality\`, and"
  "\`review/authorized.md\` files should also be"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "first-contract guide is missing verified onboarding text: $pattern" >&2
    exit 1
  fi
done

rules_line="$(grep -n '## 4. Add Protection Rules' "$DOC" | cut -d: -f1 | head -1)"
synth_line="$(grep -n '## 5. Synthesize the Witness Model' "$DOC" | cut -d: -f1 | head -1)"
validate_line="$(grep -n 'modality model validate model/default.modality --verbose' "$DOC" | cut -d: -f1 | head -1)"
commit_line="$(grep -n '## 6. Commit and Verify' "$DOC" | cut -d: -f1 | head -1)"

if [[ "$rules_line" -ge "$synth_line" || "$synth_line" -ge "$validate_line" || "$validate_line" -ge "$commit_line" ]]; then
  echo "first-contract guide should add rules, synthesize and validate the witness, then commit" >&2
  exit 1
fi

echo "first-contract doc check passed"
