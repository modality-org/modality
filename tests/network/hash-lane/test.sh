#!/usr/bin/env bash
# The hash lane orders a commit's hash without its body. A reveal pushes the
# body later; it is sequenced only if it hashes to a certified hash
# commitment. A block carries at most the network's quota of hash
# commitments. A hash-only commit is not a sequenced commit: nothing can
# REPOST from it and pull does not return it. Unnumbered: not part of the
# stable numbered suite.

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
test_init "hash-lane"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
NODE_DIR="./tmp/node1"
NOTES="./tmp/notes"
QUOTA=2

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

SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer.log"
expect_log() {
    local pattern="$1"
    local desc="$2"
    local timeout="${3:-120}"
    TESTS_RUN=$((TESTS_RUN + 1))
    if test_wait_for_log "$SEQUENCER_LOG" "$pattern" "$timeout"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $desc"
        return 0
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $desc"
    [ -f "$SEQUENCER_LOG" ] && tail -20 "$SEQUENCER_LOG"
    return 1
}

anchor_status() {
    modal contract anchor --status --commit "$2" --dir "$1" --remote "$REMOTE" --output json \
      2>>"$CURRENT_LOG" | python3 -c "import json,sys; print(json.load(sys.stdin)['commits'][0]['status'])"
}

wait_status() {
    local dir="$1" commit="$2" want="$3"
    for _ in $(seq 1 60); do
        [ "$(anchor_status "$dir" "$commit")" = "$want" ] && return 0
        sleep 2
    done
    return 1
}

echo ""
echo "Starting a devnet1 sequencer whose network has a hash lane ($QUOTA per block, 8 bits)..."
assert_success \
    "modal node create --dir $NODE_DIR --from-template devnet1/node1" \
    "Should create sequencer node"
modal node clear-storage --dir "$NODE_DIR" --yes >/dev/null 2>&1 || true
python3 - "$NODE_DIR" "$QUOTA" <<'PY'
import json, pathlib, sys
node = pathlib.Path(sys.argv[1])
info = json.loads(pathlib.Path("../../../rust/modality-networks/networks/devnet1/info.json").read_text())
info["hash_lane"] = {"quota_per_block": int(sys.argv[2]), "floor_bits": 8, "algorithm": "sha256"}
(node / "network.json").write_text(json.dumps(info, indent=2) + "\n")
config = json.loads((node / "config.json").read_text())
config["network_config_path"] = "./network.json"
(node / "config.json").write_text(json.dumps(config, indent=2) + "\n")
PY
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101"
expect_log "Hash lane: $QUOTA per sequencer block, 8 bits of sha256 work" \
  "Sequencer reports the hash lane" 10 || true

echo ""
echo "Anchoring a new contract's first two commits without their bodies..."
modal contract create --dir "$NOTES" >> "$CURRENT_LOG"
modal contract commit --dir "$NOTES" --path /notes/a.text --value "a" >> "$CURRENT_LOG"
CONTRACT_ID=$(modal contract id --dir "$NOTES")
A_ID=$(cat "$NOTES/.contract/HEAD")
check "Before anchoring, the node knows nothing of the commit" \
  test "$(anchor_status "$NOTES" "$A_ID")" = "unknown"
modal contract anchor --dir "$NOTES" --remote "$REMOTE" --remote-name origin --output json \
  > ./tmp/anchor-a.json 2>>"$CURRENT_LOG"
cat ./tmp/anchor-a.json >> "$CURRENT_LOG"
check "Anchor submits both unpushed commits" \
  python3 -c "import json; c=json.load(open('./tmp/anchor-a.json'))['commits']; assert len(c)==2 and all(x['status']=='queued' for x in c)"
expect_log "Hash commitment $A_ID for contract $CONTRACT_ID anchored in round" \
  "A certified sequencer block anchors the commit's hash" || true
check "The node reports the commit anchored" wait_status "$NOTES" "$A_ID" anchored

CLONE="./tmp/clone"
mkdir -p "$CLONE/.contract/commits"
cp "$NOTES/.contract/config.json" "$CLONE/.contract/config.json"
modal contract pull --dir "$CLONE" --remote "$REMOTE" --remote-name origin --output json \
  >> "$CURRENT_LOG" 2>&1 || true
check "Pull returns no body for a hash-only commit" test ! -f "$CLONE/.contract/commits/${A_ID}.json"

