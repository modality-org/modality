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
"$MODAL_BIN" c commit \
  --all \
  --dir "$CONTRACT_DIR" \
  --sign "$ALICE_PASSFILE" \
  --output json \
  --message "Initial contract setup" >/dev/null

"$MODAL_BIN" c status --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/status.json"
"$MODAL_BIN" c log --dir "$CONTRACT_DIR" --output json >"$TMP_DIR/log.json"

grep -q "$ALICE_ID" "$CONTRACT_DIR/state/parties/alice.id"
grep -q "$BOB_ID" "$CONTRACT_DIR/state/parties/bob.id"
grep -q '"total_commits": 2' "$TMP_DIR/status.json"
grep -q '"commits":' "$TMP_DIR/log.json"
grep -q '"signature_count": 1' "$TMP_DIR/log.json"
grep -q "$ALICE_ID" "$TMP_DIR/log.json"

echo "first-contract CLI smoke passed"
