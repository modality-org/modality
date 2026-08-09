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

CONTRACT_DIR="$TMP_DIR/first-contract"
ALICE_PASSFILE="$TMP_DIR/alice.mod_passfile"
BOB_PASSFILE="$TMP_DIR/bob.mod_passfile"

"$MODAL_BIN" --help | grep -q "Contract related commands"
"$MODAL_BIN" contract --help | grep -q "Create a new contract"
"$MODAL_BIN" c set-named-id --help | grep -q "Set a state .id file"

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
  q1 -> q1 [+POST +signed_by(/parties/bob.id)]
  q1 -> q1 [+MODEL +signed_by(/parties/alice.id)]
}
EOF

mkdir -p "$CONTRACT_DIR/rules"
cat >"$CONTRACT_DIR/rules/authorized.modality" <<'EOF'
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
  --message "Initial contract setup" >/dev/null

"$MODAL_BIN" c status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/status.json"
"$MODAL_BIN" c status --dir "$CONTRACT_DIR" >"$TMP_DIR/status.txt"
"$MODAL_BIN" c log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/log.json"
"$MODAL_BIN" c log --dir "$CONTRACT_DIR" >"$TMP_DIR/log.txt"

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

"$MODAL_BIN" c commit \
  --path /notes.text \
  --value "signed update" \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Signed update" >"$TMP_DIR/signed-post.json"

grep -q '"status": "committed"' "$TMP_DIR/signed-post.json"

"$MODAL_BIN" c status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/post-status.json"
"$MODAL_BIN" c status --dir "$CONTRACT_DIR" >"$TMP_DIR/post-status.txt"
"$MODAL_BIN" c log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/post-log.json"
"$MODAL_BIN" c log --dir "$CONTRACT_DIR" >"$TMP_DIR/post-log.txt"

grep -q '"total_commits": 3' "$TMP_DIR/post-status.json"
grep -q '"model_state": "q1"' "$TMP_DIR/post-status.json"
grep -q "Model state: q1" "$TMP_DIR/post-status.txt"
grep -q '"message": "Signed update"' "$TMP_DIR/post-log.json"
grep -Eq '"signature_count": 1' "$TMP_DIR/post-log.json"
grep -q "$ALICE_ID" "$TMP_DIR/post-log.json"
grep -q "Message: Signed update" "$TMP_DIR/post-log.txt"
grep -q "Signatures: 1" "$TMP_DIR/post-log.txt"
grep -q "$ALICE_ID" "$TMP_DIR/post-log.txt"

if "$MODAL_BIN" c commit \
  --path /unsigned.text \
  --value "unsigned update" \
  --dir "$CONTRACT_DIR" \
  --output json \
  --message "Unsigned update" >"$TMP_DIR/unsigned-post.json" 2>"$TMP_DIR/unsigned-post.err"; then
  echo "expected unsigned post-bootstrap commit to fail" >&2
  exit 1
fi

grep -q 'current states {"q1"}' "$TMP_DIR/unsigned-post.err"
grep -q "missing +signed_by(/parties/alice.id)" "$TMP_DIR/unsigned-post.err"
grep -q "missing +signed_by(/parties/bob.id)" "$TMP_DIR/unsigned-post.err"

echo "first-contract CLI smoke passed"
