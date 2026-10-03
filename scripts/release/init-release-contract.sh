#!/usr/bin/env bash
# Create a channel's release contract (once per channel). Run by a
# maintainer, from the repo root, with a `modal` built from this commit.
#
#   scripts/release/init-release-contract.sh [channel]
#
# 1. Uses (or creates) the maintainer key ~/.modality/release-keys/maintainer.mod_passfile.
#    Keep it offline afterwards: it is what may change the release key.
# 2. Generates the CI release key, stores it only as the GitHub Actions
#    secret RELEASE_CI_PASSFILE, and deletes the local copy. If it is ever
#    lost or exposed, the maintainer replaces /keys/ci.id with a commit; no
#    binary changes.
# 3. Creates the release contract and writes its pin into
#    rust/modality-common/src/release.rs.
# 4. Uploads the contract's log to <channel>/release-contract/log.json.
set -euo pipefail

CHANNEL="${1:-testnet}"
MODAL="${MODAL:-rust/target/debug/modal}"
REPO="${REPO:-modality-org/modality}"
BUCKET="${BUCKET:-get.modality.org-content}"
KEYS="$HOME/.modality/release-keys"
MAINTAINER="$KEYS/maintainer.mod_passfile"
OUT="${OUT:-release-contract-$CHANNEL}"

[ -x "$MODAL" ] || { echo "build modal first: (cd rust && cargo build -p modal)" >&2; exit 1; }
[ -e "$OUT" ] && { echo "$OUT already exists" >&2; exit 1; }

mkdir -p "$KEYS"
chmod 700 "$KEYS"
if [ ! -e "$MAINTAINER" ]; then
    "$MODAL" id create --path "$MAINTAINER" >/dev/null
    chmod 600 "$MAINTAINER"
    echo "Created maintainer key $MAINTAINER"
fi

umask 077
CI_KEY="$(mktemp)"
trap 'rm -f "$CI_KEY"' EXIT
rm -f "$CI_KEY"
"$MODAL" id create --path "$CI_KEY" >/dev/null
CI_ID=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['id'])" "$CI_KEY")
gh secret set RELEASE_CI_PASSFILE --repo "$REPO" < "$CI_KEY"
rm -f "$CI_KEY"
echo "Release key $CI_ID stored as RELEASE_CI_PASSFILE in $REPO"

"$MODAL" release init --dir "$OUT/contract" --maintainer "$MAINTAINER" --ci "$CI_ID"
"$MODAL" release export --dir "$OUT/contract" --out "$OUT/log.json"
read -r CONTRACT GENESIS < <(python3 -c "
import json,sys; d=json.load(open(sys.argv[1])); print(d['contract_id'], d['commits'][0]['commit_id'])" "$OUT/log.json")

python3 - "$CHANNEL" "$CONTRACT" "$GENESIS" <<'PY'
import pathlib, re, sys
channel, contract, genesis = sys.argv[1:4]
p = pathlib.Path("rust/modality-common/src/release.rs")
s = p.read_text()
name = channel.upper()
pattern = re.compile(
    rf'(pub const {name}: Pin = Pin \{{\n    contract_id: ")[^"]*(",\n    genesis_commit_id: ")[^"]*(",)')
if not pattern.search(s):
    sys.exit(f"no pin for {channel} in {p}")
s = pattern.sub(rf"\g<1>{contract}\g<2>{genesis}\g<3>", s)
p.write_text(s)
print(f"Pinned {channel}: {contract} at {genesis} in {p}")
PY

aws s3 cp "$OUT/log.json" "s3://$BUCKET/$CHANNEL/release-contract/log.json" --content-type application/json
echo "Published the release log. Commit release.rs, then tag $CHANNEL-<name> to release."
echo "Keep $OUT/contract (or just its log); the maintainer changes keys from it."
