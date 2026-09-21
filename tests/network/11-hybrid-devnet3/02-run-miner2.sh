#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

if [ ! -f "./tmp/node2/config.json" ]; then
    echo "Creating miner2 for devnet3-hybrid..."
    modal node create --dir "${SCRIPT_DIR}/tmp/node2" --from-template devnet3/node2

    python3 - "${SCRIPT_DIR}/tmp/node2/config.json" << 'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    c = json.load(f)
c["passfile_path"] = "./node.modal_passfile"
c["data_dir"] = "./data"
c["listeners"] = ["/ip4/0.0.0.0/tcp/10312/ws"]
c["bootstrappers"] = [
    "/ip4/127.0.0.1/tcp/10311/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    "/ip4/127.0.0.1/tcp/10313/ws/p2p/12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se",
]
c["network_config_path"] = "modality-networks://devnet3-hybrid"
c["run_miner"] = True
c["miner_nominees"] = [
    "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
    "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se",
]
c["hybrid_consensus"] = True
c["run_validator"] = True
c["initial_difficulty"] = 1
c["status_port"] = 3312
with open(path, "w") as f:
    json.dump(c, f, indent=2)
    f.write("\n")
PY
fi

modal node clear-storage --dir ./tmp/node2 --yes

echo "Starting miner2 (hybrid node)..."
modal node run-hybrid --dir ./tmp/node2 --no-tui
