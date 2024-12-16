#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modality net ping \
  --config ../../../fixtures/network-node-configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN \
  --times 100