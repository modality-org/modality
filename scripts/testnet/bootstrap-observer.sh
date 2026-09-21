#!/usr/bin/env bash
# Install the published testnet package and run Foundation node0 as an observer.
# Caddy already reverse-proxies https://node0.testnet.modality.network to :1337.
#
# Usage:
#   DIR=$HOME/testnet0 ./bootstrap-observer.sh
#
# Does not scp locally-built modal binaries. Pull from get.modality.org.
# Do not use this as a joiner; strangers use `modal node create --testnet`.

set -euo pipefail

DIR="${DIR:-$HOME/testnet0}"
BIN_DIR="${BIN_DIR:-$HOME/.modality/bin}"
BINARY_URL="${BINARY_URL:-https://get.modality.org/testnet/latest/binaries/linux-x86_64/modal}"
TEMPLATE="testnet/node0"
EXPECTED_ID="12D3KooWSB2d9ddmQkvwpPWFXVJrgnDqEiaRTQXguTacwnxL6MrE"

mkdir -p "$BIN_DIR"
echo "Installing modal from $BINARY_URL"
curl -fsSL "$BINARY_URL" -o "$BIN_DIR/modal"
chmod +x "$BIN_DIR/modal"
grep -q '.modality/bin' "$HOME/.profile" 2>/dev/null || \
    echo 'export PATH="$HOME/.modality/bin:$PATH"' >> "$HOME/.profile"
export PATH="$BIN_DIR:$PATH"
modal --version

if [[ -d "$DIR" ]]; then
    echo "Wiping previous node directory $DIR"
    sudo systemctl stop modal-observer.service 2>/dev/null || true
    rm -rf "$DIR"
fi

echo "Creating $DIR from template $TEMPLATE"
mkdir -p "$DIR"
modal node create --dir "$DIR" --from-template "$TEMPLATE"

GOT_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["id"])' "$DIR/config.json")"
if [[ "$GOT_ID" != "$EXPECTED_ID" ]]; then
    echo "peer id mismatch: got $GOT_ID want $EXPECTED_ID" >&2
    exit 1
fi

UNIT=/etc/systemd/system/modal-observer.service
sudo tee "$UNIT" >/dev/null <<EOF
[Unit]
Description=Modality testnet observer
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$(id -un)
Group=$(id -gn)
WorkingDirectory=${DIR}
Environment=HOME=${HOME}
Environment=PATH=${BIN_DIR}:/usr/local/bin:/usr/bin:/bin
ExecStart=${BIN_DIR}/modal node run-observer --dir ${DIR} --no-tui
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now modal-observer.service
systemctl is-active modal-observer.service
echo "Observer explorer at https://node0.testnet.modality.network"
