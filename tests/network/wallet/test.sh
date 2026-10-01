#!/usr/bin/env bash
# A wallet is a contract at its owner's key that only that key can extend.
# A bank sends to a wallet before it exists; the wallet receives, sends on,
# and refuses a commit its key did not sign. Unnumbered.

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
test_init "wallet"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_DIR="./tmp/node1"
BANK="./tmp/bank"
ALICE="./tmp/alice"
# Wallets default to $MODALITY_HOME/.modality/wallet; keep them in tmp.
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
    if "$@" >> "$CURRENT_LOG" 2>&1; then
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $desc (was accepted)"
    else
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
    fi
}
json() { python3 -c "import json,sys; d=json.load(sys.stdin); print($1)"; }
expect_log() {
    check "$2" test_wait_for_log "$SEQUENCER_LOG" "$1" "${3:-60}"
}

echo ""
echo "Starting a sequencer on devnet1 under predicate theory v2..."
modal node create --dir "$NODE_DIR" --from-template devnet1/node1 >> "$CURRENT_LOG" 2>&1
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
python3 - "$NODE_DIR" <<'PY'
import json, pathlib, sys
node = pathlib.Path(sys.argv[1])
info = json.loads(pathlib.Path("../../../rust/modality-networks/networks/devnet1/info.json").read_text())
info["predicate_theory_version"] = "v2"
(node / "network.json").write_text(json.dumps(info, indent=2) + "\n")
config = json.loads((node / "config.json").read_text())
config["network_config_path"] = "./network.json"
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"

echo ""
echo "A bank sends to an address that has no wallet yet..."
assert_success "modal id create --path ./tmp/alice.mod_passfile" "Should create Alice's key"
assert_success "modal id create --path ./tmp/mallory.mod_passfile" "Should create Mallory's key"
ALICE_ID=$(python3 -c "import json; print(json.load(open('./tmp/alice.mod_passfile'))['id'])")
modal contract create --dir "$BANK" >> "$CURRENT_LOG"
BANK_ID=$(modal contract id --dir "$BANK")
modal contract commit --dir "$BANK" --method create --asset-id tok --quantity 100000 --divisibility 1 --decimals 2 \
    --output json >> "$CURRENT_LOG"
modal contract commit --dir "$BANK" --method send --asset-id tok --to-contract "$ALICE_ID" --amount 250 \
    --output json >> "$CURRENT_LOG"
BANK_SEND=$(cat "$BANK/.contract/HEAD")
modal contract push --dir "$BANK" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG" 2>&1
expect_log "Sequenced commit $BANK_SEND" "The bank's SEND of 2.5 tok to Alice's address is sequenced"

echo ""
echo "Alice makes her wallet at her key and receives..."
CREATED=$(modal wallet create --key ./tmp/alice.mod_passfile --dir "$ALICE" --remote "$REMOTE" --output json 2>>"$CURRENT_LOG")
echo "$CREATED" >> "$CURRENT_LOG"
check "The wallet's address is her key's id" test "$(echo "$CREATED" | json 'd["address"]')" = "$ALICE_ID"
check "It is a new wallet" test "$(echo "$CREATED" | json 'd["status"]')" = "created"
check "modal wallet address prints it" test "$(modal wallet address --dir "$ALICE" 2>/dev/null)" = "$ALICE_ID"
check "The genesis names her key as the one signer" grep -q "$ALICE_ID" "$ALICE/state/signers/1.id"
INCOMING=$(modal wallet incoming --dir "$ALICE" --output json 2>>"$CURRENT_LOG")
check "The send waits for her" test "$(echo "$INCOMING" | json 'len(d)')" = 1
check "It names the SEND and its amount" \
    test "$(echo "$INCOMING" | json 'd[0]["send_commit_id"] + " " + str(d[0]["amount"])')" = "$BANK_SEND 250"
