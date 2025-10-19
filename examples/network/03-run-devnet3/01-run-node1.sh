#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modality node run --config ../../../fixtures/network-node-configs/devnet3/node1.json