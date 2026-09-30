#!/usr/bin/env bash
# The network's MOD contract: its genesis creates MOD, posts the emission
# parameters, sends the genesis allocations, and locks itself so that only
# the emission program's output is accepted. Every node applies the genesis
# at startup and takes its emission from it. Unnumbered.

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
test_init "mod-contract"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
GENESIS="./tmp/genesis"
ALICE="./tmp/alice-wallet"
BOB="./tmp/bob-wallet"
MINER="./tmp/miner-wallet"
mkdir -p "$PASSFILES"

check() {
    local desc="$1"
    shift
    TESTS_RUN=$((TESTS_RUN + 1))
    if "$@"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $desc"
    fi
}

for who in foundation mallory; do
    assert_success "modal id create --path $PASSFILES/$who.mod_passfile" "Should create $who's passfile"
done
FOUNDATION="$PASSFILES/foundation.mod_passfile"
MALLORY="$PASSFILES/mallory.mod_passfile"
for dir in "$ALICE" "$BOB" "$MINER"; do
    assert_success "modal contract create --dir $dir" "Should create $(basename "$dir")"
done
ALICE_ID=$(modal contract id --dir "$ALICE")
BOB_ID=$(modal contract id --dir "$BOB")
MINER_ID=$(modal contract id --dir "$MINER")

echo ""
echo "Building the MOD genesis: 21M MOD at 10^8, 50 MOD a block halving every 4 blocks..."
UNIT=100000000
cat > ./tmp/params.json <<EOF
{
  "quantity": $((21000000 * UNIT)),
  "divisibility": $UNIT,
  "block_subsidy": $((50 * UNIT)),
  "halving_interval": 4,
  "slow_start": 0,
  "cap": $((19000000 * UNIT)),
  "allocations": [
    {"to": "$ALICE_ID", "amount": $((1000000 * UNIT))},
    {"to": "$BOB_ID", "amount": $((500000 * UNIT))}
  ]
}
EOF
read -r MOD_ID MOD_SHA < <(../../../scripts/mod-genesis/build.sh --out "$GENESIS" --foundation "$FOUNDATION" --params ./tmp/params.json)
assert_file_exists "$GENESIS/genesis.json" "The genesis should be written"
MOD_DIR="$GENESIS/contract"
PROGRAM="/__programs__/emission.wasm"
read -r ALICE_SEND BOB_SEND < <(python3 - "$GENESIS/genesis.json" <<'PY'
import json, sys
g = json.load(open(sys.argv[1]))
print(*[c["commit_id"] for c in g["commits"] if any(a["method"] == "send" for a in c["body"])])
PY
)

# network <file> <python edit of info>: devnet1 on theory v2, with the genesis.
network() {
    python3 - "$1" "$GENESIS/genesis.json" "$2" <<'PY'
import json, pathlib, sys
out, genesis, edit = pathlib.Path(sys.argv[1]), json.loads(pathlib.Path(sys.argv[2]).read_text()), sys.argv[3]
info = json.loads(pathlib.Path("../../../rust/modality-networks/networks/devnet1/info.json").read_text())
info["predicate_theory_version"] = "v2"
info.pop("emission", None)
info["mod_contract"] = genesis
exec(edit)
out.write_text(json.dumps(info, indent=2) + "\n")
PY
}
node_on() {
    rm -rf "$NODE_DIR"
    modal node create --dir "$NODE_DIR" --from-template devnet1/node1 >> "$CURRENT_LOG" 2>&1
    modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
    cp "$1" "$NODE_DIR/network.json"
    python3 - "$NODE_DIR" <<'PY'
import json, pathlib, sys
node = pathlib.Path(sys.argv[1])
config = json.loads((node / "config.json").read_text())
config["network_config_path"] = "./network.json"
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
}
# refused <name> <network edit> <log pattern> <desc>: a node on that network
# does not start, and says why.
refused() {
    network "./tmp/$1.json" "$2"
    node_on "./tmp/$1.json"
    test_start_process "cd $NODE_DIR && modal node run-sequencer" "$1" >/dev/null
    check "$4" test_wait_for_log "$LOG_DIR/${CURRENT_TEST}_$1.log" "$3" 60
    test_cleanup
    sleep 1
}

echo ""
echo "A node refuses a MOD genesis it cannot trust..."
refused tampered 'info["mod_contract"]["commits"][1]["body"][0]["value"] = "1"' \
    "but its body and head hash to" "A genesis commit edited after it was named is refused"
refused theory_v0 'info["predicate_theory_version"] = "v0"' \
    "needs predicate theory v2" "A MOD contract on a network without signature checks is refused"
refused both 'info["emission"] = {"block_subsidy": 50}' \
    "names both mod_contract and emission" "A network that also names emission is refused"

echo ""
echo "Starting a sequencer on the MOD network..."
network ./tmp/network.json ''
node_on ./tmp/network.json
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"
check "The node applies the 6 genesis commits through the contract processor" \
    test_wait_for_log "$SEQUENCER_LOG" "MOD contract $MOD_ID: 6 of 6 genesis commits applied; $((21000000 * UNIT)) MOD at divisibility $UNIT" 10
