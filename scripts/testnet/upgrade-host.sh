#!/usr/bin/env bash
# Replace the published testnet modal binary on a Foundation host.
# Does not wipe chain state unless --wipe-data.
#
# Usage (on the host):
#   VERSION=<release> ./upgrade-host.sh
#   VERSION=<release> SERVICE=modal-observer ./upgrade-host.sh
#   VERSION=<release> ./upgrade-host.sh --wipe-data
#
# Do not scp locally-built modal binaries. Pull from get.modality.org.
set -euo pipefail

BIN_DIR="${BIN_DIR:-$HOME/.modality/bin}"
BASE_URL="${BASE_URL:-https://get.modality.org}"
CHANNEL="${CHANNEL:-testnet}"
# The release to install, by version (as the release contract names it).
VERSION="${VERSION:?set VERSION to the release to install, e.g. VERSION=20261003_120000-abcdef12}"
BINARY_URL="$BASE_URL/$CHANNEL/$VERSION/binaries/linux-x86_64/modal"
SERVICE="${SERVICE:-}"
WIPE_DATA=false
DIR="${DIR:-}"

ROTATE_KEY=false
for arg in "$@"; do
    case "$arg" in
        --wipe-data) WIPE_DATA=true ;;
        --rotate-key) ROTATE_KEY=true ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done
if [[ "$ROTATE_KEY" == true && "$WIPE_DATA" != true ]]; then
    echo "--rotate-key changes this node's identity, which only a new genesis can take: add --wipe-data" >&2
    exit 2
fi

if [[ -z "$SERVICE" ]]; then
    if systemctl list-unit-files modal-hybrid.service >/dev/null 2>&1 && \
        systemctl is-enabled modal-hybrid.service >/dev/null 2>&1; then
        SERVICE=modal-hybrid
    elif systemctl list-unit-files modal-observer.service >/dev/null 2>&1; then
        SERVICE=modal-observer
    elif systemctl list-unit-files modal-miner.service >/dev/null 2>&1; then
        SERVICE=modal-miner
    else
        SERVICE=modal-hybrid
    fi
fi

if [[ -z "$DIR" ]]; then
    for candidate in "$HOME/testnet1" "$HOME/testnet2" "$HOME/testnet3" "$HOME/testnet0" "$HOME/joiner"; do
        if [[ -d "$candidate" ]]; then
            DIR="$candidate"
            break
        fi
    done
fi

mkdir -p "$BIN_DIR"
echo "Downloading $BINARY_URL"
curl -fsSL "$BINARY_URL" -o /tmp/modal-new
chmod +x /tmp/modal-new

# Run nothing the release contract has not accepted. The modal already
# installed checks it: it pins the release contract and trusts no bucket.
# A host whose modal predates `release verify` needs the sha256 of a binary
# checked elsewhere: VERIFIED_SHA256.
if [[ -x "$BIN_DIR/modal" ]] && "$BIN_DIR/modal" release verify --help >/dev/null 2>&1; then
    "$BIN_DIR/modal" release verify --log "$BASE_URL/$CHANNEL/release-contract/log.json" \
        --channel "$CHANNEL" --version "$VERSION" --file "binaries/linux-x86_64/modal=/tmp/modal-new" \
        || { echo "Refusing: the release contract does not accept this binary" >&2; rm -f /tmp/modal-new; exit 1; }
elif [[ -n "${VERIFIED_SHA256:-}" ]]; then
    actual=$(sha256sum /tmp/modal-new | cut -d' ' -f1)
    if [[ "$actual" != "$VERIFIED_SHA256" ]]; then
        echo "Refusing: sha256 $actual is not the verified $VERIFIED_SHA256" >&2
        rm -f /tmp/modal-new
        exit 1
    fi
    echo "sha256 matches the binary verified elsewhere"
else
    echo "Refusing: the installed modal cannot check releases; verify the release with a modal that can and pass VERIFIED_SHA256" >&2
    rm -f /tmp/modal-new
    exit 1
fi
/tmp/modal-new --version || true

echo "Stopping $SERVICE"
sudo systemctl stop "$SERVICE" || true
sleep 1
mv /tmp/modal-new "$BIN_DIR/modal"

if [[ "$WIPE_DATA" == true ]]; then
    if [[ -z "$DIR" ]]; then
        echo "--wipe-data requires DIR=..." >&2
        exit 1
    fi
    echo "Wiping chain storage under $DIR (keeping config.json)"
    rm -rf "$DIR/data" "$DIR/storage"
fi

if [[ "$ROTATE_KEY" == true ]]; then
    [[ -e "$DIR/node.modal_passfile.next" ]] || { echo "no $DIR/node.modal_passfile.next; run new-node-key.sh first" >&2; exit 1; }
    mv "$DIR/node.modal_passfile.next" "$DIR/node.modal_passfile"
    chmod 600 "$DIR/node.modal_passfile"
    echo "Rotated the node key: $(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['id'])" "$DIR/node.modal_passfile")"
fi

echo "Starting $SERVICE"
sudo systemctl start "$SERVICE"
sleep 2
systemctl is-active "$SERVICE"
echo "Upgraded $SERVICE dir=${DIR:-unknown}"
