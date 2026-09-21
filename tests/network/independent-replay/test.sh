#!/usr/bin/env bash
# Stranger fetches sequenced prefix + re-checks locally. Unnumbered.

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
test_init "independent-replay"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
CONTRACT_DIR="./tmp/contract"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
mkdir -p "$PASSFILES" "$CONTRACT_DIR"

echo ""
echo "Creating a modeled contract and sequencing a signed commit..."
ALICE_PASS="$PASSFILES/alice.mod_passfile"
assert_success "modal id create --path $ALICE_PASS" "Should create Alice passfile"
ALICE_ID=$(modal id get --path "$ALICE_PASS")

CREATE_OUT=$(modal contract create --dir "$CONTRACT_DIR" --output json)
echo "$CREATE_OUT" >> "$CURRENT_LOG"
CONTRACT_ID=$(echo "$CREATE_OUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('contract_id',''))" 2>/dev/null || true)
if [ -z "$CONTRACT_ID" ]; then
    CONTRACT_ID=$(modal contract id --dir "$CONTRACT_DIR")
fi

assert_success "modal checkout --dir $CONTRACT_DIR" "Should checkout working tree"
assert_success "modal set-named-id /parties/alice.id $ALICE_PASS --dir $CONTRACT_DIR" \
  "Should set Alice identity"

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
    if test_wait_for_log "$VALIDATOR_LOG" "$pattern" 40; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    return 1
}

expect_log "Sequenced commit $BOOTSTRAP_HEAD" "Bootstrap commit should be sequenced" || true

SIGNED_OUT=$(modal commit --path /notes/signed.text --value yes --dir "$CONTRACT_DIR" --sign "$ALICE_PASS" --output json --message Signed)
echo "$SIGNED_OUT" >> "$CURRENT_LOG"
SIGNED_ID=$(echo "$SIGNED_OUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('commit_id',''))" 2>/dev/null || true)
if [ -z "$SIGNED_ID" ]; then
    SIGNED_ID=$(cat "$CONTRACT_DIR/.contract/HEAD")
fi
PUSH_OK=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_OK" >> "$CURRENT_LOG"
expect_log "Sequenced commit $SIGNED_ID" "Signed commit should be sequenced" || true

echo ""
echo "Stranger replays from the node with no local contract copy..."
set +e
REPLAY_JSON=$(modal contract replay --remote "$REMOTE" --contract-id "$CONTRACT_ID" --through "$SIGNED_ID" --save ./tmp/prefix.json --output json 2>./tmp/replay.err)
REPLAY_STATUS=$?
set -e
echo "$REPLAY_JSON" >> "$CURRENT_LOG"
cat ./tmp/replay.err >> "$CURRENT_LOG" || true
echo "$REPLAY_JSON" > ./tmp/replay.json
TESTS_RUN=$((TESTS_RUN + 1))
if [ "$REPLAY_STATUS" -eq 0 ] && echo "$REPLAY_JSON" | python3 -c "
import json,sys
s=sys.stdin.read()
d=json.loads(s[s.find('{'):])
assert d.get('ok') is True
assert d.get('commits_checked',0) >= 2
"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Remote replay re-checks the sequenced prefix"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Remote replay re-checks the sequenced prefix"
    echo "$REPLAY_JSON"
fi

assert_file_exists ./tmp/prefix.json "Replay should save an artifact"

OFFLINE=$(modal contract replay --artifact ./tmp/prefix.json --output json 2>./tmp/offline.err)
echo "$OFFLINE" >> "$CURRENT_LOG"
TESTS_RUN=$((TESTS_RUN + 1))
if echo "$OFFLINE" | python3 -c "
import json,sys
s=sys.stdin.read()
assert json.loads(s[s.find('{'):]).get('ok') is True
"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Offline artifact replay passes"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Offline artifact replay passes"
    echo "$OFFLINE"
fi

python3 - <<'PY'
import json
from pathlib import Path
p = Path("./tmp/prefix.json")
art = json.loads(p.read_text())
art["prefix_digest"] = "00" * 32
p.write_text(json.dumps(art))
PY

if modal contract replay --artifact ./tmp/prefix.json --output json > ./tmp/tampered.json 2> ./tmp/tampered.err; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Tampered artifact should fail replay"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Tampered artifact should fail replay"
fi

test_finalize
exit $?
