#!/usr/bin/env bash
# A network that enforces predicate theory V1 refuses a model with a dead edge,
# even though local verify (V0) accepts it and only warns. The fixed model is
# sequenced and pulled. Unnumbered: not part of the stable numbered suite.

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
test_init "theory-reject"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
CONTRACT_DIR="./tmp/escrow"
NODE_DIR="./tmp/node1"
PASSFILES="./tmp/passfiles"
mkdir -p "$PASSFILES" "$CONTRACT_DIR"

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

echo ""
echo "Alice sells Bob a laptop for 100 through an escrow contract..."
ALICE_PASS="$PASSFILES/alice.mod_passfile"
BOB_PASS="$PASSFILES/bob.mod_passfile"
assert_success "modal id create --path $ALICE_PASS" "Should create Alice passfile"
assert_success "modal id create --path $BOB_PASS" "Should create Bob passfile"
assert_success "modal contract create --dir $CONTRACT_DIR --output json" \
  "Should create a local contract"
assert_success "modal checkout --dir $CONTRACT_DIR" "Should checkout working tree"
assert_success "modal set-named-id /parties/alice.id $ALICE_PASS --dir $CONTRACT_DIR" \
  "Should set Alice identity"
assert_success "modal set-named-id /parties/bob.id $BOB_PASS --dir $CONTRACT_DIR" \
  "Should set Bob identity"
GENESIS_HEAD=$(cat "$CONTRACT_DIR/.contract/HEAD")

# The refund edge was copied from the release edge and `num_gte` was not
# deleted: paid cannot be below 100 and at least 100 at once.
mkdir -p "$CONTRACT_DIR/model"
cat > "$CONTRACT_DIR/model/default.modality" <<'EOF'
model Escrow {
  part flow {
    q0 --> open
    open --> open: +post_to_path(/escrow/paid.num)
    open --> released: +signed_by(/parties/alice.id) +num_gte(/escrow/paid.num,"100")
    open --> refunded: +signed_by(/parties/bob.id) +num_lt(/escrow/paid.num,"100") +num_gte(/escrow/paid.num,"100")
  }
}
EOF

echo ""
echo "Local verify (V0) accepts the slipped model and previews the V1 refusal..."
SLIP_OUT=$(modal commit --all --dir "$CONTRACT_DIR" --output json --message "Escrow with a slipped refund edge")
echo "$SLIP_OUT" > ./tmp/commit-slipped.json
echo "$SLIP_OUT" >> "$CURRENT_LOG"
SLIP_ID=$(cat "$CONTRACT_DIR/.contract/HEAD")
check "Local commit carries a theory_preview" grep -q '"theory_preview"' ./tmp/commit-slipped.json

modal contract theory --dir "$CONTRACT_DIR" --output json > ./tmp/theory-slipped.json
EXPLANATION=$(python3 - ./tmp/theory-slipped.json <<'PY'
import json, sys
view = json.load(open(sys.argv[1]))
edges = view["dead_edges"]
print(", ".join(edges[0]["offending"]) if edges else "")
PY
)
echo "modal contract theory: $EXPLANATION" >> "$CURRENT_LOG"
check "modal contract theory names the dead refund edge" test -n "$EXPLANATION"

echo ""
echo "Starting a devnet1 validator whose network enforces predicate theory V1..."
assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create validator node"
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
python3 - "$NODE_DIR" <<'PY'
import json, pathlib, sys
node = pathlib.Path(sys.argv[1])
info = json.loads(pathlib.Path("../../../rust/modality-networks/networks/devnet1/info.json").read_text())
info["predicate_theory_version"] = "v1"
(node / "network.json").write_text(json.dumps(info, indent=2) + "\n")
config = json.loads((node / "config.json").read_text())
config["network_config_path"] = "./network.json"
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
NODE_PID=$(test_start_process "cd $NODE_DIR && modal node run-validator" "validator")
# test_start_process runs in a subshell here, so track the PID for cleanup.
PIDS+=("$NODE_PID")
assert_success "test_wait_for_port 10101" "Validator should listen on 10101"
sleep 3

