#!/usr/bin/env bash
# The faucet `scripts/testnet/faucet/create.sh` makes, on a network under the
# testnet's predicate theory (v3): anyone funds it with a RECV; `modal wallet
# faucet` registers a new wallet's key, drips once and receives; a second
# claim by the same key gets nothing; a claim the faucet cannot cover is
# refused. The asset is a bank's, held like MOD. Unnumbered.

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
test_init "faucet"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_DIR="./tmp/node1"
BANK="./tmp/bank"
FAUCET="./tmp/faucet"
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
refused() {
    local desc="$1"
    shift
    TESTS_RUN=$((TESTS_RUN + 1))
    if "$@" > ./tmp/refused.out 2>&1; then
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $desc (was accepted)"
    else
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
    fi
    cat ./tmp/refused.out >> "$CURRENT_LOG"
}
json() { python3 -c "import json,sys; s=sys.stdin.read(); d=json.loads(s[s.find('{'):]); print($1)"; }
expect_log() {
    check "$2" test_wait_for_log "$SEQUENCER_LOG" "$1" "${3:-120}"
}
balance_of() {
    modal wallet balance --dir "$1" --output json 2>>"$CURRENT_LOG" \
        | json 'next((h["amount"] for h in d["holdings"]), "0")'
}
wait_balance() {
    local dir="$1" want="$2"
    for _ in $(seq 1 150); do
        [ "$(balance_of "$dir")" = "$want" ] && return 0
        sleep 2
    done
    return 1
}

echo ""
echo "Starting a sequencer on devnet1 under predicate theory v3..."
modal node create --dir "$NODE_DIR" --from-template devnet1/node1 >> "$CURRENT_LOG" 2>&1
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
python3 - "$NODE_DIR" <<'PY'
import json, pathlib, sys
node = pathlib.Path(sys.argv[1])
info = json.loads(pathlib.Path("../../../rust/modality-networks/networks/devnet1/info.json").read_text())
info["predicate_theory_version"] = "v3"
(node / "network.json").write_text(json.dumps(info, indent=2) + "\n")
config = json.loads((node / "config.json").read_text())
config["network_config_path"] = "./network.json"
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"

echo ""
echo "A faucet for a bank's tok, 10 tok a drip, funded with two drips..."
modal contract create --dir "$BANK" >> "$CURRENT_LOG"
BANK_ID=$(modal contract id --dir "$BANK")
assert_success "../../../scripts/testnet/faucet/create.sh $FAUCET $BANK_ID:tok 1000" "create.sh makes the faucet"
FAUCET_ID=$(modal contract id --dir "$FAUCET")
check "Its first commit is signed by no one" \
    python3 -c "import json,sys; c=json.load(open(sys.argv[1])); sys.exit(1 if c['head'].get('signatures') else 0)" \
    "$FAUCET/.contract/commits/$(cat "$FAUCET/.contract/HEAD").json"
modal contract push --dir "$FAUCET" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG" 2>&1
expect_log "Sequenced commit $(cat "$FAUCET/.contract/HEAD")" "The faucet is sequenced"

modal contract commit --dir "$BANK" --method create --asset-id tok --quantity 100000 --divisibility 1 --decimals 2 \
    --output json >> "$CURRENT_LOG"
modal contract commit --dir "$BANK" --method send --asset-id tok --to-contract "$FAUCET_ID" --amount 2000 \
    --output json >> "$CURRENT_LOG"
BANK_SEND=$(cat "$BANK/.contract/HEAD")
modal contract push --dir "$BANK" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG" 2>&1
expect_log "Sequenced commit $BANK_SEND" "The bank's SEND of 20 tok to the faucet is sequenced"

refused "A funding commit that also posts is refused" \
    bash -c "modal contract set --dir $FAUCET /notes/x.text hi > /dev/null && modal commit --all --dir $FAUCET \
        --method recv --send-commit-id $BANK_SEND --asset-contract $BANK_ID --asset-id tok --amount 2000 --output json"
rm -f "$FAUCET/state/notes/x.text"
assert_success "modal commit --all --dir $FAUCET --method recv --send-commit-id $BANK_SEND --asset-contract $BANK_ID --asset-id tok --amount 2000 --output json" \
    "Anyone funds the faucet with an unsigned RECV"
FUND=$(cat "$FAUCET/.contract/HEAD")
modal contract push --dir "$FAUCET" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG" 2>&1
expect_log "Sequenced commit $FUND" "The funding RECV is sequenced" 300
check "The RECV is applied" bash -c "! grep 'Failed to process sequenced commit $FUND' '$SEQUENCER_LOG' | grep -vq 'missing prefix_cert'"

echo ""
echo "Alice claims with a new wallet..."
modal wallet create --dir ./tmp/alice --remote "$REMOTE" --output json >> "$CURRENT_LOG" 2>&1
CLAIM=$(modal wallet faucet --dir ./tmp/alice --faucet "$FAUCET_ID" --output json 2>>"$CURRENT_LOG")
echo "$CLAIM" >> "$CURRENT_LOG"
DRIP=$(echo "$CLAIM" | json 'd["drip_commit_id"]')
check "The faucet sends her 10 tok" test "$(echo "$CLAIM" | json 'd["drip"]["amount"]')" = 1000
check "She receives the drip" test "$(echo "$CLAIM" | json 'd["received"][0]["send_commit_id"]')" = "$DRIP"
check "Her wallet holds 10 tok" wait_balance ./tmp/alice 10

echo ""
echo "A key drips once..."
AGAIN=$(modal wallet faucet --dir ./tmp/alice --faucet "$FAUCET_ID" --output json 2>&1 || true)
echo "$AGAIN" >> "$CURRENT_LOG"
check "A second claim by her key is refused" grep -q 'has had its drip' <<< "$AGAIN"
ALICE_KEY=$(python3 -c "import json; print(json.load(open('./tmp/alice/.contract/wallet.json'))['key'])")
modal pull --contract-id "$FAUCET_ID" --remote "$REMOTE" --dir ./tmp/by-hand --output json >> "$CURRENT_LOG" 2>&1
refused "Her key cannot sign a second drip by hand" \
    modal commit --all --dir ./tmp/by-hand --method send --asset-contract "$BANK_ID" --asset-id tok \
        --to-contract "$FAUCET_ID" --amount 1000 --sign "$ALICE_KEY" --output json
check "The refusal is the spent drip" grep -q 'bool_true(/claimants/[a-z0-9]*/claimed.bool) matched' ./tmp/refused.out

echo ""
echo "Bob claims the second drip; Carol finds the faucet empty..."
modal wallet create --dir ./tmp/bob --remote "$REMOTE" --output json >> "$CURRENT_LOG" 2>&1
BOB=$(modal wallet faucet --dir ./tmp/bob --faucet "$FAUCET_ID" --output json 2>>"$CURRENT_LOG")
echo "$BOB" >> "$CURRENT_LOG"
check "Bob's claim builds on Alice's" test "$(echo "$BOB" | json 'd["drip"]["amount"]')" = 1000
check "Bob's wallet holds 10 tok" wait_balance ./tmp/bob 10
modal wallet create --dir ./tmp/carol --remote "$REMOTE" --output json >> "$CURRENT_LOG" 2>&1
EMPTY=$(modal wallet faucet --dir ./tmp/carol --faucet "$FAUCET_ID" 2>&1 || true)
echo "$EMPTY" >> "$CURRENT_LOG"
check "Carol is told the faucet needs funding" grep -q 'needs funding' <<< "$EMPTY"

test_finalize
exit $?
