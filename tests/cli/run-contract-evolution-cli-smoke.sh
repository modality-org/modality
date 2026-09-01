#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODAL_BIN="${MODAL_BIN:-$ROOT_DIR/rust/target/debug/modal}"

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

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

CONTRACT_DIR="$TMP_DIR/evolving-contract"
ALICE_PASSFILE="$TMP_DIR/alice.mod_passfile"
BOB_PASSFILE="$TMP_DIR/bob.mod_passfile"

"$MODAL_BIN" contract create --dir "$CONTRACT_DIR" --output json >/dev/null
"$MODAL_BIN" id create --path "$ALICE_PASSFILE" >/dev/null
"$MODAL_BIN" id create --path "$BOB_PASSFILE" >/dev/null

ALICE_ID="$("$MODAL_BIN" id get --path "$ALICE_PASSFILE")"
BOB_ID="$("$MODAL_BIN" id get --path "$BOB_PASSFILE")"

"$MODAL_BIN" c checkout --dir "$CONTRACT_DIR" >/dev/null
"$MODAL_BIN" c set-named-id /parties/alice.id "$ALICE_PASSFILE" --dir "$CONTRACT_DIR" >/dev/null
"$MODAL_BIN" c set-named-id /parties/bob.id "$BOB_PASSFILE" --dir "$CONTRACT_DIR" >/dev/null

cat >"$CONTRACT_DIR/model/default.modality" <<'EOF'
export default model {
  initial q0

  q0 -> q1 [+POST +MODEL]
  q1 -> q1 [+POST +signed_by(/parties/alice.id)]
  q1 -> q1 [+RULE +signed_by(/parties/alice.id)]
  q1 -> q1 [+MODEL +signed_by(/parties/alice.id)]
}
EOF

"$MODAL_BIN" c commit \
  --all \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "V1 setup" >"$TMP_DIR/v1-setup.json"

grep -q '"status": "committed"' "$TMP_DIR/v1-setup.json"

if "$MODAL_BIN" c commit \
  --path /notes/unsigned-before-rule.text \
  --value "unsigned before rule" \
  --dir "$CONTRACT_DIR" \
  --output json \
  --message "Unsigned before rule" >"$TMP_DIR/unsigned-before-rule.json" 2>"$TMP_DIR/unsigned-before-rule.err"; then
  echo "expected unsigned V1 post to fail" >&2
  exit 1
fi

grep -q "missing +signed_by(/parties/alice.id)" "$TMP_DIR/unsigned-before-rule.err"

