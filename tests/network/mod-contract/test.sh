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
echo "Only the emission program writes the MOD contract..."
TESTS_RUN=$((TESTS_RUN + 1))
if modal contract commit --theory v2 --dir "$MOD_DIR" --method invoke --path "$PROGRAM" \
    --value "{\"args\":{\"op\":\"mint\",\"blocks\":[{\"index\":1,\"to\":\"$MINER_ID\"}]}}" \
    --sign "$MALLORY" --output json > ./tmp/unmined.out 2> ./tmp/unmined.err; then
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Locally, a mint of a block with no proof of work is refused (was accepted)"
else
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Locally, a mint of a block with no proof of work is refused"
fi
check "The refusal names mined_headers" grep -q "missing +mined_headers" ./tmp/unmined.err
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

echo ""
echo "A mining node mints each finalized epoch on the MOD contract..."
test_cleanup
sleep 1
MINED="./tmp/genesis-mined"
python3 - ./tmp/params.json ./tmp/params-mined.json <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))
p["hash_func"] = "sha256"
json.dump(p, open(sys.argv[2], "w"))
PY
read -r MINED_ID _ < <(../../../scripts/mod-genesis/build.sh --out "$MINED" --foundation "$FOUNDATION" --params ./tmp/params-mined.json)
GENESIS_SAVED="$GENESIS"
GENESIS="$MINED"
network ./tmp/network-mined.json 'info["blocks_per_epoch"] = 3; info["initial_difficulty"] = 1'
GENESIS="$GENESIS_SAVED"
node_on ./tmp/network-mined.json
python3 - "$NODE_DIR" <<'PY'
import json, pathlib, sys
node = pathlib.Path(sys.argv[1])
config = json.loads((node / "config.json").read_text())
config["miner_hash_func"] = "sha256"
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
MINER_LOG="$LOG_DIR/${CURRENT_TEST}_miner.log"
test_start_process "cd $NODE_DIR && modal node run-miner" "miner" >/dev/null
check "Epoch 0 is minted once the tip is two epochs past it" \
    test_wait_for_log "$MINER_LOG" "MOD contract $MINED_ID: minted epoch 0" 240
check "Epoch 1 follows" test_wait_for_log "$MINER_LOG" "MOD contract $MINED_ID: minted epoch 1" 240
check "No node-local MOD is credited on a MOD network" \
    bash -c "! grep -q 'credited.*native MOD' '$MINER_LOG'"

set +e
REPLAY_JSON=$(modal contract replay --remote "$REMOTE" --contract-id "$MINED_ID" --output json 2>./tmp/replay-mined.err)
REPLAY_STATUS=$?
set -e
echo "$REPLAY_JSON" >> "$CURRENT_LOG"
cat ./tmp/replay-mined.err >> "$CURRENT_LOG" || true
check "A stranger replays the mints: each posted header is mined and linked" replay_ok

