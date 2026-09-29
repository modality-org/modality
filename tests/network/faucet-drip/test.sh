#!/usr/bin/env bash
# A rule-gated faucet on a network that enforces predicate theory V2: a
# stranger registers her own key and drips the posted amount once; an
# oversized or second drip, or a key registered by someone else, is refused.
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
test_init "faucet-drip"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
FAUCET="./tmp/faucet"
WALLET="./tmp/carol-wallet"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
mkdir -p "$PASSFILES" "$FAUCET" "$WALLET"

for who in founder alice carol dave mallory; do
    assert_success "modal id create --path $PASSFILES/$who.mod_passfile" "Should create $who's passfile"
done
FOUNDER="$PASSFILES/founder.mod_passfile"
ALICE="$PASSFILES/alice.mod_passfile"
CAROL="$PASSFILES/carol.mod_passfile"
DAVE="$PASSFILES/dave.mod_passfile"
MALLORY="$PASSFILES/mallory.mod_passfile"

assert_success "modal contract create --dir $WALLET" "Carol creates a wallet contract"
WALLET_ID=$(modal contract id --dir "$WALLET")

assert_success "modal contract create --dir $FAUCET" "Should create the faucet contract"
FAUCET_ID=$(modal contract id --dir "$FAUCET")
assert_success "modal checkout --dir $FAUCET" "Should checkout the faucet"
assert_success "modal set-named-id /founder.id $FOUNDER --dir $FAUCET" "Should post the founder's key"
assert_success "modal set-named-id /claimants/alice.id $ALICE --dir $FAUCET" "Should register Alice"
assert_success "modal contract set --dir $FAUCET /config/drip.num 10" "Should post the drip size"

# The founder mints once. Then anyone registers a fresh slot with a key she
# holds, and a claimant drips once, alone, exactly the posted drip, marking
# her own flag. Nothing changes the config.
mkdir -p "$FAUCET/model"
cat > "$FAUCET/model/default.modality" <<'EOF'
model Faucet {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +CREATE -SEND -modifies(/claimants) -modifies(/config) +signed_by(/founder.id)
  q2 --> q2: +POST -SEND -CREATE -modifies(/claimants) -modifies(/config)
  q2 --> q2: +POST -SEND -CREATE -modifies(/config) -state_exists(/claimants/$k.id) +post_to_path(/claimants/$k.id) +posts_own_key(/claimants/$k.id) -modifies(/claimants/$k) -modifies(/claimants/!$k)
  q2 --> q2: +SEND +POST -CREATE -modifies(/config) +signed_by(/claimants/$k.id) -signed_by(/claimants/!$k.id) -bool_true(/claimants/$k/claimed.bool) +post_to(/claimants/$k/claimed.bool, "true") -modifies(/claimants/$k.id) -modifies(/claimants/!$k) +sent_eq("drops", /config/drip.num)
}
EOF

assert_success "modal add-rule --name config_fixed --dir $FAUCET 'always([+modifies(/config)] false)'" \
  "Should add: the config never changes"
assert_success "modal add-rule --name drip_size --dir $FAUCET 'always([+SEND -sent_eq(\"drops\", /config/drip.num)] false)'" \
  "Should add: every SEND is exactly the posted drip"
assert_success "modal add-rule --name drips_once --dir $FAUCET 'always([+SEND +signed_by(/claimants/\$k.id) +bool_true(/claimants/\$k/claimed.bool)] false)'" \
  "Should add: each claimant drips once"
assert_success "modal add-rule --name drip_marks --dir $FAUCET 'always([+SEND +signed_by(/claimants/\$k.id) -post_to(/claimants/\$k/claimed.bool, \"true\")] false)'" \
  "Should add: a drip marks the claimant's flag"
# V0 refuses these two against this model (it reads labels as names); the
# network runs V2, so local verify runs V2 too.
assert_success "modal add-rule --name drips_signed --dir $FAUCET 'always([+SEND -any_signed(/claimants)] false)'" \
  "Should add: every SEND is signed by a claimant"
