#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODAL_BIN="${MODAL_BIN:-$ROOT_DIR/rust/target/debug/modal}"
MODALITY_BIN="${MODALITY_BIN:-$ROOT_DIR/rust/target/debug/modality}"

if [[ ! -x "$MODAL_BIN" ]]; then
  cat >&2 <<EOF
modal binary not found at: $MODAL_BIN

Build it first:
  cd "$ROOT_DIR/rust"
  cargo build -p modal --no-default-features --features contract-onboarding

Or pass an existing binary:
  MODAL_BIN=/path/to/modal $0
EOF
  exit 2
fi

if [[ ! -x "$MODALITY_BIN" ]]; then
  cat >&2 <<EOF
modality binary not found at: $MODALITY_BIN

The first-contract smoke synthesizes and verifies the governing witness model
before committing it. Build the language CLI first:
  cd "$ROOT_DIR/rust"
  cargo build -p modality

Or pass an existing binary:
  MODALITY_BIN=/path/to/modality $0
EOF
  exit 2
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

CONTRACT_DIR="$TMP_DIR/first-contract"
ALICE_PASSFILE="$TMP_DIR/alice.mod_passfile"
BOB_PASSFILE="$TMP_DIR/bob.mod_passfile"
export MODALITY_HOME="$TMP_DIR/modality-home"
mkdir -p "$MODALITY_HOME"

"$MODAL_BIN" --help | grep -q "Contract related commands"
"$MODAL_BIN" ai --help | grep -q "set"
"$MODAL_BIN" contract --help | grep -q "Create a new contract"
"$MODAL_BIN" set-named-id --help | grep -q "Set a state .id file"
"$MODAL_BIN" ai suggest-rule --help | grep -q "Plain-language description of the rule"

"$MODAL_BIN" contract create --dir "$CONTRACT_DIR" --output json >/dev/null
"$MODAL_BIN" id create --path "$ALICE_PASSFILE" >/dev/null
"$MODAL_BIN" id create --path "$BOB_PASSFILE" >/dev/null

ALICE_ID="$("$MODAL_BIN" id get --path "$ALICE_PASSFILE")"
BOB_ID="$("$MODAL_BIN" id get --path "$BOB_PASSFILE")"

"$MODAL_BIN" checkout --dir "$CONTRACT_DIR" >/dev/null
"$MODAL_BIN" set-named-id /parties/alice.id "$ALICE_PASSFILE" --dir "$CONTRACT_DIR" >/dev/null
"$MODAL_BIN" set-named-id /parties/bob.id "$BOB_PASSFILE" --dir "$CONTRACT_DIR" >/dev/null

"$MODAL_BIN" status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/identity-status.json"
"$MODAL_BIN" status --dir "$CONTRACT_DIR" >"$TMP_DIR/identity-status.txt"
grep -q "$ALICE_ID" "$CONTRACT_DIR/state/parties/alice.id"
grep -q "$BOB_ID" "$CONTRACT_DIR/state/parties/bob.id"
grep -q "/parties/alice.id" "$TMP_DIR/identity-status.json"
grep -q "/parties/bob.id" "$TMP_DIR/identity-status.json"
grep -q "Changes in state/" "$TMP_DIR/identity-status.txt"
grep -q "+ /parties/alice.id" "$TMP_DIR/identity-status.txt"
grep -q "+ /parties/bob.id" "$TMP_DIR/identity-status.txt"

if "$MODAL_BIN" ai suggest-rule \
  "after this commit either alice or bob must sign" \
  >"$TMP_DIR/suggested-rule.out" 2>"$TMP_DIR/suggested-rule.err"; then
  echo "expected unconfigured suggest-rule to fail" >&2
  exit 1
fi
grep -q "modal ai set" "$TMP_DIR/suggested-rule.err"

"$MODAL_BIN" add-rule --name authorized \
  --dir "$CONTRACT_DIR" \
  '[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)' >/dev/null
grep -Fq '[] always' "$CONTRACT_DIR/rules/authorized.modality"
grep -Fq 'starting_at $PARENT' "$CONTRACT_DIR/rules/authorized.modality"

"$MODALITY_BIN" model lint "$CONTRACT_DIR/rules/authorized.modality" \
  >"$TMP_DIR/authorized-rule-lint.out" 2>&1
grep -q "1 formula(s) lint-clean" "$TMP_DIR/authorized-rule-lint.out"

"$MODALITY_BIN" model synthesize \
  --rule "$CONTRACT_DIR/rules/authorized.modality" \
  --verify \
  --review-bundle "$CONTRACT_DIR/review/authorized.md" \
  -o "$CONTRACT_DIR/model/default.modality" >/dev/null
