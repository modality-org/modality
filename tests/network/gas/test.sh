#!/usr/bin/env bash
# Gas on a network that prices it: every commit names a payer that signs,
# the payer's MOD covers the most the commit can cost, and the gas it used
# is paid to the sequencers that ordered it, refused commits included.
# Unnumbered.

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
test_init "gas"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_ID="12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_DIR="./tmp/node1"
UNIT=100000000
export MODALITY_HOME="$(pwd)/tmp/home"
mkdir -p "$MODALITY_HOME"

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
expect_log() {
    check "$2" test_wait_for_log "$SEQUENCER_LOG" "$1" "${3:-60}"
}
json() { python3 -c "import json,sys; s=sys.stdin.read(); d=json.loads(s[s.find('{'):] if s.lstrip()[:1] != '[' else s); print($1)"; }
held() {
    modal wallet balance --dir "$1" --output json 2>>"$CURRENT_LOG" \
        | python3 -c "import json,sys; d=json.load(sys.stdin); print(sum(h['balance'] for h in d['holdings'] if h['asset']=='MOD'))"
}
node_mod() {
    curl -s -m 20 http://127.0.0.1:18742/status.json | python3 -c "import json,sys; print(json.load(sys.stdin)['mod']['held'])"
}
# The status page is a snapshot refreshed every 10 seconds.
node_mod_after() {
    local was="$1" now
    for _ in $(seq 1 15); do
        now=$(node_mod)
        [ "$now" != "$was" ] && { echo "$now"; return; }
        sleep 2
    done
    echo "$was"
}

for who in foundation alice bob carol; do
    modal id create --path ./tmp/$who.mod_passfile >> "$CURRENT_LOG" 2>&1
done
ALICE_ID=$(python3 -c "import json; print(json.load(open('./tmp/alice.mod_passfile'))['id'])")

echo ""
echo "A MOD network that prices gas: 1 MOD unit per gas, 1000 MOD to Alice..."
cat > ./tmp/params.json <<EOF
{"quantity": $((21000000 * UNIT)), "decimals": 8, "block_subsidy": $((50 * UNIT)),
 "halving_interval": 0, "slow_start": 0, "cap": $((19000000 * UNIT)),
 "allocations": [{"to": "$ALICE_ID", "amount": $((1000 * UNIT))}]}
EOF
read -r MOD_ID _ < <(../../../scripts/mod-genesis/build.sh --out ./tmp/genesis --foundation ./tmp/foundation.mod_passfile --params ./tmp/params.json)
modal node create --dir "$NODE_DIR" --from-template devnet1/node1 >> "$CURRENT_LOG" 2>&1
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
python3 - "$NODE_DIR" ./tmp/genesis/genesis.json <<'PY'
import json, pathlib, sys
node, genesis = pathlib.Path(sys.argv[1]), json.loads(pathlib.Path(sys.argv[2]).read_text())
info = json.loads(pathlib.Path("../../../rust/modality-networks/networks/devnet1/info.json").read_text())
info.pop("emission", None)
info.update({"predicate_theory_version": "v2", "mod_contract": genesis,
             "gas_schedule": "v1", "gas_price": {"ordering": 1, "apply": 1}})
(node / "network.json").write_text(json.dumps(info, indent=2) + "\n")
config = json.loads((node / "config.json").read_text())
config["network_config_path"] = "./network.json"
config["status_port"] = 18742
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"
expect_log "MOD contract $MOD_ID: .* genesis commits applied" "The MOD genesis is applied" 20
check "The node starts with no MOD" test "$(node_mod)" = 0

echo ""
echo "Alice's first push, her wallet's genesis and the RECV, pays from what it receives..."
modal wallet create --key ./tmp/alice.mod_passfile --dir ./tmp/alice --remote "$REMOTE" >> "$CURRENT_LOG" 2>&1
RECV=$(modal wallet recv --dir ./tmp/alice --output json 2>>"$CURRENT_LOG")
echo "$RECV" >> "$CURRENT_LOG"
ALICE_RECV=$(echo "$RECV" | json 'd["received"][0]["commit_id"]')
expect_log "Sequenced commit $ALICE_RECV" "Her RECV of 1000 MOD is sequenced"
expect_log "Commit $ALICE_RECV charged [0-9]* MOD units to $ALICE_ID" "It is charged to her"
sleep 2
NODE_HELD=$(node_mod_after 0)
ALICE_HELD=$(held ./tmp/alice)
check "The node, the only sequencer, holds the fee" test "$NODE_HELD" -gt 0
check "Alice holds 1000 MOD less the fee" test "$ALICE_HELD" = "$((1000 * UNIT - NODE_HELD))"