echo ""
echo "Revealing the bodies..."
modal contract push --reveal --dir "$NOTES" --remote "$REMOTE" --remote-name origin --output json \
  >> "$CURRENT_LOG" 2>&1
expect_log "Sequenced commit $A_ID" "The reveal is checked and sequenced" || true
check "The node reports the commit revealed" wait_status "$NOTES" "$A_ID" revealed

echo ""
echo "A reveal of a commit that was never anchored is refused..."
modal contract commit --dir "$NOTES" --path /notes/b.text --value "b" >> "$CURRENT_LOG"
B_ID=$(cat "$NOTES/.contract/HEAD")
check "push --reveal refuses an unanchored commit" \
  bash -c "! modal contract push --reveal --dir '$NOTES' --remote '$REMOTE' --remote-name origin >> '$CURRENT_LOG' 2>&1"

echo ""
echo "A reveal whose body does not hash to its commitment is refused..."
modal contract anchor --dir "$NOTES" --remote "$REMOTE" --remote-name origin >> "$CURRENT_LOG" 2>&1
check "The second commit is anchored" wait_status "$NOTES" "$B_ID" anchored
B_FILE="$NOTES/.contract/commits/${B_ID}.json"
cp "$B_FILE" ./tmp/b.json
python3 - "$B_FILE" <<'PY'
import json, sys
path = sys.argv[1]
commit = json.load(open(path))
commit["body"][0]["value"] = "not b"
json.dump(commit, open(path, "w"))
PY
check "push --reveal refuses a body that is not the anchored commit" \
  bash -c "! modal contract push --reveal --dir '$NOTES' --remote '$REMOTE' --remote-name origin >> '$CURRENT_LOG' 2>&1"
cp ./tmp/b.json "$B_FILE"
modal contract push --reveal --dir "$NOTES" --remote "$REMOTE" --remote-name origin >> "$CURRENT_LOG" 2>&1
check "The true body is then sequenced" wait_status "$NOTES" "$B_ID" revealed

echo ""
echo "Anchoring five commits at once: at most $QUOTA per sequencer block..."
for i in 1 2 3 4 5; do
    modal contract commit --dir "$NOTES" --path "/notes/f$i.text" --value "f$i" >> "$CURRENT_LOG"
done
F_ID=$(cat "$NOTES/.contract/HEAD")
modal contract anchor --dir "$NOTES" --remote "$REMOTE" --remote-name origin >> "$CURRENT_LOG" 2>&1
check "The last of the five is eventually anchored" wait_status "$NOTES" "$F_ID" anchored
check "No sequencer block anchored more than $QUOTA" python3 - "$SEQUENCER_LOG" "$QUOTA" <<'PY'
import collections, re, sys
rounds = collections.Counter(re.findall(r"anchored in round (\d+)", open(sys.argv[1]).read()))
print("hash commitments per round:", dict(rounds))
sys.exit(0 if rounds and max(rounds.values()) <= int(sys.argv[2]) else 1)
PY

echo ""
echo "Nothing can REPOST from a hash-only commit..."
DEST="./tmp/dest"
modal contract create --dir "$DEST" >> "$CURRENT_LOG"
modal contract push --dir "$DEST" --remote "$REMOTE" --remote-name origin >> "$CURRENT_LOG" 2>&1
modal contract repost "$CONTRACT_ID" /notes/f5.text --from-dir "$NOTES" --dir "$DEST" >> "$CURRENT_LOG" 2>&1
modal contract commit --all --dir "$DEST" >> "$CURRENT_LOG" 2>&1
check "The node refuses a REPOST whose source commit was only anchored" \
  bash -c "! modal contract push --dir '$DEST' --remote '$REMOTE' --remote-name origin >> '$CURRENT_LOG' 2>&1"

echo ""
echo "Restarting the sequencer: the anchored and revealed commits are still indexed..."
test_cleanup
sleep 1
SEQUENCER_LOG="$LOG_DIR/${CURRENT_TEST}_sequencer2.log"
test_start_process "cd $NODE_DIR && modal node run-sequencer" "sequencer2" >/dev/null
assert_success "test_wait_for_port 10101" "Sequencer should listen on 10101 again"
check "After restart, the first commit is still revealed" wait_status "$NOTES" "$A_ID" revealed
check "After restart, the flood's last commit is still anchored only" wait_status "$NOTES" "$F_ID" anchored

test_finalize
exit $?