assert_success "modal add-rule --name own_keys --dir $FAUCET 'always([+post_to_path(/claimants/\$k.id) -posts_own_key(/claimants/\$k.id)] false)'" \
  "Should add: a key is registered by its holder"
assert_success "modal commit --theory v2 --all --dir $FAUCET --sign $FOUNDER --output json --message Bootstrap" \
  "Should commit the bootstrap"
assert_success "modal contract commit --theory v2 --dir $FAUCET --method create --asset-id drops --quantity 1000 --divisibility 1 --sign $FOUNDER --output json" \
  "The founder mints the pool"
assert_success "modal add-rule --name finite --dir $FAUCET 'always([+CREATE] false)'" \
  "Should add: no more minting"
assert_success "modal contract set --dir $FAUCET /notes/finite.text closed" "Should note the pool closed"
assert_success "modal commit --theory v2 --all --dir $FAUCET --sign $FOUNDER --output json --message Finite" \
  "Should commit the no-mint rule"
SETUP_HEAD=$(cat "$FAUCET/.contract/HEAD")

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
    modal contract push --dir "$FAUCET" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG"
}
refused_locally() {
    local desc="$1"
    shift
    TESTS_RUN=$((TESTS_RUN + 1))
    if "$@" > ./tmp/refused.out 2> ./tmp/refused.err; then
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $desc (was accepted)"
    else
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
    fi
    cat ./tmp/refused.err >> "$CURRENT_LOG"
}
expect_log "Predicate theory: v2" "Sequencer reports predicate theory v2" 10 || true

push
expect_log "Sequenced commit $SETUP_HEAD" "Bootstrap, mint and rules should be sequenced" || true

echo ""
echo "Registration: only with a key you hold..."
assert_success "modal set-named-id /claimants/dave.id $DAVE --dir $FAUCET" "Mallory writes Dave's key into a slot"
refused_locally "Mallory cannot register a key she does not hold" \
    modal commit --theory v2 --all --dir "$FAUCET" --sign "$MALLORY" --output json
rm -f "$FAUCET/state/claimants/dave.id"
assert_success "modal set-named-id /claimants/carol.id $CAROL --dir $FAUCET" "Carol writes her own key"
assert_success "modal commit --theory v2 --all --dir $FAUCET --sign $CAROL --output json --message Register" \
  "Carol registers herself"
REGISTER_HEAD=$(cat "$FAUCET/.contract/HEAD")
push
expect_log "Sequenced commit $REGISTER_HEAD" "Carol's registration should be sequenced" || true

echo ""
echo "Drips: exactly the posted amount, once..."
assert_success "modal contract set --dir $FAUCET /claimants/carol/claimed.bool false" "Carol leaves her flag unset"
refused_locally "A drip that leaves the flag unset is refused locally" \
    modal commit --theory v2 --all --dir "$FAUCET" --method send --asset-id drops \
    --to-contract "$WALLET_ID" --amount 10 --sign "$CAROL" --output json
assert_success "modal contract set --dir $FAUCET /claimants/carol/claimed.bool true" "Carol marks her flag"
refused_locally "An oversized drip is refused locally" \
    modal commit --theory v2 --all --dir "$FAUCET" --method send --asset-id drops \
    --to-contract "$WALLET_ID" --amount 11 --sign "$CAROL" --output json
TESTS_RUN=$((TESTS_RUN + 1))
if grep -q 'sent_eq' ./tmp/refused.err; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} The refusal names the drip size"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} The refusal names the drip size"
fi

# A client that skips local verify: sign an oversized drip in a scratch copy
# whose history has no model or rules. The signature binds the real parent,
# so only the network's rules stand in the way.
rm -rf ./tmp/scratch
cp -R "$FAUCET" ./tmp/scratch
python3 - ./tmp/scratch <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
for f in (root / ".contract/commits").glob("*.json"):
    c = json.loads(f.read_text())
    c["body"] = [a for a in c["body"] if a.get("method") not in ("model", "rule")]
    f.write_text(json.dumps(c, indent=2) + "\n")
