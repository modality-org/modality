#!/usr/bin/env bash
# Local commit → node → sequencer accept/reject under the same rules as local
# verify → pull back. Unnumbered: not part of the stable numbered suite.

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
test_init "sequenced-rule-reject"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
CONTRACT_DIR="./tmp/contract"
CLONE_DIR="./tmp/clone"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
mkdir -p "$PASSFILES" "$CONTRACT_DIR" "$CLONE_DIR"

echo ""
echo "Creating identities and a first-contract style model..."
ALICE_PASS="$PASSFILES/alice.mod_passfile"
BOB_PASS="$PASSFILES/bob.mod_passfile"
assert_success "modal id create --path $ALICE_PASS" "Should create Alice passfile"
assert_success "modal id create --path $BOB_PASS" "Should create Bob passfile"
ALICE_ID=$(modal id get --path "$ALICE_PASS")

assert_success "modal contract create --dir $CONTRACT_DIR --output json" \
  "Should create a local contract"
assert_success "modal checkout --dir $CONTRACT_DIR" "Should checkout working tree"
assert_success "modal set-named-id /parties/alice.id $ALICE_PASS --dir $CONTRACT_DIR" \
  "Should set Alice identity"
assert_success "modal set-named-id /parties/bob.id $BOB_PASS --dir $CONTRACT_DIR" \
  "Should set Bob identity"

mkdir -p "$CONTRACT_DIR/model"
cat > "$CONTRACT_DIR/model/default.modality" <<'EOF'
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
  q1 --> q1: +POST +signed_by(/parties/bob.id)
}
EOF

assert_success "modal add-rule --name authorized --dir $CONTRACT_DIR '[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)'" \
  "Should add authorized rule"

BOOTSTRAP_OUT=$(modal commit --all --dir "$CONTRACT_DIR" --output json --message "Bootstrap model")
echo "$BOOTSTRAP_OUT" > ./tmp/bootstrap.json
echo "$BOOTSTRAP_OUT" >> "$CURRENT_LOG"
BOOTSTRAP_HEAD=$(cat "$CONTRACT_DIR/.contract/HEAD")

echo ""
echo "Local verify must reject an unsigned follow-up..."
if modal commit --path /notes/unsigned.text --value nope --dir "$CONTRACT_DIR" --output json --message "Unsigned" \
  > ./tmp/local-unsigned.json 2> ./tmp/local-unsigned.err; then
    test_fail "Local commit should reject unsigned post after the model"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Local commit rejects unsigned post"
fi
grep -q "missing +signed_by(/parties/alice.id)" ./tmp/local-unsigned.err
assert_file_exists ./tmp/local-unsigned.err "Unsigned local rejection should write diagnostics"

echo ""
echo "Starting sequencer (run-validator)..."
assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create validator node"
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
NODE_PID=$(test_start_process "cd $NODE_DIR && modal node run-validator" "validator")
assert_success "test_wait_for_port 10101" "Validator should listen on 10101"
sleep 3

echo ""
echo "Pushing bootstrap commits..."
PUSH_BOOT=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_BOOT" >> "$CURRENT_LOG"
echo "$PUSH_BOOT" > ./tmp/push-bootstrap.json
TESTS_RUN=$((TESTS_RUN + 1))
if echo "$PUSH_BOOT" | grep -q '"status": "pushed"'; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Bootstrap push queued"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Bootstrap push queued"
    echo "$PUSH_BOOT"
fi

VALIDATOR_LOG="$LOG_DIR/${CURRENT_TEST}_validator.log"
expect_log() {
    local pattern="$1"
    local desc="$2"
    TESTS_RUN=$((TESTS_RUN + 1))
    if test_wait_for_log "$VALIDATOR_LOG" "$pattern" 40; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    if [ -f "$VALIDATOR_LOG" ]; then
        echo "Last 40 lines of validator log:" >> "$CURRENT_LOG"
        tail -40 "$VALIDATOR_LOG" >> "$CURRENT_LOG"
        tail -20 "$VALIDATOR_LOG"
    fi
    return 1
}

expect_log "Sequenced commit $BOOTSTRAP_HEAD" "Bootstrap commit should be sequenced" || true

