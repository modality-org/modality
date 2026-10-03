#!/usr/bin/env bash
# An agent on a git repo: edits and local commits are free, but the origin's
# main moves only along the contract's path. A clean change lands after a
# passing attestation; a protected change needs a review of that sha; a
# failing, unattested, forged, or wrongly signed step is refused by the hub,
# and a push of main the contract did not accept is refused by the origin.
# No network node: a local hub holds the log. Unnumbered.

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
MODAL_GIT="$(cd ../../../examples/agent-git-landing && pwd)/modal-git"

rm -rf ./tmp
test_init "agent-git-landing"
TMP="$(pwd)/tmp"
KEYS="$TMP/keys"
mkdir -p "$KEYS"
export GIT_AUTHOR_NAME=agent GIT_AUTHOR_EMAIL=agent@example.com
export GIT_COMMITTER_NAME=agent GIT_COMMITTER_EMAIL=agent@example.com

for who in steward agent ci reviewer mallory; do
    assert_success "modal id create --path $KEYS/$who.mod_passfile" "Should create $who's passfile"
done
STEWARD="$KEYS/steward.mod_passfile"
AGENT="$KEYS/agent.mod_passfile"
CI="$KEYS/ci.mod_passfile"
REVIEWER="$KEYS/reviewer.mod_passfile"
MALLORY="$KEYS/mallory.mod_passfile"

PORT=18571
test_start_process "modal hub start --host 127.0.0.1 --port $PORT --rpc-port 0 --data-dir $TMP/hub" "hub" >/dev/null
assert_success "test_wait_for_port $PORT" "The hub should listen on $PORT"

# The governed repo: a bare origin whose main holds a passing test suite.
git init --quiet --bare --initial-branch=main "$TMP/origin.git"
git init --quiet --initial-branch=main "$TMP/seed"
mkdir -p "$TMP/seed/src" "$TMP/seed/tests"
echo "ok" > "$TMP/seed/src/app.txt"
printf '#!/bin/sh\ngrep -q ok src/app.txt\n' > "$TMP/seed/tests/test.sh"
git -C "$TMP/seed" add -A && git -C "$TMP/seed" commit --quiet -m "Seed"
git -C "$TMP/seed" push --quiet "$TMP/origin.git" main

assert_success "$MODAL_GIT init --hub http://127.0.0.1:$PORT --origin $TMP/origin.git \
    --steward $STEWARD --agent $AGENT --ci $CI --reviewer $REVIEWER \
    --protected 'tests/**' --test 'sh tests/test.sh' > $TMP/url" \
    "The steward bootstraps the contract and the origin hook"
export MODAL_GIT_HUB="$(tail -1 "$TMP/url")"
mg() { "$MODAL_GIT" "$@"; }
origin_main() { git -C "$TMP/origin.git" rev-parse refs/heads/main; }

git clone --quiet "$TMP/origin.git" "$TMP/work"
WORK="$TMP/work"

echo ""
echo "Editing and local commits are free..."
echo "ok, with a feature" > "$WORK/src/app.txt"
assert_success "git -C $WORK commit --quiet -am 'Add a feature'" "The agent commits locally with no contract step"
SHA_A=$(git -C "$WORK" rev-parse HEAD)
assert_failure "git -C $WORK push --quiet origin HEAD:main" \
    "Pushing main straight to the origin is refused"

echo ""
echo "A clean change lands along the path..."
assert_failure "mg propose --repo $WORK --sign $MALLORY" "A key that is not an agent's cannot propose"
assert_success "mg propose --repo $WORK --sign $AGENT" "The agent proposes its commit"
assert_failure "mg land --repo $WORK --sign $AGENT" "Landing before any attestation is refused"
assert_failure "mg attest --origin $TMP/origin.git --sign $AGENT" "The agent cannot attest its own change"
assert_success "mg attest --origin $TMP/origin.git --sign $CI > $TMP/attest-a.json" "The runner attests"
assert_success "grep -q '\"passed\": true' $TMP/attest-a.json && grep -q '\"protected\": false' $TMP/attest-a.json" \
    "The attestation: tests pass, nothing protected"
assert_failure "mg land --repo $WORK --sign $CI" "The runner cannot land"
assert_success "mg land --repo $WORK --sign $AGENT" "The agent lands"
assert_success "[ \"\$(origin_main)\" = $SHA_A ]" "The origin's main is the landed sha"

