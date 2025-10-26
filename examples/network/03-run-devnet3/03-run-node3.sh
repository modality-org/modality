#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modal node run --config ../../../fixtures/network-node-configs/devnet3/node3.json