VALIDATOR_LOG="$LOG_DIR/${CURRENT_TEST}_validator.log"
# Rounds are ~30 s apart on one node; a push can take a few rounds to apply.
expect_log() {
    local pattern="$1"
    local desc="$2"
    local timeout="${3:-180}"
    TESTS_RUN=$((TESTS_RUN + 1))
    if test_wait_for_log "$VALIDATOR_LOG" "$pattern" "$timeout"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    if [ -f "$VALIDATOR_LOG" ]; then
        echo "Last 40 lines of validator log:" >> "$CURRENT_LOG"
        tail -40 "$VALIDATOR_LOG" >> "$CURRENT_LOG"
        tail -20 "$VALIDATOR_LOG"
    fi
    return 1
}
expect_log "Predicate theory: v1" "Validator reports predicate theory v1" 10 || true

echo ""
echo "Pushing the slipped model..."
PUSH_SLIP=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_SLIP" >> "$CURRENT_LOG"
expect_log "Failed to process sequenced commit $SLIP_ID" \
  "Validator refuses the model with a dead edge" || true
check "Validator names predicate theory V1 in the refusal" \
  grep -q "no commit can take (predicate theory V1)" "$VALIDATOR_LOG"
check "Validator and modal contract theory give the same explanation" \
  grep -qF "$EXPLANATION" "$VALIDATOR_LOG"

CLONE_DIR="./tmp/clone"
mkdir -p "$CLONE_DIR/.contract/commits"
cp "$CONTRACT_DIR/.contract/config.json" "$CLONE_DIR/.contract/config.json"
modal contract pull --dir "$CLONE_DIR" --remote "$REMOTE" --remote-name origin --output json \
  >> "$CURRENT_LOG" 2>&1 || true
check "Pull does not return the refused commit" \
  test ! -f "$CLONE_DIR/.contract/commits/${SLIP_ID}.json"

echo ""
echo "Deleting the stray num_gte and committing the fixed model..."
echo "$GENESIS_HEAD" > "$CONTRACT_DIR/.contract/HEAD"
cat > "$CONTRACT_DIR/model/default.modality" <<'EOF'
model Escrow {
  part flow {
    q0 --> open
    open --> open: +post_to_path(/escrow/paid.num)
    open --> released: +signed_by(/parties/alice.id) +num_gte(/escrow/paid.num,"100")
    open --> refunded: +signed_by(/parties/bob.id) +num_lt(/escrow/paid.num,"100")
  }
}
EOF
FIX_OUT=$(modal commit --all --dir "$CONTRACT_DIR" --output json --message "Escrow")
echo "$FIX_OUT" > ./tmp/commit-fixed.json
echo "$FIX_OUT" >> "$CURRENT_LOG"
FIX_ID=$(cat "$CONTRACT_DIR/.contract/HEAD")
modal contract theory --dir "$CONTRACT_DIR" --output json > ./tmp/theory-fixed.json
check "modal contract theory finds no dead edges in the fixed model" \
  python3 -c "import json,sys; sys.exit(1 if json.load(open('./tmp/theory-fixed.json'))['dead_edges'] else 0)"

rm -rf "$CONTRACT_DIR/.contract/refs/remotes/origin"
PUSH_FIX=$(modal contract push --dir "$CONTRACT_DIR" --remote "$REMOTE" --remote-name origin --output json)
echo "$PUSH_FIX" >> "$CURRENT_LOG"
expect_log "Sequenced commit $FIX_ID" "Validator sequences the fixed model" || true

CLONE2_DIR="./tmp/clone-after-fix"
mkdir -p "$CLONE2_DIR/.contract/commits"
cp "$CONTRACT_DIR/.contract/config.json" "$CLONE2_DIR/.contract/config.json"
modal contract pull --dir "$CLONE2_DIR" --remote "$REMOTE" --remote-name origin --output json \
  >> "$CURRENT_LOG" 2>&1 || true
check "Pull returns the fixed model" test -f "$CLONE2_DIR/.contract/commits/${FIX_ID}.json"
check "Refused commit is still absent" test ! -f "$CLONE2_DIR/.contract/commits/${SLIP_ID}.json"

test_finalize
exit $?
