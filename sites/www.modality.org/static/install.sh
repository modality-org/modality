#!/bin/sh
# Current `modal` CLI (contract, identity, node) is published from the
# testnet package feed. This wrapper keeps the historical
# https://www.modality.org/install.sh URL working.

set -e

echo "Installing the current modal CLI from the testnet package feed..."
echo "This is not mainnet."
curl -fsSL https://get.modality.org/testnet/latest/install.sh | sh
