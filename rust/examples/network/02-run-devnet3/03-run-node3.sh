#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modality net run-node --config ../../../fixtures/network-node-configs/devnet3/node3.json