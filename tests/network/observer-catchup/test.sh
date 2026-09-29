#!/usr/bin/env bash
# An observer that joins after contracts are sequenced pulls their history
# from the sequencer, then follows new commits, and serves the same commits
# and balances over its explorer API. Unnumbered.

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
test_init "observer-catchup"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
SOURCE="./tmp/source"
WALLET="./tmp/wallet"
NODE_DIR="./tmp/node1"
OBSERVER_DIR="./tmp/observer"
OBSERVER_STATUS_PORT=18111
mkdir -p ./tmp

assert_success "modal contract create --dir $SOURCE" "Should create the source contract"
assert_success "modal contract create --dir $WALLET" "Should create the wallet contract"
SOURCE_ID=$(modal contract id --dir "$SOURCE")
WALLET_ID=$(modal contract id --dir "$WALLET")

assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create sequencer node"
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"
sleep 3

SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
OBSERVER_LOG="$LOG_DIR/${CURRENT_TEST}_observer.log"
expect_in_log() {
    local log="$1"
    local pattern="$2"
    local desc="$3"
    TESTS_RUN=$((TESTS_RUN + 1))
    if test_wait_for_log "$log" "$pattern" 120; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    return 1
}
push() {
    modal contract push --dir "$1" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG"
}

echo ""
echo "Before the observer exists: mint, send, receive..."
assert_success "modal contract commit --dir $SOURCE --method create --asset-id drops --quantity 100 --divisibility 1 --output json" \
  "The source mints 100 drops"
assert_success "modal contract commit --dir $SOURCE --method send --asset-id drops --to-contract $WALLET_ID --amount 10 --output json" \
  "The source sends 10 to the wallet"
SEND_ID=$(cat "$SOURCE/.contract/HEAD")
push "$SOURCE"
expect_in_log "$SEQUENCER_LOG" "Sequenced commit $SEND_ID" "The SEND is sequenced" || true
assert_success "modal contract commit --dir $WALLET --method recv --send-commit-id $SEND_ID --output json" \
  "The wallet receives it"
RECV_ID=$(cat "$WALLET/.contract/HEAD")
push "$WALLET"
expect_in_log "$SEQUENCER_LOG" "Sequenced commit $RECV_ID" "The RECV is sequenced" || true

echo ""
echo "An observer joins and catches up..."
assert_success "modal node create --dir $OBSERVER_DIR --bootstrappers $REMOTE" \
  "Should create the observer node"
python3 - "$OBSERVER_DIR/config.json" "$OBSERVER_STATUS_PORT" <<'PY'
import json, sys
path, port = sys.argv[1], int(sys.argv[2])
cfg = json.load(open(path))
cfg["listeners"] = ["/ip4/0.0.0.0/tcp/10111/ws"]
cfg["status_port"] = port
json.dump(cfg, open(path, "w"), indent=2)
PY
test_start_process "cd $OBSERVER_DIR && modal node run-observer" "observer" >/dev/null
assert_success "test_wait_for_port $OBSERVER_STATUS_PORT" "Observer status server should listen"
expect_in_log "$OBSERVER_LOG" "Contract catch-up applied" "The observer pulls the sequenced history" || true

held() {
    # held <contract> <asset contract>: the balance the observer serves
    curl -s -m 5 "http://127.0.0.1:$OBSERVER_STATUS_PORT/api/contracts/$1" | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    print('none'); sys.exit()
for b in d.get('balances') or []:
    if b['asset_contract'] == '$2' and b['asset_id'] == 'drops':
        print(b['balance']); break
else:
    print('none')
"
}
expect_balance() {
    local contract="$1"
    local want="$2"
    local desc="$3"
    local got=""
    TESTS_RUN=$((TESTS_RUN + 1))
    for _ in $(seq 1 60); do
        got=$(held "$contract" "$SOURCE_ID")
        [ "$got" = "$want" ] && break
        sleep 2
    done
    if [ "$got" = "$want" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $desc (observer serves: $got)"
    fi
}
expect_balance "$SOURCE_ID" 90 "The observer serves the source's 90 drops"
expect_balance "$WALLET_ID" 10 "The observer serves the wallet's 10 drops"

TESTS_RUN=$((TESTS_RUN + 1))
if curl -s -m 5 "http://127.0.0.1:$OBSERVER_STATUS_PORT/api/contracts/$WALLET_ID/commits" | python3 -c "
import json, sys
commits = json.load(sys.stdin)
assert any(c['commit_id'] == '$RECV_ID' and c['sequenced'] for c in commits), commits
"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} The observer lists the RECV as sequenced"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} The observer lists the RECV as sequenced"
fi

echo ""
echo "After it joined: a new SEND reaches the observer..."
assert_success "modal contract commit --dir $SOURCE --method send --asset-id drops --to-contract $WALLET_ID --amount 5 --output json" \
  "The source sends 5 more"
SECOND_SEND=$(cat "$SOURCE/.contract/HEAD")
push "$SOURCE"
expect_in_log "$SEQUENCER_LOG" "Sequenced commit $SECOND_SEND" "The second SEND is sequenced" || true
expect_balance "$SOURCE_ID" 85 "The observer follows it to 85 drops"

test_finalize
exit $?
