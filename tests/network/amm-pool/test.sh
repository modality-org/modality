#!/usr/bin/env bash
# A constant-product pool as a contract: two token contracts mint, an LP adds
# liquidity, a trader swaps, and the LP removes. Only the pool program's output
# moves the pool's assets; a RECV that misstates its SEND, a payout to someone
# who did not pay in, and a hand-written payout are refused. Unnumbered.

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
test_init "amm-pool"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
POOL="./tmp/pool"
KA="./tmp/token-a"
KB="./tmp/token-b"
LP_WALLET="./tmp/lp-wallet"
TRADER_WALLET="./tmp/trader-wallet"
MALLORY_WALLET="./tmp/mallory-wallet"
mkdir -p "$PASSFILES"

for who in founder lp trader mallory; do
    assert_success "modal id create --path $PASSFILES/$who.mod_passfile" "Should create $who's passfile"
done
FOUNDER="$PASSFILES/founder.mod_passfile"
LP_KEY="$PASSFILES/lp.mod_passfile"
TRADER="$PASSFILES/trader.mod_passfile"
MALLORY="$PASSFILES/mallory.mod_passfile"

for dir in "$KA" "$KB" "$LP_WALLET" "$TRADER_WALLET" "$MALLORY_WALLET" "$POOL"; do
    assert_success "modal contract create --dir $dir" "Should create $(basename "$dir")"
done
KA_ID=$(modal contract id --dir "$KA")
KB_ID=$(modal contract id --dir "$KB")
LP_ID=$(modal contract id --dir "$LP_WALLET")
TRADER_ID=$(modal contract id --dir "$TRADER_WALLET")
MALLORY_ID=$(modal contract id --dir "$MALLORY_WALLET")
POOL_ID=$(modal contract id --dir "$POOL")
TOK_A="$KA_ID:tokA"
TOK_B="$KB_ID:tokB"

echo ""
echo "Building the pool program..."
read -r WASM POOL_SHA < <(../../../examples/programs/constant-product-pool/build.sh)
assert_file_exists "$WASM" "The pool program should be built"
PROGRAM="/__programs__/pool.wasm"

echo ""
echo "Setting up the pool..."
assert_success "modal checkout --dir $POOL" "Should checkout the pool"
assert_success "modal set-named-id /founder.id $FOUNDER --dir $POOL" "Should post the founder's key"
assert_success "modal contract set --dir $POOL /config/token_a.text $TOK_A" "Should name asset A"
assert_success "modal contract set --dir $POOL /config/token_b.text $TOK_B" "Should name asset B"
assert_success "modal contract set --dir $POOL /config/fee.num 0.003" "Should post the fee"
for path in /reserves/a.num /reserves/b.num /lp/supply.num; do
    assert_success "modal contract set --dir $POOL $path 0" "Should start $path at 0"
done
mkdir -p "$POOL/state/__programs__"
python3 - "$WASM" "$POOL/state/__programs__/pool.wasm" <<'PY'
import base64, pathlib, sys
pathlib.Path(sys.argv[2]).write_text(base64.b64encode(pathlib.Path(sys.argv[1]).read_bytes()).decode("ascii"))
PY