(root / "model/default.modality").unlink()
for f in (root / "rules").glob("*"):
    f.unlink()
PY
assert_success "modal commit --all --dir ./tmp/scratch --method send --asset-id drops --to-contract $WALLET_ID --amount 11 --sign $CAROL --output json" \
  "A client that skips local verify signs an oversized drip"
BIG_ID=$(cat ./tmp/scratch/.contract/HEAD)
cp "./tmp/scratch/.contract/commits/$BIG_ID.json" "$FAUCET/.contract/commits/"
echo "$BIG_ID" > "$FAUCET/.contract/HEAD"
push
expect_log "Failed to process sequenced commit $BIG_ID" "The sequencer refuses the oversized drip" || true
TESTS_RUN=$((TESTS_RUN + 1))
if grep "Failed to process sequenced commit $BIG_ID" "$SEQUENCER_LOG" | grep -q 'sent_eq'; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} The sequencer names the drip size"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} The sequencer names the drip size"
fi
echo "$REGISTER_HEAD" > "$FAUCET/.contract/HEAD"
rm -f "$FAUCET/.contract/commits/$BIG_ID.json"
rm -rf "$FAUCET/.contract/refs/remotes/origin"

assert_success "modal commit --theory v2 --all --dir $FAUCET --method send --asset-id drops --to-contract $WALLET_ID --amount 10 --sign $CAROL --output json --message Drip" \
  "Carol drips the posted amount"
DRIP_HEAD=$(cat "$FAUCET/.contract/HEAD")
push
expect_log "Sequenced commit $DRIP_HEAD" "Carol's drip should be sequenced" || true

echo ""
echo "Carol's wallet receives the drip..."
push_wallet() {
    modal contract push --dir "$WALLET" --remote "$REMOTE" --remote-name origin --output json >> "$CURRENT_LOG"
}
assert_success "modal contract commit --dir $WALLET --method recv --send-commit-id $DRIP_HEAD --sign $CAROL --output json" \
  "Carol receives the drip into her wallet"
RECV_HEAD=$(cat "$WALLET/.contract/HEAD")
push_wallet
expect_log "Sequenced commit $RECV_HEAD" "The RECV should be sequenced" || true
assert_success "modal contract commit --dir $WALLET --method recv --send-commit-id $DRIP_HEAD --sign $CAROL --output json" \
  "Carol signs a second RECV of the same drip"
RECV_AGAIN=$(cat "$WALLET/.contract/HEAD")
push_wallet
expect_log "Failed to process sequenced commit $RECV_AGAIN" "The second RECV is refused" || true

assert_success "modal contract set --dir $FAUCET /notes/again.text again" "Carol tries again"
refused_locally "A second drip is refused" \
    modal commit --theory v2 --all --dir "$FAUCET" --method send --asset-id drops \
    --to-contract "$WALLET_ID" --amount 10 --sign "$CAROL" --output json

set +e
REPLAY_JSON=$(modal contract replay --remote "$REMOTE" --contract-id "$FAUCET_ID" --through "$DRIP_HEAD" --output json 2>./tmp/replay.err)
REPLAY_STATUS=$?
set -e
echo "$REPLAY_JSON" >> "$CURRENT_LOG"
cat ./tmp/replay.err >> "$CURRENT_LOG" || true
TESTS_RUN=$((TESTS_RUN + 1))
if [ "$REPLAY_STATUS" -eq 0 ] && echo "$REPLAY_JSON" | python3 -c "
import json,sys
s=sys.stdin.read()
assert json.loads(s[s.find('{'):]).get('ok') is True
"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} A stranger replays the faucet and accepts Carol's drip"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} A stranger replays the faucet and accepts Carol's drip"
    echo "$REPLAY_JSON"
fi

test_finalize
exit $?