grep -q "# Modality Synthesis Review Bundle" "$CONTRACT_DIR/review/authorized.md"
grep -q "Status: passed (\`--verify\`)" "$CONTRACT_DIR/review/authorized.md"
grep -q "## Extracted Facts" "$CONTRACT_DIR/review/authorized.md"
grep -q "## Witness Model" "$CONTRACT_DIR/review/authorized.md"
"$MODALITY_BIN" model validate "$CONTRACT_DIR/model/default.modality" \
  --verbose >"$TMP_DIR/synthesized-model-validate.out" 2>&1
grep -q "Contract is valid!" "$TMP_DIR/synthesized-model-validate.out"
grep -q "Transitions: 3" "$TMP_DIR/synthesized-model-validate.out"
grep -q "All properties are predicates or commit method labels (verifier-observed)." \
  "$TMP_DIR/synthesized-model-validate.out"
"$MODALITY_BIN" model mermaid "$CONTRACT_DIR/model/default.modality" \
  >"$TMP_DIR/synthesized-model-mermaid.out"
grep -q "stateDiagram-v2" "$TMP_DIR/synthesized-model-mermaid.out"
grep -q "q0 --> q1 : +POST" "$TMP_DIR/synthesized-model-mermaid.out"
grep -q '"+POST +signed_by(/parties/alice.id)"' "$TMP_DIR/synthesized-model-mermaid.out"
grep -q '"+POST +signed_by(/parties/bob.id)"' "$TMP_DIR/synthesized-model-mermaid.out"
if grep -q "+MODEL" "$TMP_DIR/synthesized-model-mermaid.out"; then
  echo "synthesized first-contract witness mermaid still includes +MODEL" >&2
  cat "$TMP_DIR/synthesized-model-mermaid.out" >&2
  exit 1
fi
"$MODALITY_BIN" model view "$CONTRACT_DIR/model/default.modality" --no-open \
  >"$TMP_DIR/synthesized-model-view.out"
grep -q "^Wrote " "$TMP_DIR/synthesized-model-view.out"
VIEW_HTML="$(sed -n 's/^Wrote //p' "$TMP_DIR/synthesized-model-view.out")"
grep -q "mermaid.min.js" "$VIEW_HTML"
grep -q "stateDiagram-v2" "$VIEW_HTML"
grep -q "part flow" "$VIEW_HTML"
grep -q "q0 --&gt; q1: +POST" "$VIEW_HTML"
grep -q "+signed_by(/parties/alice.id)" "$VIEW_HTML"
grep -q "+signed_by(/parties/bob.id)" "$VIEW_HTML"
rm -f "$VIEW_HTML"

"$MODAL_BIN" commit \
  --all \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Initial contract setup" >/dev/null

"$MODAL_BIN" status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/status.json"
"$MODAL_BIN" status --dir "$CONTRACT_DIR" >"$TMP_DIR/status.txt"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/log.json"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" >"$TMP_DIR/log.txt"

grep -q "$ALICE_ID" "$CONTRACT_DIR/state/parties/alice.id"
grep -q "$BOB_ID" "$CONTRACT_DIR/state/parties/bob.id"
grep -q '"total_commits": 2' "$TMP_DIR/status.json"
grep -q '"model_state": "q1"' "$TMP_DIR/status.json"
grep -q "Model state: q1" "$TMP_DIR/status.txt"
grep -q '"commits":' "$TMP_DIR/log.json"
grep -q '"message": "Initial contract setup"' "$TMP_DIR/log.json"
grep -Eq '"signature_count": 1' "$TMP_DIR/log.json"
grep -q "$ALICE_ID" "$TMP_DIR/log.json"
grep -q "Message: Initial contract setup" "$TMP_DIR/log.txt"
grep -q "Signatures: 1" "$TMP_DIR/log.txt"
grep -q "$ALICE_ID" "$TMP_DIR/log.txt"
grep -q '\[\] always' "$CONTRACT_DIR/rules/authorized.modality"
grep -q 'q0 .* q1.*+POST' "$CONTRACT_DIR/model/default.modality"
grep -q 'q1 .* q1.*+POST.*+signed_by(/parties/alice.id)' "$CONTRACT_DIR/model/default.modality"
grep -q 'q1 .* q1.*+POST.*+signed_by(/parties/bob.id)' "$CONTRACT_DIR/model/default.modality"
if grep -q '+MODEL' "$CONTRACT_DIR/model/default.modality"; then
  echo "synthesized first-contract witness still includes +MODEL" >&2
  cat "$CONTRACT_DIR/model/default.modality" >&2
  exit 1
