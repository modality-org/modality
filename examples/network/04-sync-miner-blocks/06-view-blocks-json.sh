#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# View blocks in JSON format (without persistence)
# Useful for inspecting block data or piping to jq

NODE1_PEER_ID="12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"

echo "Fetching blocks in JSON format..."

modality net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/$NODE1_PEER_ID \
  --mode all \
  --format json

echo ""
echo "Tip: Pipe to jq for filtering:"
echo "  $0 | jq '.blocks[] | select(.epoch == 0)'"

