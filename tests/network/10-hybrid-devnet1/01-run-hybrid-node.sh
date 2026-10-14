#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

modal node create --dir ./tmp/node1
python3 - <<'PYCONFIG'
import json
from pathlib import Path

node = Path("tmp/node1")
network = {
    "config_version": 2,
    "name": "devnet1-hybrid-smoke",
    "bootstrappers": [],
    "blocks_per_epoch": 2,
    "initial_difficulty": 1,
    "target_block_time_secs": 1,
}
(node / "network.json").write_text(json.dumps(network, indent=2) + "\n")
config_path = node / "config.json"
config = json.loads(config_path.read_text())
config.update(
    network_config_path="./network.json",
    bootstrappers=[],
    listeners=["/ip4/127.0.0.1/tcp/10111/ws"],
    status_port=3111,
    initial_difficulty=1,
    miner_hash_func="sha256",
)
config_path.write_text(json.dumps(config, indent=2) + "\n")
PYCONFIG

exec modal node run-hybrid --dir ./tmp/node1
