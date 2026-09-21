#!/usr/bin/env bash
# Deterministic invoke: emitted POSTs still face accumulated rules. Unnumbered.

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
test_init "invoke-rule-gate"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
CONTRACT_DIR="./tmp/contract"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
mkdir -p "$PASSFILES" "$CONTRACT_DIR" ./tmp

echo ""
echo "Compiling a fixture program that always POSTs /notes/from-program.text..."
WASM_OUT="$(pwd)/tmp/gate.wasm"
(cd ../../../rust && cargo run -q -p modality-wasm-runtime --example emit_fixed_post -- \
  /notes/from-program.text pwned "$WASM_OUT")
assert_file_exists ./tmp/gate.wasm "Fixture WASM should be written"

ALICE_PASS="$PASSFILES/alice.mod_passfile"
assert_success "modal id create --path $ALICE_PASS" "Should create Alice passfile"

CREATE_OUT=$(modal contract create --dir "$CONTRACT_DIR" --output json)
echo "$CREATE_OUT" >> "$CURRENT_LOG"
CONTRACT_ID=$(echo "$CREATE_OUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('contract_id',''))" 2>/dev/null || true)
if [ -z "$CONTRACT_ID" ]; then
    CONTRACT_ID=$(modal contract id --dir "$CONTRACT_DIR")
fi

assert_success "modal checkout --dir $CONTRACT_DIR" "Should checkout working tree"
assert_success "modal set-named-id /parties/alice.id $ALICE_PASS --dir $CONTRACT_DIR" \
  "Should set Alice identity"

mkdir -p "$CONTRACT_DIR/state/__programs__"
python3 - <<PY
import base64, pathlib
raw = pathlib.Path("./tmp/gate.wasm").read_bytes()
path = pathlib.Path("$CONTRACT_DIR/state/__programs__/gate.wasm")
path.write_text(base64.b64encode(raw).decode("ascii"))
PY

mkdir -p "$CONTRACT_DIR/model"
cat > "$CONTRACT_DIR/model/default.modality" <<'EOF'
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
}
EOF

assert_success "modal add-rule --name authorized --dir $CONTRACT_DIR '[] always([-signed_by(/parties/alice.id)] false)'" \
  "Should add authorized rule"

assert_success "modal commit --all --dir $CONTRACT_DIR --output json --message Bootstrap" \
  "Should commit bootstrap"

BOOTSTRAP_HEAD=$(cat "$CONTRACT_DIR/.contract/HEAD")

assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create validator node"
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
NODE_PID=$(test_start_process "cd $NODE_DIR && modal node run-validator" "validator")
assert_success "test_wait_for_port 10101" "Validator should listen on 10101"
sleep 3

PUSH_BOOT=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_BOOT" >> "$CURRENT_LOG"
VALIDATOR_LOG="$LOG_DIR/${CURRENT_TEST}_validator.log"

expect_log() {
    local pattern="$1"
    local desc="$2"
    TESTS_RUN=$((TESTS_RUN + 1))
    if test_wait_for_log "$VALIDATOR_LOG" "$pattern" 90; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    return 1
}

expect_log "Sequenced commit $BOOTSTRAP_HEAD" "Program upload should be sequenced" || true

echo ""
echo "Unsigned invoke that would emit POST must be rejected..."
if modal contract commit --dir "$CONTRACT_DIR" --method invoke \
    --path "/__programs__/gate.wasm" --value '{"args":{}}' --output json \
    > ./tmp/local-unsigned.json 2> ./tmp/local-unsigned.err; then
    test_fail "Local commit should reject unsigned invoke after the model"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Local commit rejects unsigned invoke"
fi

UNSIGNED_ID=$(python3 - "$CONTRACT_DIR" <<'PY'
import hashlib, json, pathlib, sys
contract = pathlib.Path(sys.argv[1])
head = (contract / ".contract/HEAD").read_text().strip()
commit = {
    "body": [{
        "method": "invoke",
        "path": "/__programs__/gate.wasm",
        "value": {"args": {}}
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
expect_log "Failed to process sequenced commit $UNSIGNED_ID" \
  "Sequencer should reject the unsigned invoke" || true

echo "$BOOTSTRAP_HEAD" > "$CONTRACT_DIR/.contract/HEAD"
rm -rf "$CONTRACT_DIR/.contract/refs/remotes/origin"

SIGNED_OUT=$(modal contract commit --dir "$CONTRACT_DIR" --method invoke \
    --path "/__programs__/gate.wasm" --value '{"args":{}}' \
    --sign "$ALICE_PASS" --output json --message "Signed invoke")
echo "$SIGNED_OUT" >> "$CURRENT_LOG"
SIGNED_ID=$(echo "$SIGNED_OUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('commit_id',''))" 2>/dev/null || true)
if [ -z "$SIGNED_ID" ]; then
    SIGNED_ID=$(cat "$CONTRACT_DIR/.contract/HEAD")
fi
PUSH_OK=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_OK" >> "$CURRENT_LOG"
expect_log "Sequenced commit $SIGNED_ID" "Signed invoke should be sequenced" || true

set +e
REPLAY_JSON=$(modal contract replay --remote "$REMOTE" --contract-id "$CONTRACT_ID" --through "$SIGNED_ID" --save ./tmp/prefix.json --output json 2>./tmp/replay.err)
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
assert d.get('wasm_modules',0) >= 1
assert d.get('invokes_expanded',0) >= 1
"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Stranger replay re-executes the program and re-checks rules"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Stranger replay re-executes the program and re-checks rules"
    echo "$REPLAY_JSON"
fi

test_finalize
exit $?
