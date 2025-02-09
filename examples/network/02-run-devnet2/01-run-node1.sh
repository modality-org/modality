#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modality-js net run-node --config ../../../fixtures/network-node-configs/devnet2/node1.json --enable_consensus