echo ""
echo "Injecting an unsigned commit the CLI would refuse, then pushing it..."
UNSIGNED_ID=$(python3 - "$CONTRACT_DIR" <<'PY'
import hashlib, json, pathlib, sys
contract = pathlib.Path(sys.argv[1])
head = (contract / ".contract/HEAD").read_text().strip()
commit = {
    "body": [{"method": "post", "path": "/notes/unsigned.text", "value": "nope"}],
    "head": {"parent": head},
}
blob = json.dumps(commit, separators=(",", ":"), ensure_ascii=False)
commit_id = hashlib.sha256(blob.encode()).hexdigest()
(contract / ".contract/commits" / f"{commit_id}.json").write_text(json.dumps(commit, indent=2) + "\n")
(contract / ".contract/HEAD").write_text(commit_id + "\n")
print(commit_id)
PY
)
echo "Unsigned commit id: $UNSIGNED_ID" >> "$CURRENT_LOG"

PUSH_BAD=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_BAD" >> "$CURRENT_LOG"
echo "$PUSH_BAD" > ./tmp/push-unsigned.json

expect_log "Failed to process sequenced commit $UNSIGNED_ID" \
  "Sequencer should reject the unsigned commit" || true

echo ""
echo "Pull into a clone should not include the rejected commit..."
mkdir -p "$CLONE_DIR/.contract/commits"
cp "$CONTRACT_DIR/.contract/config.json" "$CLONE_DIR/.contract/config.json"
PULL1=$(modal contract pull --dir "$CLONE_DIR" --remote "$REMOTE" --remote-name origin --output json || true)
echo "$PULL1" >> "$CURRENT_LOG"
echo "$PULL1" > ./tmp/pull1.json
TESTS_RUN=$((TESTS_RUN + 1))
if [ ! -f "$CLONE_DIR/.contract/commits/${UNSIGNED_ID}.json" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Pull does not return the rejected unsigned commit"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Pull does not return the rejected unsigned commit"
fi

echo ""
echo "Resetting local HEAD and committing a signed follow-up..."
echo "$BOOTSTRAP_HEAD" > "$CONTRACT_DIR/.contract/HEAD"
SIGNED_OUT=$(modal commit --path /notes/signed.text --value yes --dir "$CONTRACT_DIR" --sign "$ALICE_PASS" --output json --message "Signed")
echo "$SIGNED_OUT" >> "$CURRENT_LOG"
echo "$SIGNED_OUT" > ./tmp/signed.json
SIGNED_ID=$(echo "$SIGNED_OUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('commit_id',''))" 2>/dev/null || true)
if [ -z "$SIGNED_ID" ]; then
    SIGNED_ID=$(cat "$CONTRACT_DIR/.contract/HEAD")
fi
echo "Signed commit id: $SIGNED_ID Alice: $ALICE_ID" >> "$CURRENT_LOG"

# Remote head may still point at the unsigned id; force a push of the signed child
# by clearing remote tracking so unpushed walks from signed HEAD.
rm -rf "$CONTRACT_DIR/.contract/refs/remotes/origin"
PUSH_OK=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_OK" >> "$CURRENT_LOG"
echo "$PUSH_OK" > ./tmp/push-signed.json

expect_log "Sequenced commit $SIGNED_ID" "Signed commit should be sequenced" || true

PULL2_DIR="./tmp/clone-after-signed"
mkdir -p "$PULL2_DIR/.contract/commits"
cp "$CONTRACT_DIR/.contract/config.json" "$PULL2_DIR/.contract/config.json"
PULL2=$(modal contract pull --dir "$PULL2_DIR" --remote "$REMOTE" --remote-name origin --output json || true)
echo "$PULL2" >> "$CURRENT_LOG"
echo "$PULL2" > ./tmp/pull2.json
TESTS_RUN=$((TESTS_RUN + 1))
if [ -f "$PULL2_DIR/.contract/commits/${SIGNED_ID}.json" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Pull returns the accepted signed commit"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Pull returns the accepted signed commit"
    echo "$PULL2"
fi

TESTS_RUN=$((TESTS_RUN + 1))
if [ ! -f "$PULL2_DIR/.contract/commits/${UNSIGNED_ID}.json" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Rejected unsigned commit still absent after later pull"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Rejected unsigned commit still absent after later pull"
fi

test_finalize
exit $?