mkdir -p "$CONTRACT_DIR/rules"
cat >"$CONTRACT_DIR/rules/signed-posts.modality" <<'EOF'
export default rule {
  starting_at $PARENT
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
EOF

"$MODAL_BIN" c commit \
  --all \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Add signed-post rule" >"$TMP_DIR/add-rule.json"

grep -q '"status": "committed"' "$TMP_DIR/add-rule.json"
grep -q '\[\] always' "$CONTRACT_DIR/rules/signed-posts.modality"

cat >"$CONTRACT_DIR/model/default.modality" <<'EOF'
export default model {
  initial q0

  q0 -> q1 [+POST +MODEL]
  q1 -> q1 [+POST]
  q1 -> q1 [+RULE +signed_by(/parties/alice.id)]
  q1 -> q1 [+MODEL +signed_by(/parties/alice.id)]
}
EOF

if "$MODAL_BIN" c commit \
  --all \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Bad unsigned model" >"$TMP_DIR/bad-model.json" 2>"$TMP_DIR/bad-model.err"; then
  echo "expected unsigned replacement model to fail" >&2
  exit 1
fi

grep -q "Model violates rule" "$TMP_DIR/bad-model.err"
grep -q "failed anchor state: q1" "$TMP_DIR/bad-model.err"
grep -Fq "formula: [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)" "$TMP_DIR/bad-model.err"

cat >"$CONTRACT_DIR/model/default.modality" <<'EOF'
export default model {
  initial q0

  q0 -> q1 [+POST +MODEL]
  q1 -> q1 [+POST +signed_by(/parties/alice.id)]
  q1 -> q1 [+POST +signed_by(/parties/bob.id)]
  q1 -> q1 [+RULE +signed_by(/parties/alice.id)]
  q1 -> q1 [+MODEL +signed_by(/parties/alice.id)]
}
EOF

"$MODAL_BIN" c commit \
  --all \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Accept Bob in V2" >"$TMP_DIR/good-model.json"

grep -q '"status": "committed"' "$TMP_DIR/good-model.json"

"$MODAL_BIN" c commit \
  --path /notes/bob.text \
  --value "bob signed update" \
  --dir "$CONTRACT_DIR" \
  --sign "$BOB_PASSFILE" \
  --output json \
  --message "Bob signed V2 update" >"$TMP_DIR/bob-post.json"

grep -q '"status": "committed"' "$TMP_DIR/bob-post.json"

if "$MODAL_BIN" c commit \
  --path /notes/unsigned-after-v2.text \
  --value "unsigned after v2" \
  --dir "$CONTRACT_DIR" \
  --output json \
  --message "Unsigned after V2" >"$TMP_DIR/unsigned-after-v2.json" 2>"$TMP_DIR/unsigned-after-v2.err"; then
  echo "expected unsigned V2 post to fail" >&2
  exit 1
fi

grep -q "missing +signed_by(/parties/alice.id)" "$TMP_DIR/unsigned-after-v2.err"
grep -q "missing +signed_by(/parties/bob.id)" "$TMP_DIR/unsigned-after-v2.err"

"$MODAL_BIN" c status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/status.json"
"$MODAL_BIN" c log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/log.json"

grep -q '"total_commits": 5' "$TMP_DIR/status.json"
grep -q '"model_state": "q1"' "$TMP_DIR/status.json"
grep -q '"message": "Add signed-post rule"' "$TMP_DIR/log.json"
grep -q '"message": "Accept Bob in V2"' "$TMP_DIR/log.json"
grep -q '"message": "Bob signed V2 update"' "$TMP_DIR/log.json"
grep -q "$ALICE_ID" "$TMP_DIR/log.json"
grep -q "$BOB_ID" "$TMP_DIR/log.json"

MEMBERS_CONTRACT_DIR="$TMP_DIR/protected-members-contract"
CAROL_PASSFILE="$TMP_DIR/carol.mod_passfile"

"$MODAL_BIN" contract create --dir "$MEMBERS_CONTRACT_DIR" --output json >/dev/null
"$MODAL_BIN" id create --path "$CAROL_PASSFILE" >/dev/null

"$MODAL_BIN" c checkout --dir "$MEMBERS_CONTRACT_DIR" >/dev/null
"$MODAL_BIN" c set-named-id /members/alice.id "$ALICE_PASSFILE" --dir "$MEMBERS_CONTRACT_DIR" >/dev/null

cat >"$MEMBERS_CONTRACT_DIR/model/default.modality" <<'EOF'
export default model {
  initial q0

  q0 -> active [+POST +MODEL]
  active -> active [+POST +any_signed(/members) -modifies(/members)]
  active -> active [+POST +modifies(/members) +all_signed(/members)]
  active -> active [+MODEL +all_signed(/members)]
}
EOF

"$MODAL_BIN" c commit \
  --all \
  --dir "$MEMBERS_CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Protected members setup" >"$TMP_DIR/members-setup.json"

grep -q '"status": "committed"' "$TMP_DIR/members-setup.json"

"$MODAL_BIN" c commit \
  --path /notes/member-update.text \
  --value "one member can update ordinary state" \
  --dir "$MEMBERS_CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Alice ordinary update" >"$TMP_DIR/alice-ordinary-update.json"

grep -q '"status": "committed"' "$TMP_DIR/alice-ordinary-update.json"

"$MODAL_BIN" c set-named-id /members/bob.id "$BOB_PASSFILE" --dir "$MEMBERS_CONTRACT_DIR" >/dev/null

"$MODAL_BIN" c commit \
  --all \
  --dir "$MEMBERS_CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Alice adds Bob" >"$TMP_DIR/alice-adds-bob.json"

grep -q '"status": "committed"' "$TMP_DIR/alice-adds-bob.json"

"$MODAL_BIN" c commit \
  --path /notes/bob-member-update.text \
  --value "bob can update ordinary state after membership acceptance" \
  --dir "$MEMBERS_CONTRACT_DIR" \
  --sign "$BOB_PASSFILE" \
  --output json \
  --message "Bob ordinary update" >"$TMP_DIR/bob-ordinary-update.json"

grep -q '"status": "committed"' "$TMP_DIR/bob-ordinary-update.json"

cat >"$MEMBERS_CONTRACT_DIR/model/default.modality" <<'EOF'
export default model {
  initial q0

  q0 -> active [+POST +MODEL]
  active -> active [+POST +any_signed(/members) -modifies(/members)]
  active -> active [+POST +modifies(/members) +all_signed(/members)]
  active -> active [+POST +signed_by(/members/bob.id) -modifies(/members)]
  active -> active [+MODEL +all_signed(/members)]
}
EOF

if "$MODAL_BIN" c commit \
  --all \
  --dir "$MEMBERS_CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Alice-only model replacement" >"$TMP_DIR/alice-only-model-replacement.json" 2>"$TMP_DIR/alice-only-model-replacement.err"; then
  echo "expected one-signer model replacement to fail after Bob is accepted" >&2
  exit 1
fi

grep -q "missing +all_signed(/members)" "$TMP_DIR/alice-only-model-replacement.err"

"$MODAL_BIN" c commit \
  --all \
  --dir "$MEMBERS_CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --sign "$BOB_PASSFILE" \
  --output json \
  --message "Alice and Bob replace model" >"$TMP_DIR/alice-bob-model-replacement.json"

grep -q '"status": "committed"' "$TMP_DIR/alice-bob-model-replacement.json"

cat >"$MEMBERS_CONTRACT_DIR/model/default.modality" <<'EOF'
export default model {
  initial q0

  q0 -> active [+POST +MODEL]
  active -> active [+POST +any_signed(/members) -modifies(/members)]
  active -> active [+POST +modifies(/members) +all_signed(/members)]
  active -> active [+MODEL +all_signed(/members)]
}
EOF

"$MODAL_BIN" c set-named-id /members/carol.id "$CAROL_PASSFILE" --dir "$MEMBERS_CONTRACT_DIR" >/dev/null

if "$MODAL_BIN" c commit \
  --all \
  --dir "$MEMBERS_CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Alice-only member change" >"$TMP_DIR/alice-only-member-change.json" 2>"$TMP_DIR/alice-only-member-change.err"; then
  echo "expected one-signer protected member change to fail" >&2
  exit 1
fi

grep -q "missing +all_signed(/members)" "$TMP_DIR/alice-only-member-change.err"

"$MODAL_BIN" c status --dir "$MEMBERS_CONTRACT_DIR" --output json >"$TMP_DIR/members-status.json"
"$MODAL_BIN" c log --dir "$MEMBERS_CONTRACT_DIR" --output json >"$TMP_DIR/members-log.json"

grep -q '"total_commits": 6' "$TMP_DIR/members-status.json"
grep -q '"model_state": "active"' "$TMP_DIR/members-status.json"
grep -q '"message": "Alice ordinary update"' "$TMP_DIR/members-log.json"
grep -q '"message": "Alice adds Bob"' "$TMP_DIR/members-log.json"
grep -q '"message": "Bob ordinary update"' "$TMP_DIR/members-log.json"
grep -q '"message": "Alice and Bob replace model"' "$TMP_DIR/members-log.json"
grep -q "$ALICE_ID" "$TMP_DIR/members-log.json"
grep -q "$BOB_ID" "$TMP_DIR/members-log.json"

echo "contract evolution CLI smoke passed"
