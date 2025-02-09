#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modality net run-node --config ../../../fixtures/network-node-configs/devnet2/node2.json --enable-consensus