echo ""
echo "Alice pays Bob; Bob's first push pays from what Alice sent..."
modal wallet create --key ./tmp/bob.mod_passfile --dir ./tmp/bob --remote "$REMOTE" >> "$CURRENT_LOG" 2>&1
BOB_ID=$(modal wallet address --dir ./tmp/bob 2>/dev/null)
SENT=$(modal wallet send --dir ./tmp/alice --to "$BOB_ID" --amount 1 --output json 2>>"$CURRENT_LOG")
echo "$SENT" >> "$CURRENT_LOG"
expect_log "Sequenced commit $(echo "$SENT" | json 'd["commit_id"]')" "Her SEND of 1 MOD is sequenced"
sleep 2
BOB_RECV=$(modal wallet recv --dir ./tmp/bob --output json 2>>"$CURRENT_LOG" | json 'd["received"][0]["commit_id"]')
expect_log "Sequenced commit $BOB_RECV" "Bob receives it"
sleep 2
BOB_HELD=$(held ./tmp/bob)
check "Bob holds 1 MOD less his fee" test "$BOB_HELD" -gt 0 -a "$BOB_HELD" -lt "$UNIT"
NODE_HELD=$(node_mod_after "$NODE_HELD")
check "No MOD is made or lost: Alice, Bob and the node hold the 1000" \
    test "$(( $(held ./tmp/alice) + BOB_HELD + NODE_HELD ))" = "$((1000 * UNIT))"

echo ""
echo "Commits that cannot pay are refused before they are ordered..."
modal wallet create --key ./tmp/carol.mod_passfile --dir ./tmp/carol --remote "$REMOTE" >> "$CURRENT_LOG" 2>&1
CAROL_ID=$(modal wallet address --dir ./tmp/carol 2>/dev/null)
modal contract commit --dir ./tmp/carol --path /notes/x.text --value hi \
    --sign ./tmp/carol.mod_passfile --payer "$CAROL_ID" --output json >> "$CURRENT_LOG" 2>&1
check "A payer with no MOD is refused at push" bash -c \
    "! modal contract push --dir ./tmp/carol --remote '$REMOTE' --output json > ./tmp/carol.out 2>&1"
check "The refusal says what the payer holds" grep -q "holds 0 MOD units" ./tmp/carol.out
modal contract create --dir ./tmp/plain >> "$CURRENT_LOG" 2>&1
check "A commit that names no payer is refused at push" bash -c \
    "! modal contract push --dir ./tmp/plain --remote '$REMOTE' --output json > ./tmp/plain.out 2>&1"
check "The refusal names the payer" grep -q "names no payer" ./tmp/plain.out

echo ""
echo "A commit refused when applied still pays for its work..."
BEFORE=$(held ./tmp/alice)
NODE_BEFORE=$NODE_HELD
modal contract commit --dir ./tmp/alice --method send --asset-contract "$MOD_ID" --asset-id MOD \
    --to-contract "$BOB_ID" --amount 9000000000000000 --sign ./tmp/alice.mod_passfile --payer "$ALICE_ID" \
    --output json >> "$CURRENT_LOG" 2>&1
TOO_MUCH=$(cat ./tmp/alice/.contract/HEAD)
modal contract push --dir ./tmp/alice --remote "$REMOTE" --output json >> "$CURRENT_LOG" 2>&1
expect_log "Failed to process sequenced commit $TOO_MUCH" "The overdrawn SEND is refused"
expect_log "Commit $TOO_MUCH charged [0-9]* MOD units to $ALICE_ID .*(refused)" "It is charged anyway"
sleep 2
NODE_AFTER=$(node_mod_after "$NODE_BEFORE")
check "Alice paid what the node earned" \
    test "$(( BEFORE - $(held ./tmp/alice) ))" = "$(( NODE_AFTER - NODE_BEFORE ))"
check "And paid something" test "$(held ./tmp/alice)" -lt "$BEFORE"

test_finalize
exit $?
