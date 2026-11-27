---
sidebar_position: 7
title: Observe the public testnet
---

# Observe the public testnet

Run your own read-only observer to compare the public testnet's miner chain and
sequenced contract history with the Foundation explorer. This is a **testnet**
node, not a validator or a mining pool. A reachable bootstrapper is required;
see [Join the public testnet](join-testnet.md) for the current network and
installation notes.

## Start a fresh observer

```bash
modal node create --dir ./my-observer --testnet
python3 - <<'PY'
import json
from pathlib import Path

path = Path("my-observer/config.json")
config = json.loads(path.read_text())
config["status_port"] = 1338  # choose an unused local port
path.write_text(json.dumps(config, indent=2) + "\n")
PY
modal node run-observer --dir ./my-observer --no-tui
```

`run-observer` watches and catches up without mining, even though `--testnet`
sets a miner default in the generated config. Use a fresh directory and key;
do not use the Foundation `testnet/node0` template. Keep the passfile private.
The status server binds all interfaces on `status_port`; firewall that port if
only local inspection is intended. TCP 4040 is the node's peer listener.

## Inspect what this node actually has

In another terminal, after the observer has connected:

```bash
curl -fsS http://127.0.0.1:1338/status.json | python3 -m json.tool
curl -fsS http://127.0.0.1:1338/chain.json | python3 -m json.tool
curl -fsS http://127.0.0.1:1338/api/contracts | python3 -m json.tool
```

Open `http://127.0.0.1:1338/` for the status page and contract explorer. For a
specific contract ID, inspect `/api/contracts/<contract-id>` and
`/api/contracts/<contract-id>/commits`. Compare the same ID on a second
observer (for example, the public node0 explorer), allowing for catch-up lag.
An empty contract list or an older tip is not proof of agreement; first check
that the observer has connected and caught up. `status.json` and `chain.json`
report this node's local view, not an independently certified network verdict.

The local [observer catch-up smoke](../../tests/network/observer-catchup/test.sh)
starts an observer after a contract is sequenced and checks that its explorer
serves the received balance and commit history. It does not prove current public
network health or independent operator participation.
