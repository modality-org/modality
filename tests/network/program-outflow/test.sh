#!/usr/bin/env bash
# Outflow only through a posted program: a SEND the program did not emit is
# refused, whoever signs it. Unnumbered.

set -e
cd "$(dirname "$0")"

source ../test-lib.sh

if ! command -v rebuild &>/dev/null; then
    rebuild() { (cd ../../../rust && cargo build --package modal); }
fi
command -v modal &> /dev/null || rebuild
if [ -d ../../../rust/target/debug ]; then
    export PATH="$(cd ../../../rust/target/debug && pwd):$PATH"
fi

rm -rf ./tmp
test_init "program-outflow"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
CONTRACT_DIR="./tmp/treasury"
DEST_DIR="./tmp/dest"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
mkdir -p "$PASSFILES" "$CONTRACT_DIR" "$DEST_DIR" ./tmp

OWNER_PASS="$PASSFILES/owner.mod_passfile"
assert_success "modal id create --path $OWNER_PASS" "Should create the owner's passfile"

assert_success "modal contract create --dir $DEST_DIR" "Should create a destination contract"
DEST_ID=$(modal contract id --dir "$DEST_DIR")

echo ""
echo "Compiling a payout program that always SENDs 5 and then 2 drops to the destination..."
WASM_OUT="$(pwd)/tmp/payout.wasm"
(cd ../../../rust && cargo run -q -p modality-wasm-runtime --example emit_fixed_actions -- \
  "[{\"method\":\"send\",\"path\":null,\"value\":{\"asset_id\":\"drops\",\"to_contract\":\"$DEST_ID\",\"amount\":5}},{\"method\":\"send\",\"path\":null,\"value\":{\"asset_id\":\"drops\",\"to_contract\":\"$DEST_ID\",\"amount\":2}}]" \
  "$WASM_OUT")
assert_file_exists ./tmp/payout.wasm "Payout program should be written"
PAYOUT_SHA=$(python3 -c "import hashlib; print(hashlib.sha256(open('./tmp/payout.wasm','rb').read()).hexdigest())")

CREATE_OUT=$(modal contract create --dir "$CONTRACT_DIR" --output json)
echo "$CREATE_OUT" >> "$CURRENT_LOG"
CONTRACT_ID=$(modal contract id --dir "$CONTRACT_DIR")
assert_success "modal checkout --dir $CONTRACT_DIR" "Should checkout working tree"
assert_success "modal set-named-id /owner.id $OWNER_PASS --dir $CONTRACT_DIR" \
  "Should set the owner's identity"

mkdir -p "$CONTRACT_DIR/state/__programs__"
python3 - <<PY
import base64, pathlib
raw = pathlib.Path("./tmp/payout.wasm").read_bytes()
pathlib.Path("$CONTRACT_DIR/state/__programs__/payout.wasm").write_text(base64.b64encode(raw).decode("ascii"))
PY

# The owner mints once. After that only the payout program's output moves
# assets, and the program does not change.
mkdir -p "$CONTRACT_DIR/model"
cat > "$CONTRACT_DIR/model/default.modality" <<EOF
model Treasury {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +CREATE -SEND -modifies(/__programs__) +signed_by(/owner.id)
  q2 --> q2: +POST -SEND -CREATE -modifies(/__programs__)
  q2 --> q2: +SEND -CREATE -modifies(/__programs__) +emitted_by(/__programs__/payout.wasm, "$PAYOUT_SHA")
}
EOF

assert_success "modal add-rule --name program_only --dir $CONTRACT_DIR 'always([+SEND -emitted_by(/__programs__/payout.wasm, \"$PAYOUT_SHA\")] false)'" \
  "Should add the program-only outflow rule"
assert_success "modal add-rule --name program_fixed --dir $CONTRACT_DIR 'always([+modifies(/__programs__)] false)'" \
  "Should add the fixed-program rule"
assert_success "modal commit --all --dir $CONTRACT_DIR --output json --message Bootstrap" \
  "Should commit the bootstrap"
assert_success "modal contract commit --dir $CONTRACT_DIR --method create --asset-id drops --quantity 1000 --divisibility 1 --sign $OWNER_PASS --output json" \
  "The owner mints the pool"
MINT_HEAD=$(cat "$CONTRACT_DIR/.contract/HEAD")

assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create sequencer node"
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
NODE_PID=$(test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer")
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"
sleep 3

SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
expect_log() {
    local pattern="$1"
    local desc="$2"
    TESTS_RUN=$((TESTS_RUN + 1))
    if test_wait_for_log "$SEQUENCER_LOG" "$pattern" 90; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    return 1
}

PUSH_BOOT=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_BOOT" >> "$CURRENT_LOG"
expect_log "Sequenced commit $MINT_HEAD" "Bootstrap and mint should be sequenced" || true

echo ""
echo "The owner's hand-written SEND must be refused..."
if modal contract commit --dir "$CONTRACT_DIR" --method send --asset-id drops \
    --to-contract "$DEST_ID" --amount 500 --sign "$OWNER_PASS" --output json \
    > ./tmp/local-send.json 2> ./tmp/local-send.err; then
    test_fail "Local commit should refuse a SEND the program did not emit"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Local commit refuses the owner's SEND"
fi

HAND_ID=$(python3 - "$CONTRACT_DIR" "$DEST_ID" <<'PY'
import hashlib, json, pathlib, sys
contract = pathlib.Path(sys.argv[1])
head = (contract / ".contract/HEAD").read_text().strip()
commit = {
    "body": [{
        "method": "send",
        "path": None,
        "value": {"asset_id": "drops", "to_contract": sys.argv[2], "amount": 500}
    }],
    "head": {"parent": head},
}
blob = json.dumps(commit, separators=(",", ":"), ensure_ascii=False)
commit_id = hashlib.sha256(blob.encode()).hexdigest()
(contract / ".contract/commits" / f"{commit_id}.json").write_text(json.dumps(commit, indent=2) + "\n")
(contract / ".contract/HEAD").write_text(commit_id + "\n")
print(commit_id)
PY
)
PUSH_BAD=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_BAD" >> "$CURRENT_LOG"
expect_log "Failed to process sequenced commit $HAND_ID" \
  "Sequencer should refuse the hand-written SEND" || true

echo "$MINT_HEAD" > "$CONTRACT_DIR/.contract/HEAD"
rm -rf "$CONTRACT_DIR/.contract/refs/remotes/origin"

echo ""
echo "The payout program's SEND is accepted..."
PAY_OUT=$(modal contract commit --dir "$CONTRACT_DIR" --method invoke \
    --path "/__programs__/payout.wasm" --value '{"args":{}}' \
    --sign "$OWNER_PASS" --output json --message "Payout")
echo "$PAY_OUT" >> "$CURRENT_LOG"
PAY_ID=$(cat "$CONTRACT_DIR/.contract/HEAD")
PUSH_OK=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_OK" >> "$CURRENT_LOG"
expect_log "Sequenced commit $PAY_ID" "The program's payout should be sequenced" || true

set +e
REPLAY_JSON=$(modal contract replay --remote "$REMOTE" --contract-id "$CONTRACT_ID" --through "$PAY_ID" --output json 2>./tmp/replay.err)
REPLAY_STATUS=$?
set -e
echo "$REPLAY_JSON" >> "$CURRENT_LOG"
cat ./tmp/replay.err >> "$CURRENT_LOG" || true
TESTS_RUN=$((TESTS_RUN + 1))
if [ "$REPLAY_STATUS" -eq 0 ] && echo "$REPLAY_JSON" | python3 -c "
import json,sys
s=sys.stdin.read()
d=json.loads(s[s.find('{'):])
assert d.get('ok') is True
assert d.get('invokes_expanded',0) >= 1
"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Stranger replay re-runs the program and accepts its SEND"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Stranger replay re-runs the program and accepts its SEND"
    echo "$REPLAY_JSON"
fi

echo ""
echo "The destination receives each of the program's SENDs, once..."
push_dest() {
    modal contract push --dir "$DEST_DIR" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG"
}
assert_success "modal contract commit --dir $DEST_DIR --method recv --send-commit-id $PAY_ID --output json" \
  "The destination signs a RECV of the first emitted SEND"
RECV_FIRST=$(cat "$DEST_DIR/.contract/HEAD")
push_dest
expect_log "Sequenced commit $RECV_FIRST" "The RECV of an emitted SEND should be sequenced" || true
assert_success "modal contract commit --dir $DEST_DIR --method recv --send-commit-id $PAY_ID --send-index 1 --output json" \
  "The destination signs a RECV of the second emitted SEND"
RECV_SECOND=$(cat "$DEST_DIR/.contract/HEAD")
push_dest
expect_log "Sequenced commit $RECV_SECOND" "The RECV of the second SEND should be sequenced" || true
assert_success "modal contract commit --dir $DEST_DIR --method recv --send-commit-id $PAY_ID --send-index 1 --output json" \
  "The destination signs a second RECV of the second SEND"
RECV_AGAIN=$(cat "$DEST_DIR/.contract/HEAD")
push_dest
expect_log "Failed to process sequenced commit $RECV_AGAIN" "Each SEND is received once" || true

test_finalize
exit $?
