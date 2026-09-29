#!/usr/bin/env bash
# Push admission: a sequencer with tight `push_limits` refuses a push that
# carries too many commits and a contract over its rate, and still takes
# pushes for other contracts. A local filter, not consensus. Unnumbered.

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
test_init "push-admission"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_DIR="./tmp/node1"

assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create sequencer node"
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
python3 - "$NODE_DIR" <<'PY'
import json, pathlib, sys
node = pathlib.Path(sys.argv[1])
config = json.loads((node / "config.json").read_text())
config["push_limits"] = {
    "max_commits": 3,
    "requests_per_peer_per_minute": 0,
    "commits_per_contract_per_minute": 4,
    "commits_per_node_per_minute": 0,
}
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
NODE_PID=$(test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer")
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"
sleep 3
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"

# A contract with its genesis and `n` more commits.
contract() {
    local dir="$1" n="$2"
    modal contract create --dir "$dir" >> "$CURRENT_LOG"
    for i in $(seq 1 "$n"); do
        modal contract commit --dir "$dir" --path "/notes/n$i.text" --value "n$i" >> "$CURRENT_LOG"
    done
}
push() {
    modal contract push --dir "$1" --remote "$REMOTE" --remote-name origin --output json \
        >> "$CURRENT_LOG" 2> ./tmp/push.err
}
refused() {
    local desc="$1" dir="$2" pattern="$3"
    TESTS_RUN=$((TESTS_RUN + 1))
    if push "$dir"; then
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $desc (was accepted)"
    elif grep -q "$pattern" ./tmp/push.err; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $desc (wrong reason)"
        cat ./tmp/push.err
    fi
    cat ./tmp/push.err >> "$CURRENT_LOG"
}

contract ./tmp/big 3
refused "A push of four commits is refused" ./tmp/big "4 commits in one request"

contract ./tmp/busy 2
assert_success "push ./tmp/busy" "Three commits fit"
modal contract commit --dir ./tmp/busy --path /notes/more.text --value more >> "$CURRENT_LOG"
assert_success "push ./tmp/busy" "A fourth commit fits the contract's minute"
modal contract commit --dir ./tmp/busy --path /notes/over.text --value over >> "$CURRENT_LOG"
refused "A fifth commit in the minute is refused" ./tmp/busy "commits a minute"

contract ./tmp/quiet 1
assert_success "push ./tmp/quiet" "Another contract still pushes"

TESTS_RUN=$((TESTS_RUN + 1))
if grep -q "Refused /contract/push" "$SEQUENCER_LOG"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} The sequencer logs each refusal"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} The sequencer logs each refusal"
fi

test_finalize
exit $?
