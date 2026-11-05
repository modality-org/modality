#!/usr/bin/env bash
set -euo pipefail

echo "Verifying parameter values..."

python3 - <<'PYVERIFY'
import json
from pathlib import Path

config = json.loads(Path("../../../fixtures/network-configs/devnet1/config.json").read_text())
results = Path("./tmp/test-network-params")
expected = {
    "name.txt": config["name"],
    "difficulty.txt": str(config["latest_parameters"]["difficulty"]),
    "sequencer0.txt": config["latest_parameters"]["sequencers"][0],
}
for filename, value in expected.items():
    actual = (results / filename).read_text().strip()
    if actual != value:
        raise SystemExit(f"{filename}: expected {value!r}, got {actual!r}")
    print(f"  ✓ {filename}: {actual}")
PYVERIFY

echo "✓ All parameter values verified"
