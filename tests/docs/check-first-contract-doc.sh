#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/getting-started/first-contract.md"
FIRST_CONTRACT_SMOKE="$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"

required_patterns=(
  "modal contract create --dir ./my-first-contract"
  "modal id create --name example/alice"
  "modal id create --name example/bob"
  "modal c set-named-id /parties/alice.id example/alice"
  "modal c set-named-id /parties/bob.id example/bob"
  "modal c status"
  "Changes in state/"
  "+ /parties/alice.id"
  "+ /parties/bob.id"
  "\`~/.modality/passfiles/example/\`"
  "\`~/.modality/ids/example/\`"
  "\`modal c ai suggest-rule\` can"
  "## 4. Add Protection Rules"
  "## 5. Synthesize the Witness Model"
  "every commit after this one must be signed by either Alice or"
  "You don't need to know Modality syntax yet"
  "modal c ai suggest-rule \"after this commit either alice or bob must sign\""
  "modal c add-rule --name authorized"
  "✅ Rule 'authorized' added to /rules/authorized.modality"
  "modality model lint rules/authorized.modality"
  "--review-bundle review/authorized.md"
  "modality model validate model/default.modality --verbose"
  "modality model mermaid model/default.modality"
  "modality model view model/default.modality"
  "default browser"
  '```mermaid'
  "stateDiagram-v2"
  "The same witness as a state diagram:"
  "1 formula(s) lint-clean"
  "Contract is valid!"
  "Transitions: 4"
  "review"
  "the synthesized candidate"
  "parser-backed extracted facts"
  "passed verifier result"
  "known gaps"
  "## 6. Commit and Verify"
  "modal c commit --all --sign example/alice"
  "accepted rule, witness model, and synthesis review bundle"
  "synthesis review bundle"
  "rejected"
  "does not alter those accepted artifacts"
  "## 7. Prove the Rule Is Active"
  "missing +signed_by"
  "should still end at the last accepted signed update"
  "modal c checkout"
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

set_bob_line="$(grep -n 'modal c set-named-id /parties/bob.id example/bob' "$DOC" | cut -d: -f1 | head -1)"
identity_status_line="$(grep -n 'Changes in state/' "$DOC" | cut -d: -f1 | head -1)"
rules_line="$(grep -n '## 4. Add Protection Rules' "$DOC" | cut -d: -f1 | head -1)"
rule_intent_line="$(grep -n 'every commit after this one must be signed by either Alice or' "$DOC" | cut -d: -f1 | head -1)"
suggest_rule_line="$(grep -n 'modal c ai suggest-rule "after this commit either alice or bob must sign"' "$DOC" | cut -d: -f1 | head -1)"
add_rule_line="$(grep -n 'modal c add-rule --name authorized' "$DOC" | cut -d: -f1 | head -1)"
synth_line="$(grep -n '## 5. Synthesize the Witness Model' "$DOC" | cut -d: -f1 | head -1)"
lint_line="$(grep -n 'modality model lint rules/authorized.modality' "$DOC" | cut -d: -f1 | head -1)"
validate_line="$(grep -n 'modality model validate model/default.modality --verbose' "$DOC" | cut -d: -f1 | head -1)"
mermaid_cmd_line="$(grep -n 'modality model mermaid model/default.modality' "$DOC" | cut -d: -f1 | head -1)"
view_cmd_line="$(grep -n 'modality model view model/default.modality' "$DOC" | cut -d: -f1 | head -1)"
mermaid_diagram_line="$(grep -n 'The same witness as a state diagram:' "$DOC" | cut -d: -f1 | head -1)"
commit_line="$(grep -n '## 6. Commit and Verify' "$DOC" | cut -d: -f1 | head -1)"

if [[ "$set_bob_line" -ge "$identity_status_line" || "$identity_status_line" -ge "$rules_line" || "$rules_line" -ge "$rule_intent_line" || "$rule_intent_line" -ge "$suggest_rule_line" || "$suggest_rule_line" -ge "$add_rule_line" || "$add_rule_line" -ge "$synth_line" || "$synth_line" -ge "$lint_line" || "$lint_line" -ge "$validate_line" || "$validate_line" -ge "$mermaid_cmd_line" || "$mermaid_cmd_line" -ge "$view_cmd_line" || "$view_cmd_line" -ge "$mermaid_diagram_line" || "$mermaid_diagram_line" -ge "$commit_line" ]]; then
  echo "first-contract guide should add identities, inspect uncommitted status, add rules, lint them, synthesize and validate the witness, visualize it after the model text, then commit" >&2
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
  "synthesized-model-mermaid.out"
  "stateDiagram-v2"
  'q0 --> q1 : +POST +MODEL'
  '"+POST +signed_by(/parties/alice.id)"'
  '"+POST +signed_by(/parties/bob.id)"'
  "synthesized-model-view.out"
  "model view"
  "--no-open"
  "mermaid.min.js"
  "All properties are predicates or commit method labels (verifier-observed)."
  "sha256sum --check"
  "accepted-artifacts.sha256"
  "identity-status.json"
  "identity-status.txt"
  "Changes in state/"
  "+ /parties/alice.id"
  "+ /parties/bob.id"
  "c add-rule --name authorized"
  "c ai suggest-rule"
  "after this commit either alice or bob must sign"
  "suggested-rule.out"
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

for forbidden_pattern in \
  "mkdir -p rules" \
  "mkdir -p model" \
  "mkdir -p review" \
  "cat > rules/authorized.modality"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "first-contract guide still uses leftover directory/file scaffolding: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "first-contract doc check passed"
