#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modality net run-node --config ../../../fixtures/network-node-configs/node1.json