#!/usr/bin/env bash
# Generate this host's next node key, beside the live one. The running node
# is untouched; `upgrade-host.sh --wipe-data --rotate-key` swaps it in at the
# next genesis. The private key never leaves this host. Prints the new peer
# id, for the network config's bootstrappers and validators.
#
# Usage (on the host): ./new-node-key.sh        DIR=~/testnet1 ./new-node-key.sh
set -euo pipefail

BIN_DIR="${BIN_DIR:-$HOME/.modality/bin}"
DIR="${DIR:-}"
if [[ -z "$DIR" ]]; then
    for candidate in "$HOME/testnet1" "$HOME/testnet2" "$HOME/testnet3" "$HOME/testnet0" "$HOME/joiner"; do
        [[ -d "$candidate" ]] && { DIR="$candidate"; break; }
    done
fi
[[ -n "$DIR" ]] || { echo "no node directory; set DIR" >&2; exit 1; }
NEXT="$DIR/node.modal_passfile.next"
if [[ ! -e "$NEXT" ]]; then
    umask 077
    "$BIN_DIR/modal" id create --path "$NEXT" >/dev/null
fi
python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['id'])" "$NEXT"
