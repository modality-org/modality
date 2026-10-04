#!/usr/bin/env bash
# Push the testnet faucet (testnet.json: its genesis and first commit, as
# create.sh made it) to a node. Run after each new testnet genesis, then fund
# it: send MOD to its id from any wallet and commit the RECV in a copy of
# the faucet (anyone may; see README.md).
#
#   scripts/testnet/faucet/push.sh [remote]
#
# remote: a node's multiaddress; default: the testnet's first bootstrapper.
set -euo pipefail

MODAL="${MODAL:-modal}"
HERE="$(cd "$(dirname "$0")" && pwd)"
INFO="$HERE/../../../rust/modality-networks/networks/testnet/info.json"
REMOTE="${1:-$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['bootstrappers'][0])" "$INFO")}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
# `release import` makes a contract directory from any exported log.
"$MODAL" release import --log "$HERE/testnet.json" --dir "$WORK/faucet" > /dev/null
"$MODAL" contract push --dir "$WORK/faucet" --remote "$REMOTE" --remote-name origin
echo "Pushed faucet $("$MODAL" contract id --dir "$WORK/faucet") to $REMOTE"
