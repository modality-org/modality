#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/getting-started/first-contract.md"
FIRST_CONTRACT_SMOKE="$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"

required_patterns=(
  "modal contract create --dir ./my-first-contract"
  "modal id create --name example/alice"
  "modal id create --name example/bob"
  "modal set-named-id /parties/alice.id example/alice"
  "modal set-named-id /parties/bob.id example/bob"
  "modal status"
  "Changes in state/"
  "+ /parties/alice.id"
  "+ /parties/bob.id"
  "\`~/.modality/passfiles/example/\`"
  "\`~/.modality/ids/example/\`"
  "\`modal ai suggest-rule\` can"
  "## 4. Add Protection Rules"
  "## 5. Synthesize a Witness Model"
  "every commit after this one must be signed by either Alice or"
  "You don't need to know Modality syntax yet"
  "your choice of AI"
  "[AI Commands](/docs/cli/ai-commands)"
  "modal ai set"
  "modal ai suggest-rule \"after this commit either alice or bob must sign\""
  "yours may differ"
  "modal add-rule --name authorized"
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
  "layout: elk"
  'q1 --> q1 : "+signed_by(/parties/alice.id)"'
  'q1 --> q1 : "+signed_by(/parties/bob.id)"'
  "something off about the witness model"
  "1 formula(s) lint-clean"
  "Contract is valid!"
  "Transitions: 2"
  "Transitions: 3"
  "review"
  "the synthesized candidate"
  "parser-backed extracted facts"
  "passed verifier result"
  "known gaps"
  "## 6. Commit and Verify"
  "modal commit --all -m \"Initial contract setup\""
  "accepted rule, witness model, and synthesis review bundle"
  "synthesis review bundle"
  "rejected"
  "does not alter those accepted artifacts"
  "## 7. Prove the Rule Is Active"
  "missing +signed_by"
  "should still end at the last accepted signed update"
  "modal checkout"
  "state/notes.text"
  "state/unsigned.text"
  "\`rules/authorized.modality\`, \`model/default.modality\`, and"
  "\`review/authorized.md\` files should also be"
  "## 8. Let Bob Replace the Witness"
  "Bob tries to commit"
  "only lets Alice sign"
  "The rule itself does let him"
  "q0 --> q1"
  "q1 --> q1: +signed_by(/parties/alice.id)"
  "q1 --> q1: +signed_by(/parties/bob.id)"
  "signed Alice move or a signed"
  "modal commit --all --sign example/bob"
  "Let Bob replace the witness"
  "Bob replaced the witness"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "first-contract guide is missing verified onboarding text: $pattern" >&2
    exit 1
  fi
done

if [[ ! -f "$ROOT_DIR/docs/cli/ai-commands.md" ]]; then
  echo "first-contract links to missing AI commands page: docs/cli/ai-commands.md" >&2
  exit 1
fi

set_bob_line="$(grep -n 'modal set-named-id /parties/bob.id example/bob' "$DOC" | cut -d: -f1 | head -1)"
identity_status_line="$(grep -n 'Changes in state/' "$DOC" | cut -d: -f1 | head -1)"
rules_line="$(grep -n '## 4. Add Protection Rules' "$DOC" | cut -d: -f1 | head -1)"
rule_intent_line="$(grep -n 'every commit after this one must be signed by either Alice or' "$DOC" | cut -d: -f1 | head -1)"
suggest_rule_line="$(grep -n 'modal ai suggest-rule "after this commit either alice or bob must sign"' "$DOC" | cut -d: -f1 | head -1)"
ai_setup_line="$(grep -nF '[AI Commands](/docs/cli/ai-commands)' "$DOC" | cut -d: -f1 | head -1)"
add_rule_line="$(grep -n 'modal add-rule --name authorized' "$DOC" | cut -d: -f1 | head -1)"
synth_line="$(grep -n '## 5. Synthesize a Witness Model' "$DOC" | cut -d: -f1 | head -1)"
lint_line="$(grep -n 'modality model lint rules/authorized.modality' "$DOC" | cut -d: -f1 | head -1)"
validate_line="$(grep -n 'modality model validate model/default.modality --verbose' "$DOC" | cut -d: -f1 | head -1)"
mermaid_cmd_line="$(grep -n 'modality model mermaid model/default.modality' "$DOC" | cut -d: -f1 | head -1)"
view_cmd_line="$(grep -n 'modality model view model/default.modality' "$DOC" | cut -d: -f1 | head -1)"
mermaid_diagram_line="$(grep -n 'The same witness as a state diagram:' "$DOC" | cut -d: -f1 | head -1)"
vague_note_line="$(grep -n 'something off about the witness model' "$DOC" | cut -d: -f1 | head -1)"
commit_line="$(grep -n '## 6. Commit and Verify' "$DOC" | cut -d: -f1 | head -1)"
prove_line="$(grep -n '## 7. Prove the Rule Is Active' "$DOC" | cut -d: -f1 | head -1)"
bob_section_line="$(grep -n '## 8. Let Bob Replace the Witness' "$DOC" | cut -d: -f1 | head -1)"
bob_try_line="$(grep -n 'Bob tries to commit' "$DOC" | cut -d: -f1 | head -1)"
rule_lets_bob_line="$(grep -n 'The rule itself does let him' "$DOC" | cut -d: -f1 | head -1)"
bob_model_line="$(grep -n 'q1 --> q1: +signed_by(/parties/bob.id)' "$DOC" | cut -d: -f1 | head -1)"
bob_commit_line="$(grep -n 'modal commit --all --sign example/bob' "$DOC" | cut -d: -f1 | head -1)"

if [[ "$set_bob_line" -ge "$identity_status_line" || "$identity_status_line" -ge "$rules_line" || "$rules_line" -ge "$rule_intent_line" || "$rule_intent_line" -ge "$ai_setup_line" || "$ai_setup_line" -ge "$suggest_rule_line" || "$suggest_rule_line" -ge "$add_rule_line" || "$add_rule_line" -ge "$synth_line" || "$synth_line" -ge "$lint_line" || "$lint_line" -ge "$validate_line" || "$validate_line" -ge "$mermaid_cmd_line" || "$mermaid_cmd_line" -ge "$view_cmd_line" || "$view_cmd_line" -ge "$mermaid_diagram_line" || "$mermaid_diagram_line" -ge "$vague_note_line" || "$vague_note_line" -ge "$commit_line" || "$commit_line" -ge "$prove_line" || "$prove_line" -ge "$bob_section_line" || "$bob_section_line" -ge "$bob_try_line" || "$bob_try_line" -ge "$rule_lets_bob_line" || "$rule_lets_bob_line" -ge "$bob_model_line" || "$bob_model_line" -ge "$bob_commit_line" ]]; then
  echo "first-contract guide should add identities, inspect uncommitted status, link to AI setup, suggest and add rules, lint them, synthesize and validate the witness, visualize it after the model text, then commit, prove the rule, and let Bob replace the incomplete witness" >&2
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
  "Transitions: 2"
  "synthesized-model-mermaid.out"
  "stateDiagram-v2"
  'q0 --> q1'
  '"+signed_by(/parties/alice.id)"'
  "synthesized-model-view.out"
  "model view"
  "--no-open"
  "mermaid.esm.min.mjs"
  "registerLayoutLoaders"
  "mermaid-layout-elk"
  "part flow"
  "All properties are predicates or commit method labels (verifier-observed)."
  "sha256sum --check"
  "accepted-artifacts.sha256"
  "identity-status.json"
  "identity-status.txt"
  "Changes in state/"
  "+ /parties/alice.id"
  "+ /parties/bob.id"
  "add-rule --name authorized"
  "ai suggest-rule"
  "after this commit either alice or bob must sign"
  "suggested-rule.out"
  "ai --help"
  "ai suggest-rule --help"
  "modal ai set"
  "rejected unsigned commit changed replayed contract state"
  "rejected unsigned commit was appended to the contract log"
  "rejected unsigned commit was shown in the human-readable contract log"
  'grep -q "signed update" "$CONTRACT_DIR/state/notes.text"'
  '[[ -e "$CONTRACT_DIR/state/unsigned.text" ]]'
  "Bob tries to commit"
  "bob-empty-commit.err"
  "q0 --> q1"
  "q1 --> q1: +signed_by(/parties/alice.id)"
  "q1 --> q1: +signed_by(/parties/bob.id)"
  "replaced-model-validate.out"
  "Transitions: 3"
  "Let Bob replace the witness"
  '"total_commits": 4'
  "bob-model-replacement.json"
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

if grep -Fq "+MODEL" "$DOC"; then
  echo "first-contract guide should not put +MODEL on witness transitions" >&2
  exit 1
fi

if grep -Fq "+POST" "$DOC"; then
  echo "first-contract guide should not put +POST on witness transitions" >&2
  exit 1
fi

if grep -Fq ' or +signed_by' "$DOC"; then
  echo "first-contract guide should not put or-combined transition labels in mermaid" >&2
  exit 1
fi

echo "first-contract doc check passed"
