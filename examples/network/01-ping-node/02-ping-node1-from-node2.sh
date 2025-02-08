#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

modality net ping \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd \
  --times 100