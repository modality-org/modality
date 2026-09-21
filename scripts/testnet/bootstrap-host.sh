#!/usr/bin/env bash
# Run on a Foundation Lightsail host after SSH is back.
# Installs the published testnet package, creates the node from a bundled
# template, wipes any old miner datastore, and starts systemd run-hybrid.
#
# Usage:
#   NODE=1 DIR=$HOME/testnet1 ./bootstrap-host.sh
#   NODE=2 DIR=$HOME/testnet2 ./bootstrap-host.sh
#   NODE=3 DIR=$HOME/testnet3 ./bootstrap-host.sh
#
# Strangers should use `modal node create --testnet` instead of this script.
# Do not scp locally-built modal binaries onto these hosts.

set -euo pipefail

NODE="${NODE:?set NODE to 1, 2, or 3}"
DIR="${DIR:-$HOME/testnet${NODE}}"
BIN_DIR="${BIN_DIR:-$HOME/.modality/bin}"
BINARY_URL="${BINARY_URL:-https://get.modality.org/testnet/latest/binaries/linux-x86_64/modal}"
TEMPLATE="testnet/node${NODE}"
STATUS_PORT=$((3100 + NODE))
LIBSSL11_URL="${LIBSSL11_URL:-http://archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.24_amd64.deb}"

EXPECTED_IDS=(
    ""
    "12D3KooWE4NPREQxLkevA5Rxd61Xiue4tTkUGN22qNABD7Mw5JhM"
    "12D3KooWJpFYTRHNuPfwoj1hTf87aqB7CDJHKtVFp3RhPNB1DrRw"
    "12D3KooWLHTsoeBE1ZWBgzumeSi6hsm3o9AndFufrGx7xLTyq2dw"
)

if [[ ! "$NODE" =~ ^[123]$ ]]; then
    echo "NODE must be 1, 2, or 3" >&2
    exit 1
fi

mkdir -p "$BIN_DIR"
echo "Installing modal from $BINARY_URL"
curl -fsSL "$BINARY_URL" -o "$BIN_DIR/modal"
chmod +x "$BIN_DIR/modal"
# Published linux builds until 2026-09-21 linked OpenSSL 1.1; Ubuntu 24.04 does not ship it.
if ldd "$BIN_DIR/modal" 2>/dev/null | grep -q 'libssl.so.1.1 => not found'; then
    echo "Installing OpenSSL 1.1 compatibility package"
    curl -fsSL "$LIBSSL11_URL" -o /tmp/libssl1.1.deb
    sudo dpkg -i /tmp/libssl1.1.deb
fi
grep -q '.modality/bin' "$HOME/.profile" 2>/dev/null || \
    echo 'export PATH="$HOME/.modality/bin:$PATH"' >> "$HOME/.profile"
export PATH="$BIN_DIR:$PATH"
modal --version

if [[ -d "$DIR" ]]; then
    echo "Wiping previous node directory $DIR"
    sudo systemctl stop modal-hybrid.service 2>/dev/null || true
    rm -rf "$DIR"
fi

echo "Creating $DIR from template $TEMPLATE"
mkdir -p "$DIR"
modal node create --dir "$DIR" --from-template "$TEMPLATE"

GOT_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["id"])' "$DIR/config.json")"
WANT_ID="${EXPECTED_IDS[$NODE]}"
if [[ "$GOT_ID" != "$WANT_ID" ]]; then
    echo "peer id mismatch: got $GOT_ID want $WANT_ID" >&2
    exit 1
fi

if [[ -d "$DIR/data" ]] || [[ -d "$DIR/storage" ]]; then
    echo "Wiping old chain storage under $DIR"
    modal node clear-storage --dir "$DIR" --yes || true
    rm -rf "$DIR/storage" "$DIR/data"
fi

# RandomX can still spike; keep a small swap even on 4 GB hosts.
if ! swapon --show | grep -q .; then
    echo "Adding 2G swap"
    sudo fallocate -l 2G /swapfile || sudo dd if=/dev/zero of=/swapfile bs=1M count=2048
    sudo chmod 600 /swapfile
    sudo mkswap /swapfile
    sudo swapon /swapfile
    grep -q '/swapfile' /etc/fstab || echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
fi

UNIT=/etc/systemd/system/modal-hybrid.service
sudo tee "$UNIT" >/dev/null <<EOF
[Unit]
Description=Modality hybrid testnet node ${NODE}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$(id -un)
Group=$(id -gn)
WorkingDirectory=${DIR}
Environment=HOME=${HOME}
Environment=PATH=${BIN_DIR}:/usr/local/bin:/usr/bin:/bin
ExecStart=${BIN_DIR}/modal node run-hybrid --dir ${DIR} --no-tui
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now modal-hybrid.service

if ! command -v caddy >/dev/null 2>&1; then
    echo "Installing Caddy for https://node${NODE}.testnet.modality.network"
    sudo apt-get update -qq
    if ! sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq caddy; then
        echo "Caddy install failed; HTTPS status skipped (TCP 4040 still serves P2P)"
    fi
fi

if command -v caddy >/dev/null 2>&1; then
    sudo tee /etc/caddy/Caddyfile >/dev/null <<EOF
node${NODE}.testnet.modality.network {
	reverse_proxy 127.0.0.1:${STATUS_PORT}
}

node${NODE}.testnet.modal.money {
	redir https://node${NODE}.testnet.modality.network{uri} permanent
}
EOF
    sudo systemctl enable --now caddy
    sudo systemctl reload caddy || sudo systemctl restart caddy
fi

echo "Service status:"
sudo systemctl --no-pager --full status modal-hybrid.service || true
echo "Listening check:"
ss -ltnp | grep -E '4040|80|443' || true
echo "Done node${NODE} peer $GOT_ID"