echo ""
echo "Only the network writes the MOD contract: a client's copy of a real mint is refused..."
COPY="./tmp/mined-copy"
modal contract pull --contract-id "$MINED_ID" --remote "$REMOTE" --dir "$COPY" >> "$CURRENT_LOG" 2>&1
NEXT=$(cat "$COPY/state/emission/next_index.num" 2>/dev/null || echo 0)
check "A client's pulled copy has the mints" test "$NEXT" -gt 1
test_cleanup
sleep 1
# The next unpaid block, as the node stored it, in the shape the node mints.
HEADER=$(modal node inspect --dir "$NODE_DIR" block _ "$NEXT" 2>/dev/null | python3 -c "
import hashlib, json, re, sys
f = dict(re.findall(r'^([A-Za-z ]+): (\S+)', sys.stdin.read(), re.M))
to, number = f['Nominated Peer'], int(f['Miner Number'])
print(json.dumps({'index': $NEXT, 'to': to, 'hash': f['Hash'], 'previous_hash': f['Previous Hash'],
    'timestamp': int(f['Timestamp']), 'data_hash': hashlib.sha256(f'{to}{number}'.encode()).hexdigest(),
    'difficulty': f['Target Difficulty'], 'nonce': f['Nonce'], 'miner_number': number}))
")
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_mined_sequencer.log"
test_start_process "cd $NODE_DIR && modal node run-sequencer" "mined_sequencer" >/dev/null
assert_success "test_wait_for_port 10101" "The node should listen on 10101 again"
PULLED_HEAD=$(cat "$COPY/.contract/HEAD")
modal contract commit --theory v2 --dir "$COPY" --method invoke --path "$PROGRAM" \
    --value "{\"args\":{\"op\":\"mint\",\"blocks\":[$HEADER]}}" \
    --sign "$MALLORY" --output json >> "$CURRENT_LOG" 2>&1 || true
MINT=$(cat "$COPY/.contract/HEAD")
check "Locally, a mint of the next mined block passes the rules" \
    bash -c "[ '$MINT' != '$PULLED_HEAD' ] && [ \"\$(cat '$COPY/state/emission/next_index.num')\" -eq $((NEXT + 1)) ]"
check "A mint pushed by a client is refused" bash -c "! modal contract push --dir '$COPY' --remote '$REMOTE' --remote-name origin --output json >> '$CURRENT_LOG' 2>&1"
check "The refusal names the MOD contract" grep -q "is the network's MOD contract; only the network writes it" "$CURRENT_LOG"
check "The mint is not sequenced" bash -c "! grep -q 'Sequenced commit $MINT' '$SEQUENCER_LOG'"
TESTS_RUN=$((TESTS_RUN + 1))
if modal contract commit --theory v2 --dir "$COPY" --method invoke --path "$PROGRAM" \
    --value "{\"args\":{\"op\":\"mint\",\"blocks\":[$HEADER]}}" \
    --sign "$MALLORY" --output json > ./tmp/again.out 2> ./tmp/again.err; then
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Locally, paying block $NEXT twice is refused (was accepted)"
else
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Locally, paying block $NEXT twice is refused"
fi
check "The refusal names the next block" grep -q "the next block to pay is $((NEXT + 1))" ./tmp/again.err

echo ""
echo "The miner takes its MOD: a contract on the nominee's key receives the mint's SEND..."
MINT0=$(grep -oE "MOD contract $MINED_ID: minted epoch 0 \([0-9]+ blocks\) in commit [0-9a-f]+" "$MINER_LOG" | awk '{print $NF}')
PAYOUT="./tmp/payout"
modal contract create --dir "$PAYOUT" --key "$NODE_DIR/node.modal_passfile" --output json >> "$CURRENT_LOG"
check "The payout contract's id is the nominee's" \
    test "$(modal contract id --dir "$PAYOUT")" = "$(python3 -c "import json; print(json.load(open('$NODE_DIR/node.modal_passfile'))['id'])")"
modal contract commit --dir "$PAYOUT" --method recv --send-commit-id "$MINT0" --send-index 0 \
    --asset-id MOD --asset-contract "$MINED_ID" --amount $((50 * UNIT)) --output json >> "$CURRENT_LOG"
PAYOUT_RECV=$(cat "$PAYOUT/.contract/HEAD")
push "$PAYOUT"
expect_log "Sequenced commit $PAYOUT_RECV" "The nominee receives block 1's 50 MOD from the epoch 0 mint"
cp -R "$PAYOUT" ./tmp/payout-lie
modal contract commit --dir ./tmp/payout-lie --method recv --send-commit-id "$MINT0" --send-index 1 \
    --asset-id MOD --asset-contract "$MINED_ID" --amount $((60 * UNIT)) --output json >> "$CURRENT_LOG"
PAYOUT_LIE=$(cat ./tmp/payout-lie/.contract/HEAD)
push ./tmp/payout-lie
expect_log "Failed to process sequenced commit $PAYOUT_LIE.*has amount $((50 * UNIT)), not $((60 * UNIT))" \
    "A RECV that overstates block 2's subsidy is refused, naming the amount"
check "Block 1's RECV was applied" bash -c "! grep -q 'Failed to process sequenced commit $PAYOUT_RECV' '$SEQUENCER_LOG'"

echo ""
echo "Vesting by MOD height: the payout locks its MOD until the chain is paid past a height..."
VIEW="./tmp/mod-view"
modal contract pull --contract-id "$MINED_ID" --remote "$REMOTE" --dir "$VIEW" >> "$CURRENT_LOG" 2>&1
HEIGHT=$(cat "$VIEW/state/emission/next_index.num")
HEIGHT_PATH="/reposts/$MINED_ID/emission/next_index.num"
# lock <dir> <height>: the owner's key, and the rules that hold MOD until <height>.
lock() {
    modal contract set-named-id /owner.id "$NODE_DIR/node.modal_passfile" --dir "$1" >> "$CURRENT_LOG" 2>&1
    cat > "$1/model/default.modality" <<EOF
model lockbox {
  part flow {
    q0 --> q1
    q1 --> q1: -SEND -post_to_path(/reposts)
    q1 --> q1: +SEND +signed_by(/owner.id) +num_gte($HEIGHT_PATH, "$2") -post_to_path(/reposts)
  }
}
EOF
    (cd "$1" \
        && modal c add-rule --name only_reposts 'always([+post_to_path(/reposts)] false)' \
        && modal c add-rule --name owner_sends 'always([+SEND -signed_by(/owner.id)] false)' \
        && modal c add-rule --name vests "always([+SEND -num_gte($HEIGHT_PATH, \"$2\")] false)" \
        && modal c commit --all --sign "../../$NODE_DIR/node.modal_passfile" -m "Lock until MOD height $2") \
        >> "$CURRENT_LOG" 2>&1
}
cp -R "$PAYOUT" ./tmp/payout-far
lock "$PAYOUT" "$HEIGHT"
LOCKED=$(cat "$PAYOUT/.contract/HEAD")
check "The lock commits" test "$LOCKED" != "$PAYOUT_RECV"
push "$PAYOUT"
expect_log "Sequenced commit $LOCKED" "The network takes the lock"
send_out() {
    modal contract commit --dir "$1" --method send --asset-contract "$MINED_ID" --asset-id MOD --to-contract "$ALICE_ID" \
        --amount $((50 * UNIT)) --sign "$NODE_DIR/node.modal_passfile" --output json > "$2.out" 2> "$2.err"
}
refused() {
    TESTS_RUN=$((TESTS_RUN + 1))
    if "${@:2}"; then
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $1 (was accepted)"
    else
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $1"
    fi
}
refused "Before any MOD height is reposted, the SEND is refused" send_out "$PAYOUT" ./tmp/vest-early
check "The refusal names the height" grep -q "missing +num_gte" ./tmp/vest-early.err
post_height() {
    modal contract commit --dir "$PAYOUT" --path "$HEIGHT_PATH" --value 1000000 \
        --sign "$NODE_DIR/node.modal_passfile" --output json > ./tmp/fake-height.out 2> ./tmp/fake-height.err
}
refused "A hand-written height is refused" post_height
check "That refusal names the POST under /reposts" grep -q "post_to_path(/reposts)" ./tmp/fake-height.err
modal contract repost "$MINED_ID" /emission/next_index.num --from-dir "$VIEW" --dir "$PAYOUT" >> "$CURRENT_LOG" 2>&1
modal contract commit --dir "$PAYOUT" --all --sign "$NODE_DIR/node.modal_passfile" --output json >> "$CURRENT_LOG" 2>&1
REPOSTED=$(cat "$PAYOUT/.contract/HEAD")
push "$PAYOUT"
expect_log "REPOST validated: $(modal contract id --dir "$PAYOUT")$HEIGHT_PATH <- $MINED_ID:/emission/next_index.num" \
    "The network checks the reposted height against the MOD contract"
expect_log "Sequenced commit $REPOSTED" "The REPOST of MOD height $HEIGHT is sequenced"
send_out "$PAYOUT" ./tmp/vest-now
VESTED=$(cat "$PAYOUT/.contract/HEAD")
check "At the height, the owner's SEND passes the rules" test "$VESTED" != "$REPOSTED"
push "$PAYOUT"
expect_log "Sequenced commit $VESTED" "The network sequences the vested SEND"
check "The vested SEND is applied" bash -c "! grep -q 'Failed to process sequenced commit $VESTED' '$SEQUENCER_LOG'"

lock ./tmp/payout-far $((HEIGHT + 1000))
modal contract repost "$MINED_ID" /emission/next_index.num --from-dir "$VIEW" --dir ./tmp/payout-far >> "$CURRENT_LOG" 2>&1
modal contract commit --dir ./tmp/payout-far --all --sign "$NODE_DIR/node.modal_passfile" --output json >> "$CURRENT_LOG" 2>&1
refused "Below a later height, the same SEND is refused" send_out ./tmp/payout-far ./tmp/vest-far
check "That refusal names the height too" grep -q "missing +num_gte" ./tmp/vest-far.err

test_finalize
exit $?