echo ""
echo "A protected change needs a review of that sha..."
printf '#!/bin/sh\ngrep -q ok src/app.txt && test -f src/app.txt\n' > "$WORK/tests/test.sh"
git -C "$WORK" commit --quiet -am "Tighten the test"
SHA_B=$(git -C "$WORK" rev-parse HEAD)
assert_success "mg propose --repo $WORK --sign $AGENT" "The agent proposes a change to the tests"
assert_success "mg attest --origin $TMP/origin.git --sign $CI > $TMP/attest-b.json" "The runner attests"
assert_success "grep -q '\"protected\": true' $TMP/attest-b.json" "The attestation marks it protected"
assert_failure "mg land --repo $WORK --sign $AGENT" "Landing a protected change without a review is refused"
assert_failure "mg review --sign $AGENT" "The agent cannot review its own change"
assert_success "mg review --sign $REVIEWER" "The reviewer approves that sha"
assert_success "mg land --repo $WORK --sign $AGENT" "The reviewed change lands"
assert_success "[ \"\$(origin_main)\" = $SHA_B ]" "The origin's main is the reviewed sha"

echo ""
echo "A failing change does not land..."
echo "nope" > "$WORK/src/app.txt"
git -C "$WORK" commit --quiet -am "Break it"
SHA_C=$(git -C "$WORK" rev-parse HEAD)
assert_success "mg propose --repo $WORK --sign $AGENT" "The agent proposes a breaking change"
assert_success "mg attest --origin $TMP/origin.git --sign $CI > $TMP/attest-c.json" "The runner attests"
assert_success "grep -q '\"passed\": false' $TMP/attest-c.json" "The attestation records the failure"
assert_failure "mg land --repo $WORK --sign $AGENT" "A failed run is refused"
assert_failure "git -C $WORK push --quiet origin $SHA_C:main" "The origin refuses the unlanded sha"
mg status > "$TMP/status.json" || true
assert_success "grep -q 'passing tests' $TMP/status.json" "Status names what is missing"

echo ""
echo "A client that skips local verify is refused by the hub..."
# Sign a head write in a scratch copy whose history has no model or rules.
# The signature binds the real parent, so only the hub's rules stand in the way.
modal c pull "$MODAL_GIT_HUB" --dir "$TMP/forge" --output json >> "$CURRENT_LOG"
cp -R "$TMP/forge" "$TMP/scratch"
python3 - "$TMP/scratch" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
for f in (root / ".contract/commits").glob("*.json"):
    c = json.loads(f.read_text())
    c["body"] = [a for a in c["body"] if a.get("method") not in ("model", "rule")]
    f.write_text(json.dumps(c, indent=2) + "\n")
(root / "model/default.modality").unlink(missing_ok=True)
for f in (root / "rules").glob("*"):
    f.unlink()
PY
modal c set --dir "$TMP/scratch" /main/head.text "$SHA_C"
assert_success "modal c commit --all --dir $TMP/scratch --sign $AGENT --output json" \
    "A client that skips the rules signs a head write"
FORGED=$(cat "$TMP/scratch/.contract/HEAD")
cp "$TMP/scratch/.contract/commits/$FORGED.json" "$TMP/forge/.contract/commits/"
echo "$FORGED" > "$TMP/forge/.contract/HEAD"
assert_failure "modal c push --dir $TMP/forge --remote $MODAL_GIT_HUB --output json" \
    "The hub refuses the forged land"
assert_failure "git -C $WORK push --quiet origin $SHA_C:main" "The origin still refuses the unlanded sha"

echo ""
echo "Audit: the log, the evidence, and the origin agree..."
assert_success "mg audit --origin $TMP/origin.git > $TMP/audit.json" "The audit replays the log and finds no problems"
assert_success "[ \"\$(grep -c '\"commit\"' $TMP/audit.json)\" = 2 ]" "The audit lists both lands"
# The operator moves main around the hook. Anyone with the log sees it.
git -C "$TMP/origin.git" update-ref refs/heads/main "$SHA_C"
assert_failure "mg audit --origin $TMP/origin.git" "The audit catches a main the contract did not accept"

test_finalize