RECV=$(modal wallet recv --dir "$ALICE" --output json 2>>"$CURRENT_LOG")
echo "$RECV" >> "$CURRENT_LOG"
ALICE_RECV=$(echo "$RECV" | json 'd["received"][0]["commit_id"]')
check "recv pushes the genesis and the RECV" test "$(echo "$RECV" | json 'len(d["pushed"])')" = 2
expect_log "Sequenced commit $ALICE_RECV" "The network sequences her RECV"
check "The RECV is applied" bash -c "! grep -q 'Failed to process sequenced commit $ALICE_RECV' '$SEQUENCER_LOG'"
sleep 2
BALANCE=$(modal wallet balance --dir "$ALICE" --output json 2>>"$CURRENT_LOG")
echo "$BALANCE" >> "$CURRENT_LOG"
check "She holds 2.5 tok" test "$(echo "$BALANCE" | json 'd["holdings"][0]["amount"]')" = "2.5"
check "Nothing waits any more" test "$(echo "$BALANCE" | json 'd["incoming"]')" = 0
AGAIN=$(modal wallet recv --dir "$ALICE" --output json 2>>"$CURRENT_LOG")
check "A second recv receives nothing" test "$(echo "$AGAIN" | json 'len(d["received"])')" = 0

echo ""
echo "Only Alice's key extends her wallet..."
refused "A commit signed by Mallory is refused" \
    modal contract commit --dir "$ALICE" --path /notes/x.text --value hi --sign ./tmp/mallory.mod_passfile
refused "An unsigned commit is refused" modal contract commit --dir "$ALICE" --path /notes/x.text --value hi

echo ""
echo "Bob's wallet in the default place, and Alice pays him..."
BOB_OUT=$(modal wallet create --remote "$REMOTE" --output json 2>>"$CURRENT_LOG")
echo "$BOB_OUT" >> "$CURRENT_LOG"
BOB_ID=$(echo "$BOB_OUT" | json 'd["address"]')
check "With no --dir, the wallet is under MODALITY_HOME" test -d "$MODALITY_HOME/.modality/wallet/.contract"
check "With no --key, a new key is made in the wallet" test -f "$MODALITY_HOME/.modality/wallet/owner.mod_passfile"
refused "Sending more than she holds is refused" \
    modal wallet send --dir "$ALICE" --to "$BOB_ID" --amount 3 --asset tok --asset-contract "$BANK_ID"
refused "An amount finer than the asset is refused" \
    modal wallet send --dir "$ALICE" --to "$BOB_ID" --amount 0.001 --asset tok --asset-contract "$BANK_ID"
SENT=$(modal wallet send --dir "$ALICE" --to "$BOB_ID" --amount 1 --asset tok --asset-contract "$BANK_ID" \
    --output json 2>>"$CURRENT_LOG")
echo "$SENT" >> "$CURRENT_LOG"
ALICE_SEND=$(echo "$SENT" | json 'd["commit_id"]')
check "She sends 1 tok, 100 in smallest units" test "$(echo "$SENT" | json 'd["amount"]')" = 100
expect_log "Sequenced commit $ALICE_SEND" "The network sequences her SEND"
sleep 2
BOB_RECV=$(modal wallet recv --output json 2>>"$CURRENT_LOG")
echo "$BOB_RECV" >> "$CURRENT_LOG"
BOB_RECV_ID=$(echo "$BOB_RECV" | json 'd["received"][0]["commit_id"]')
expect_log "Sequenced commit $BOB_RECV_ID" "Bob receives it"
sleep 2
check "Bob holds 1 tok" test "$(modal wallet balance --output json 2>>"$CURRENT_LOG" | json 'd["holdings"][0]["amount"]')" = "1"
check "Alice holds 1.5 tok" test "$(modal wallet balance --dir "$ALICE" --output json 2>>"$CURRENT_LOG" | json 'd["holdings"][0]["amount"]')" = "1.5"

echo ""
echo "Alice's key on another machine finds her wallet, not a second one..."
COPY=$(modal wallet create --key ./tmp/alice.mod_passfile --dir ./tmp/alice-copy --remote "$REMOTE" --output json 2>>"$CURRENT_LOG" | python3 -c "import sys; s=sys.stdin.read(); print(s[s.rfind('{\n  \"address\"'):])")
echo "$COPY" >> "$CURRENT_LOG"
check "create copies the wallet from the network" test "$(echo "$COPY" | json 'd["status"]')" = "copied from the network"
check "The copy has her balance" test "$(modal wallet balance --dir ./tmp/alice-copy --output json 2>>"$CURRENT_LOG" | json 'd["holdings"][0]["amount"]')" = "1.5"
check "The copy's head is the network's" test "$(cat ./tmp/alice-copy/.contract/HEAD)" = "$ALICE_SEND"

test_finalize
exit $?