check "The network's emission is the contract's posts" \
    test_wait_for_log "$SEQUENCER_LOG" "Network emission from the MOD contract: {\"block_subsidy\":$((50 * UNIT)),\"halving_interval_blocks\":4,\"slow_start_blocks\":0,\"cap\":$((19000000 * UNIT))}" 10

push() {
    modal contract push --dir "$1" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG" 2>&1
}
expect_log() {
    check "$2" test_wait_for_log "$SEQUENCER_LOG" "$1" "${3:-60}"
}

echo ""
echo "The genesis allocations are ordinary SENDs..."
modal contract commit --dir "$ALICE" --method recv --send-commit-id "$ALICE_SEND" \
    --asset-id MOD --asset-contract "$MOD_ID" --amount $((1000000 * UNIT)) --output json >> "$CURRENT_LOG"
ALICE_RECV=$(cat "$ALICE/.contract/HEAD")
push "$ALICE"
expect_log "Sequenced commit $ALICE_RECV" "Alice's wallet receives its 1,000,000 MOD, stating the amount"
modal contract commit --dir "$BOB" --method recv --send-commit-id "$BOB_SEND" \
    --asset-id MOD --asset-contract "$MOD_ID" --amount $((1000000 * UNIT)) --output json >> "$CURRENT_LOG"
LIE=$(cat "$BOB/.contract/HEAD")
push "$BOB"
expect_log "Failed to process sequenced commit $LIE.*has amount $((500000 * UNIT)), not $((1000000 * UNIT))" \
    "A RECV that overstates Bob's allocation is refused, naming the amount"

echo ""
echo "Only the network writes the MOD contract..."
modal contract commit --theory v2 --dir "$MOD_DIR" --method invoke --path "$PROGRAM" \
    --value "{\"args\":{\"op\":\"mint\",\"blocks\":[{\"index\":1,\"to\":\"$MINER_ID\"}]}}" \
    --sign "$MALLORY" --output json >> "$CURRENT_LOG"
MINT=$(cat "$MOD_DIR/.contract/HEAD")
check "Locally, the program's mint of block 1 passes the rules" test -n "$MINT"
check "A mint pushed by a client is refused" bash -c "! modal contract push --dir '$MOD_DIR' --remote '$REMOTE' --remote-name origin --output json >> '$CURRENT_LOG' 2>&1"
check "The refusal names the MOD contract" grep -q "is the network's MOD contract; only the network writes it" "$CURRENT_LOG"
check "The mint is not sequenced" bash -c "! grep -q 'Sequenced commit $MINT' '$SEQUENCER_LOG'"
TESTS_RUN=$((TESTS_RUN + 1))
if modal contract commit --theory v2 --dir "$MOD_DIR" --method send --asset-id MOD --to-contract "$MINER_ID" \
    --amount 5 --sign "$FOUNDATION" --output json > ./tmp/theft.out 2> ./tmp/theft.err; then
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Locally, a hand-written SEND by the foundation's key is refused (was accepted)"
else
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Locally, a hand-written SEND by the foundation's key is refused"
fi
check "The refusal names emitted_by" grep -q "missing +emitted_by" ./tmp/theft.err
TESTS_RUN=$((TESTS_RUN + 1))
if modal contract commit --theory v2 --dir "$MOD_DIR" --method invoke --path "$PROGRAM" \
    --value "{\"args\":{\"op\":\"mint\",\"blocks\":[{\"index\":1,\"to\":\"$MINER_ID\"}]}}" \
    --sign "$MALLORY" --output json > ./tmp/again.out 2> ./tmp/again.err; then
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Locally, paying block 1 twice is refused (was accepted)"
else
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Locally, paying block 1 twice is refused"
fi
check "The refusal names the next block" grep -q "the next block to pay is 2" ./tmp/again.err

echo ""
echo "A stranger replays the MOD contract from the sequencer..."
set +e
REPLAY_JSON=$(modal contract replay --remote "$REMOTE" --contract-id "$MOD_ID" --output json 2>./tmp/replay.err)
REPLAY_STATUS=$?
set -e
echo "$REPLAY_JSON" >> "$CURRENT_LOG"
cat ./tmp/replay.err >> "$CURRENT_LOG" || true
replay_ok() {
    [ "$REPLAY_STATUS" -eq 0 ] && echo "$REPLAY_JSON" | python3 -c "
import json, sys
s = sys.stdin.read()
assert json.loads(s[s.find('{'):]).get('ok') is True
"
}
check "The replay accepts the genesis" replay_ok

echo ""
echo "Restarting the sequencer: the genesis is not applied twice..."
test_cleanup
sleep 1
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer2.log"
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer2" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101 again"
check "After restart, no genesis commit is applied again" \
    test_wait_for_log "$SEQUENCER_LOG" "MOD contract $MOD_ID: 0 of 6 genesis commits applied" 10
check "After restart, the emission is still the contract's" \
    test_wait_for_log "$SEQUENCER_LOG" "Network emission from the MOD contract: {\"block_subsidy\":$((50 * UNIT))" 10

test_finalize
exit $?
