#!/usr/bin/env bash
# Replace the published testnet modal binary on a Foundation host.
# Does not wipe chain state unless --wipe-data.
#
# Usage (on the host):
#   ./upgrade-host.sh
#   SERVICE=modal-observer ./upgrade-host.sh
#   ./upgrade-host.sh --wipe-data
#
# Do not scp locally-built modal binaries. Pull from get.modality.org.
set -euo pipefail

BIN_DIR="${BIN_DIR:-$HOME/.modality/bin}"
BINARY_URL="${BINARY_URL:-https://get.modality.org/testnet/latest/binaries/linux-x86_64/modal}"
SERVICE="${SERVICE:-}"
WIPE_DATA=false
DIR="${DIR:-}"

if [[ "${1:-}" == "--wipe-data" ]]; then
    WIPE_DATA=true
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

echo "Starting $SERVICE"
sudo systemctl start "$SERVICE"
sleep 2
systemctl is-active "$SERVICE"
echo "Upgraded $SERVICE dir=${DIR:-unknown}"
