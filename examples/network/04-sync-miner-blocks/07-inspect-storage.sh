#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Inspect node1's datastore and show miner block statistics

echo "Inspecting node1's datastore..."
echo ""

modality net storage --config ./configs/node1.json