# Every operating edge restates the invariants: each one is a named
# predicate the rules below require on every commit.
KEEPS="+tracks(/reserves/a.num, \"$TOK_A\") +tracks(/reserves/b.num, \"$TOK_B\") +tracks(/lp/supply.num, \"lp\", \"issued\") +keeps_product_per_share(/reserves/a.num, /reserves/b.num, /lp/supply.num) +pays_senders(\"$TOK_A\") +pays_senders(\"$TOK_B\") +pays_senders(\"lp\") -CREATE -modifies(/config) -modifies(/__programs__) -modifies(/rules) -modifies(/model)"
EMITTED="+emitted_by($PROGRAM, \"$POOL_SHA\")"
mkdir -p "$POOL/model"
cat > "$POOL/model/default.modality" <<EOF
model Pool {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +CREATE -SEND -RECV -modifies(/__programs__) +signed_by(/founder.id)
  q2 --> q3: +modifies(/rules) -SEND -RECV -CREATE -modifies(/__programs__) +signed_by(/founder.id)
  q3 --> q3: -SEND -RECV -modifies(/reserves) -modifies(/lp) $KEEPS
  q3 --> q3: $EMITTED -modifies(/lp) +keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num) $KEEPS
  q3 --> q3: $EMITTED +modifies(/lp) $KEEPS
}
EOF
assert_success "modal commit --theory v2 --all --dir $POOL --sign $FOUNDER --output json --message Bootstrap" \
  "Should commit the bootstrap"
assert_success "modal contract commit --theory v2 --dir $POOL --method create --asset-id lp --quantity 1000000000000 --divisibility 1 --sign $FOUNDER --output json" \
  "The pool creates its share asset"

add_rule() {
    assert_success "modal add-rule --name $1 --dir $POOL '$2'" "Should add: $3"
}
add_rule program_sends "always([+SEND -emitted_by($PROGRAM, \"$POOL_SHA\")] false)" "only the program SENDs"
add_rule program_recvs "always([+RECV -emitted_by($PROGRAM, \"$POOL_SHA\")] false)" "only the program RECVs"
add_rule tracks_a "always([-tracks(/reserves/a.num, \"$TOK_A\")] false)" "reserve A is what A came in less what went out"
add_rule tracks_b "always([-tracks(/reserves/b.num, \"$TOK_B\")] false)" "reserve B likewise"
add_rule tracks_lp "always([-tracks(/lp/supply.num, \"lp\", \"issued\")] false)" "the share supply is what went out less what came back"
add_rule pays_a "always([-pays_senders(\"$TOK_A\")] false)" "A goes only to someone who paid in"
add_rule pays_b "always([-pays_senders(\"$TOK_B\")] false)" "B likewise"
add_rule pays_lp "always([-pays_senders(\"lp\")] false)" "shares likewise"
add_rule curve "always([+modifies(/reserves) -modifies(/lp) -keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)] false)" "a swap keeps the fee-adjusted product"
add_rule per_share "always([-keeps_product_per_share(/reserves/a.num, /reserves/b.num, /lp/supply.num)] false)" "no commit lowers the product per share"
add_rule config_fixed "always([+modifies(/config)] false)" "the config never changes"
add_rule program_fixed "always([+modifies(/__programs__)] false)" "the program never changes"
add_rule no_mint "always([+CREATE] false)" "no more shares are created"
add_rule rules_fixed "always([+modifies(/rules)] false)" "no rule is added later"
add_rule model_fixed "always([+modifies(/model)] false)" "the model never changes"
assert_success "modal commit --theory v2 --all --dir $POOL --sign $FOUNDER --output json --message Rules" \
  "Should commit the rules"
POOL_SETUP=$(cat "$POOL/.contract/HEAD")

echo ""
echo "Minting the two tokens..."
assert_success "modal contract commit --dir $KA --method create --asset-id tokA --quantity 1000000 --divisibility 1 --output json" "KA mints tokA"
assert_success "modal contract commit --dir $KA --method send --asset-id tokA --to-contract $LP_ID --amount 10000 --output json" "KA sends tokA to the LP"
KA_TO_LP=$(cat "$KA/.contract/HEAD")
assert_success "modal contract commit --dir $KA --method send --asset-id tokA --to-contract $TRADER_ID --amount 1000 --output json" "KA sends tokA to the trader"
KA_TO_TRADER=$(cat "$KA/.contract/HEAD")
assert_success "modal contract commit --dir $KB --method create --asset-id tokB --quantity 1000000 --divisibility 1 --output json" "KB mints tokB"
assert_success "modal contract commit --dir $KB --method send --asset-id tokB --to-contract $LP_ID --amount 40000 --output json" "KB sends tokB to the LP"
KB_TO_LP=$(cat "$KB/.contract/HEAD")

assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create sequencer node"
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
NODE_PID=$(test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer")
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"
sleep 3

SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
expect_log() {
    local pattern="$1"
    local desc="$2"
    local timeout="${3:-180}"
    TESTS_RUN=$((TESTS_RUN + 1))
    if test_wait_for_log "$SEQUENCER_LOG" "$pattern" "$timeout"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    tail -20 "$SEQUENCER_LOG" || true
    return 1
}
push() {
    modal contract push --dir "$1" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG"
}
# commit <dir> <description> <modal contract commit args...>: commit, push,
# and wait for the sequencer; prints nothing, leaves the id in $LAST.
commit() {
    local dir="$1"
    local desc="$2"
    shift 2
    assert_success "modal contract commit --dir $dir $* --output json" "$desc"
    LAST=$(cat "$dir/.contract/HEAD")
    push "$dir"
    expect_log "Sequenced commit $LAST" "$desc: sequenced" || true
}
# claim <send_commit_id> <from> <asset_contract or -> <asset_id> <amount> <memo JSON>
claim() {
    python3 - "$@" <<'PY'
import json, sys
commit, sender, creator, asset, amount, memo = sys.argv[1:]
c = {"send_commit_id": commit, "from_contract": sender, "asset_id": asset, "amount": int(amount), "memo": json.loads(memo)}
if creator != "-":
    c["asset_contract"] = creator
print(json.dumps(c))
PY
}
# invoke <op> <claims...>: the invoke value for an operation.
invoke_value() {
    local op="$1"
    shift
    python3 -c "import json,sys; print(json.dumps({'args': {'op': sys.argv[1], 'sends': [json.loads(a) for a in sys.argv[2:]]}}))" "$op" "$@"
}
invoke() {
    local desc="$1"
    local signer="$2"
    local value="$3"
    assert_success "modal contract commit --theory v2 --dir $POOL --method invoke --path $PROGRAM --value '$value' --sign $signer --output json" "$desc"
    LAST=$(cat "$POOL/.contract/HEAD")
    push "$POOL"
}
reset_pool() {
    echo "$1" > "$POOL/.contract/HEAD"
    rm -f "$POOL/.contract/commits/$2.json"
    rm -rf "$POOL/.contract/refs/remotes/origin"
}

expect_log "Predicate theory: v2" "Sequencer reports predicate theory v2" 10 || true
push "$POOL"
expect_log "Sequenced commit $POOL_SETUP" "The pool's bootstrap, share asset and rules should be sequenced" || true
push "$KA"
expect_log "Sequenced commit $KA_TO_TRADER" "KA's mint and sends should be sequenced" || true
push "$KB"
expect_log "Sequenced commit $KB_TO_LP" "KB's mint and send should be sequenced" || true

commit "$LP_WALLET" "The LP receives tokA" --method recv --send-commit-id "$KA_TO_LP"
commit "$LP_WALLET" "The LP receives tokB" --method recv --send-commit-id "$KB_TO_LP"
commit "$TRADER_WALLET" "The trader receives tokA" --method recv --send-commit-id "$KA_TO_TRADER"

echo ""
echo "The LP adds 1000 A and 4000 B..."
ADD='{"op":"add"}'
commit "$LP_WALLET" "The LP sends 1000 A to the pool" --method send --asset-contract "$KA_ID" --asset-id tokA --to-contract "$POOL_ID" --amount 1000 --memo "'$ADD'"
LP_SEND_A=$LAST
commit "$LP_WALLET" "The LP sends 4000 B to the pool" --method send --asset-contract "$KB_ID" --asset-id tokB --to-contract "$POOL_ID" --amount 4000 --memo "'$ADD'"
LP_SEND_B=$LAST
invoke "The LP invokes add" "$LP_KEY" "$(invoke_value add "$(claim "$LP_SEND_A" "$LP_ID" "$KA_ID" tokA 1000 "$ADD")" "$(claim "$LP_SEND_B" "$LP_ID" "$KB_ID" tokB 4000 "$ADD")")"
ADD_ID=$LAST
expect_log "Sequenced commit $ADD_ID" "The add should be sequenced" || true
commit "$LP_WALLET" "The LP receives sqrt(1000 * 4000) = 2000 shares" --method recv --send-commit-id "$ADD_ID" --asset-contract "$POOL_ID" --asset-id lp --amount 2000

echo ""
echo "The trader swaps 100 A for B, at least 300..."
SWAP='{"op":"swap","min_out":300}'
commit "$TRADER_WALLET" "The trader sends 100 A with min_out 300" --method send --asset-contract "$KA_ID" --asset-id tokA --to-contract "$POOL_ID" --amount 100 --memo "'$SWAP'"
SWAP_SEND=$LAST
BEFORE_SWAP=$(cat "$POOL/.contract/HEAD")
invoke "Mallory invokes the swap, claiming the SEND was 1000" "$MALLORY" "$(invoke_value swap "$(claim "$SWAP_SEND" "$TRADER_ID" "$KA_ID" tokA 1000 "$SWAP")")"
LIE_ID=$LAST
expect_log "Failed to process sequenced commit $LIE_ID.*has amount 100, not 1000" "The sequencer refuses a RECV that overstates its SEND, naming the amount" || true
reset_pool "$BEFORE_SWAP" "$LIE_ID"
invoke "Mallory invokes the swap for the trader, honestly" "$MALLORY" "$(invoke_value swap "$(claim "$SWAP_SEND" "$TRADER_ID" "$KA_ID" tokA 100 "$SWAP")")"
SWAP_ID=$LAST
expect_log "Sequenced commit $SWAP_ID" "The swap should be sequenced" || true
commit "$TRADER_WALLET" "The trader receives 4000 * 99.7 / 1099.7 = 362 B" --method recv --send-commit-id "$SWAP_ID" --asset-contract "$KB_ID" --asset-id tokB --amount 362

echo ""
echo "Payouts the program did not compute are refused..."
BEFORE_THEFT=$(cat "$POOL/.contract/HEAD")
TESTS_RUN=$((TESTS_RUN + 1))
if modal contract commit --theory v2 --dir "$POOL" --method send --asset-contract "$KB_ID" --asset-id tokB \
    --to-contract "$MALLORY_ID" --amount 1000 --sign "$MALLORY" --output json > ./tmp/theft.out 2> ./tmp/theft.err; then
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Local commit refuses a hand-written payout (was accepted)"
else
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Local commit refuses a hand-written payout"
fi
THEFT_ID=$(python3 - "$POOL" "$KB_ID" "$MALLORY_ID" <<'PY'
import hashlib, json, pathlib, sys
pool, creator, to = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
head = (pool / ".contract/HEAD").read_text().strip()
commit = {
    "body": [{"method": "send", "path": None,
              "value": {"asset_contract": creator, "asset_id": "tokB", "to_contract": to, "amount": 1000}}],
    "head": {"parent": head},
}
# The id hashes the commit as a node serializes it: compact, keys sorted.
blob = json.dumps(commit, separators=(",", ":"), ensure_ascii=False, sort_keys=True)
commit_id = hashlib.sha256(blob.encode()).hexdigest()
(pool / ".contract/commits" / f"{commit_id}.json").write_text(json.dumps(commit, indent=2) + "\n")
(pool / ".contract/HEAD").write_text(commit_id + "\n")
print(commit_id)
PY
)
push "$POOL"
expect_log "Failed to process sequenced commit $THEFT_ID.*emitted_by" "The sequencer refuses a hand-written payout, naming emitted_by" || true
reset_pool "$BEFORE_THEFT" "$THEFT_ID"

echo ""
echo "A swap that misses its min_out is returned, whatever the invoker claims..."
TIGHT='{"op":"swap","min_out":400}'
commit "$TRADER_WALLET" "The trader sends 100 A with min_out 400" --method send --asset-contract "$KA_ID" --asset-id tokA --to-contract "$POOL_ID" --amount 100 --memo "'$TIGHT'"
TIGHT_SEND=$LAST
BEFORE_TIGHT=$(cat "$POOL/.contract/HEAD")
invoke "Mallory invokes it, claiming min_out 0" "$MALLORY" "$(invoke_value swap "$(claim "$TIGHT_SEND" "$TRADER_ID" "$KA_ID" tokA 100 '{"op":"swap","min_out":0}')")"
LOOSE_ID=$LAST
expect_log "Failed to process sequenced commit $LOOSE_ID.*has memo" "The sequencer refuses a RECV that misstates the memo, naming it" || true
reset_pool "$BEFORE_TIGHT" "$LOOSE_ID"
invoke "The trader invokes it as sent" "$TRADER" "$(invoke_value swap "$(claim "$TIGHT_SEND" "$TRADER_ID" "$KA_ID" tokA 100 "$TIGHT")")"
REFUND_ID=$LAST
expect_log "Sequenced commit $REFUND_ID" "The returned swap should be sequenced" || true
commit "$TRADER_WALLET" "The trader receives the 100 A back" --method recv --send-commit-id "$REFUND_ID" --asset-contract "$KA_ID" --asset-id tokA --amount 100

echo ""
echo "The LP removes half..."
REMOVE='{"op":"remove"}'
commit "$LP_WALLET" "The LP sends 1000 shares back" --method send --asset-contract "$POOL_ID" --asset-id lp --to-contract "$POOL_ID" --amount 1000 --memo "'$REMOVE'"
REMOVE_SEND=$LAST
rm -rf ./tmp/pool-stale
cp -R "$POOL" ./tmp/pool-stale
invoke "The LP invokes remove" "$LP_KEY" "$(invoke_value remove "$(claim "$REMOVE_SEND" "$LP_ID" - lp 1000 "$REMOVE")")"
REMOVE_ID=$LAST
expect_log "Sequenced commit $REMOVE_ID" "The remove should be sequenced" || true
RACE_VALUE=$(invoke_value refund "$(claim "$REMOVE_SEND" "$LP_ID" - lp 1000 "$REMOVE")")
assert_success "modal contract commit --theory v2 --dir ./tmp/pool-stale --method invoke --path $PROGRAM --value '$RACE_VALUE' --sign $MALLORY --output json" \
  "Mallory, on the head before the remove, invokes a refund of the same SEND"
RACE_ID=$(cat ./tmp/pool-stale/.contract/HEAD)
push ./tmp/pool-stale
expect_log "Failed to process sequenced commit $RACE_ID.*forks the contract" "The sequencer refuses a second child of the same head" || true
commit "$LP_WALLET" "The LP receives 1100 / 2 = 550 A" --method recv --send-commit-id "$REMOVE_ID" --asset-contract "$KA_ID" --asset-id tokA --amount 550
commit "$LP_WALLET" "The LP receives 3638 / 2 = 1819 B" --method recv --send-commit-id "$REMOVE_ID" --send-index 1 --asset-contract "$KB_ID" --asset-id tokB --amount 1819

set +e
REPLAY_JSON=$(modal contract replay --remote "$REMOTE" --contract-id "$POOL_ID" --through "$REMOVE_ID" --output json 2>./tmp/replay.err)
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
assert d.get('invokes_expanded',0) >= 4
"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} A stranger replays the pool, re-running every invoke"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} A stranger replays the pool, re-running every invoke"
    echo "$REPLAY_JSON"
fi

test_finalize
exit $?
