#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/getting-started/first-contract.md"
FIRST_CONTRACT_SMOKE="$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"

required_patterns=(
  "## 4. Add Protection Rules"
  "## 5. Synthesize the Witness Model"
  "mkdir -p model"
  "mkdir -p review"
  "modality model lint rules/authorized.modality"
  "--review-bundle review/authorized.md"
  "modality model validate model/default.modality --verbose"
  "The lint command should report that the rule formula is lint-clean"
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
  "should still show \`Total commits: 3\` and \`Model state: q1\`"
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
lint_line="$(grep -n 'modality model lint rules/authorized.modality' "$DOC" | cut -d: -f1 | head -1)"
validate_line="$(grep -n 'modality model validate model/default.modality --verbose' "$DOC" | cut -d: -f1 | head -1)"
commit_line="$(grep -n '## 6. Commit and Verify' "$DOC" | cut -d: -f1 | head -1)"

if [[ "$rules_line" -ge "$synth_line" || "$synth_line" -ge "$lint_line" || "$lint_line" -ge "$validate_line" || "$validate_line" -ge "$commit_line" ]]; then
  echo "first-contract guide should add rules, lint them, synthesize and validate the witness, then commit" >&2
  exit 1
fi

first_contract_smoke_patterns=(
  "authorized-rule-lint.out"
  "1 formula(s) lint-clean"
  "# Modality Synthesis Review Bundle"
  'Status: passed (\`--verify\`)'
  "## Extracted Facts"
  "## Witness Model"
  "synthesized-model-validate.out"
  "Contract is valid!"
  "Transitions: 4"
  "All properties are predicates or commit method labels (verifier-observed)."
  "sha256sum --check"
  "accepted-artifacts.sha256"
  "rejected unsigned commit changed replayed contract state"
  "rejected unsigned commit was appended to the contract log"
  "rejected unsigned commit was shown in the human-readable contract log"
  'grep -q "signed update" "$CONTRACT_DIR/state/notes.text"'
  '[[ -e "$CONTRACT_DIR/state/unsigned.text" ]]'
)

for pattern in "${first_contract_smoke_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$FIRST_CONTRACT_SMOKE"; then
    echo "first-contract smoke is missing documented first-contract assertion: $pattern" >&2
    exit 1
  fi
done

echo "first-contract doc check passed"