fi
sha256sum \
  "$CONTRACT_DIR/rules/authorized.modality" \
  "$CONTRACT_DIR/model/default.modality" \
  "$CONTRACT_DIR/review/authorized.md" \
  >"$TMP_DIR/accepted-artifacts.sha256"

"$MODAL_BIN" commit \
  --path /notes.text \
  --value "signed update" \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Signed update" >"$TMP_DIR/signed-post.json"

grep -q '"status": "committed"' "$TMP_DIR/signed-post.json"

"$MODAL_BIN" status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/post-status.json"
"$MODAL_BIN" status --dir "$CONTRACT_DIR" >"$TMP_DIR/post-status.txt"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/post-log.json"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" >"$TMP_DIR/post-log.txt"
"$MODAL_BIN" checkout --dir "$CONTRACT_DIR" >/dev/null

grep -q '"total_commits": 3' "$TMP_DIR/post-status.json"
grep -q '"model_state": "q1"' "$TMP_DIR/post-status.json"
grep -q "Model state: q1" "$TMP_DIR/post-status.txt"
grep -q '"message": "Signed update"' "$TMP_DIR/post-log.json"
grep -Eq '"signature_count": 1' "$TMP_DIR/post-log.json"
grep -q "$ALICE_ID" "$TMP_DIR/post-log.json"
grep -q "Message: Signed update" "$TMP_DIR/post-log.txt"
grep -q "Signatures: 1" "$TMP_DIR/post-log.txt"
grep -q "$ALICE_ID" "$TMP_DIR/post-log.txt"
grep -q "signed update" "$CONTRACT_DIR/state/notes.text"

if "$MODAL_BIN" commit \
  --path /unsigned.text \
  --value "unsigned update" \
  --dir "$CONTRACT_DIR" \
  --output json \
  --message "Unsigned update" >"$TMP_DIR/unsigned-post.json" 2>"$TMP_DIR/unsigned-post.err"; then
  echo "expected unsigned post-bootstrap commit to fail" >&2
  exit 1
fi

grep -q 'current states {"q1"}' "$TMP_DIR/unsigned-post.err"
grep -Eq "Closest candidate transition: (part flow )?candidate from current state q1: q1 -+> q1 \[\\+POST \\+signed_by\\(/parties/alice.id\\)\]; failed predicates: missing \\+signed_by\\(/parties/alice.id\\)" "$TMP_DIR/unsigned-post.err"
grep -q "Candidate transitions ranked by predicate distance:" "$TMP_DIR/unsigned-post.err"
grep -Eq "(part flow )?candidate from current state q1: q1 -+> q1 \[\\+POST \\+signed_by\\(/parties/bob.id\\)\]; failed predicates: missing \\+signed_by\\(/parties/bob.id\\)" "$TMP_DIR/unsigned-post.err"
grep -q "missing +signed_by(/parties/alice.id)" "$TMP_DIR/unsigned-post.err"
grep -q "missing +signed_by(/parties/bob.id)" "$TMP_DIR/unsigned-post.err"
diagnostic_order="$(tr '\n' ' ' <"$TMP_DIR/unsigned-post.err")"
if [[ "$diagnostic_order" != *'current states {"q1"}'*'Closest candidate transition:'*'Candidate transitions ranked by predicate distance:'*'missing +signed_by(/parties/bob.id)'* ]]; then
  echo "unsigned rejection diagnostics are not in current-state, closest, ranked, alternate order" >&2
  cat "$TMP_DIR/unsigned-post.err" >&2
  exit 1
fi

"$MODAL_BIN" status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/rejected-status.json"
"$MODAL_BIN" status --dir "$CONTRACT_DIR" >"$TMP_DIR/rejected-status.txt"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/rejected-log.json"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" >"$TMP_DIR/rejected-log.txt"
"$MODAL_BIN" checkout --dir "$CONTRACT_DIR" >/dev/null

grep -q '"total_commits": 3' "$TMP_DIR/rejected-status.json"
grep -q '"model_state": "q1"' "$TMP_DIR/rejected-status.json"
grep -q "Total commits: 3" "$TMP_DIR/rejected-status.txt"
grep -q "Model state: q1" "$TMP_DIR/rejected-status.txt"
grep -q '"message": "Signed update"' "$TMP_DIR/rejected-log.json"
grep -q "Message: Signed update" "$TMP_DIR/rejected-log.txt"
grep -q "Signatures: 1" "$TMP_DIR/rejected-log.txt"
grep -q "$ALICE_ID" "$TMP_DIR/rejected-log.txt"
grep -q "signed update" "$CONTRACT_DIR/state/notes.text"
sha256sum --check "$TMP_DIR/accepted-artifacts.sha256" >/dev/null
if [[ -e "$CONTRACT_DIR/state/unsigned.text" ]]; then
  echo "rejected unsigned commit changed replayed contract state" >&2
  exit 1
fi
if grep -q '"message": "Unsigned update"' "$TMP_DIR/rejected-log.json"; then
  echo "rejected unsigned commit was appended to the contract log" >&2
  exit 1
fi
if grep -q "Message: Unsigned update" "$TMP_DIR/rejected-log.txt"; then
  echo "rejected unsigned commit was shown in the human-readable contract log" >&2
  exit 1
fi

if "$MODAL_BIN" commit \
  --method model \
  --path /model/default.modality \
  --value "$(cat "$CONTRACT_DIR/model/default.modality")" \
  --dir "$CONTRACT_DIR" \
  --sign "$BOB_PASSFILE" \
  --output json \
  --message "Bob tries to replace the witness" \
  >"$TMP_DIR/bob-same-model.json" 2>"$TMP_DIR/bob-same-model.err"; then
  echo "expected Bob MODEL replacement of the current witness to fail" >&2
  cat "$TMP_DIR/bob-same-model.json" >&2
  exit 1
fi

grep -q 'current states {"q1"}' "$TMP_DIR/bob-same-model.err"
grep -q "missing +POST" "$TMP_DIR/bob-same-model.err"
grep -q "+POST +signed_by(/parties/bob.id)" "$TMP_DIR/bob-same-model.err"

cat >"$CONTRACT_DIR/model/default.modality" <<'EOF'
model Contract {
  part flow {
    q0 --> q1: +POST
    q1 --> q1: +signed_by(/parties/alice.id)
    q1 --> q1: +signed_by(/parties/bob.id)
  }
}
EOF

grep -q "q1 --> q1: +signed_by(/parties/alice.id)" \
  "$CONTRACT_DIR/model/default.modality"
grep -q "q1 --> q1: +signed_by(/parties/bob.id)" \
  "$CONTRACT_DIR/model/default.modality"
if grep -q '+MODEL' "$CONTRACT_DIR/model/default.modality"; then
  echo "Bob's replacement witness still includes +MODEL" >&2
  cat "$CONTRACT_DIR/model/default.modality" >&2
  exit 1
fi

"$MODALITY_BIN" model validate "$CONTRACT_DIR/model/default.modality" \
  --verbose >"$TMP_DIR/replaced-model-validate.out" 2>&1
grep -q "Contract is valid!" "$TMP_DIR/replaced-model-validate.out"
grep -q "Transitions: 3" "$TMP_DIR/replaced-model-validate.out"

"$MODAL_BIN" commit \
  --all \
  --dir "$CONTRACT_DIR" \
  --sign "$BOB_PASSFILE" \
  --output json \
  --message "Let Bob replace the witness" >"$TMP_DIR/bob-model-replacement.json"

grep -q '"status": "committed"' "$TMP_DIR/bob-model-replacement.json"

"$MODAL_BIN" status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/bob-status.json"
"$MODAL_BIN" status --dir "$CONTRACT_DIR" >"$TMP_DIR/bob-status.txt"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/bob-log.json"
"$MODAL_BIN" log --dir "$CONTRACT_DIR" >"$TMP_DIR/bob-log.txt"

grep -q '"total_commits": 4' "$TMP_DIR/bob-status.json"
grep -q '"model_state": "q1"' "$TMP_DIR/bob-status.json"
grep -q "Total commits: 4" "$TMP_DIR/bob-status.txt"
grep -q "Model state: q1" "$TMP_DIR/bob-status.txt"
grep -q '"message": "Let Bob replace the witness"' "$TMP_DIR/bob-log.json"
grep -q "$BOB_ID" "$TMP_DIR/bob-log.json"
grep -q "Message: Let Bob replace the witness" "$TMP_DIR/bob-log.txt"
grep -q "model /model/default.modality" "$TMP_DIR/bob-log.txt"
grep -q "q1 --> q1: +signed_by(/parties/bob.id)" \
  "$CONTRACT_DIR/model/default.modality"
if ! grep -E 'authorized\.modality|authorized\.md' "$TMP_DIR/accepted-artifacts.sha256" \
  | sha256sum --check >/dev/null; then
  echo "Bob's witness replacement changed the accepted rule or review bundle" >&2
  exit 1
fi
if grep 'default.modality' "$TMP_DIR/accepted-artifacts.sha256" \
  | sha256sum --check >/dev/null 2>&1; then
  echo "Bob's witness replacement did not change model/default.modality" >&2
  exit 1
fi

echo "first-contract CLI smoke passed"
