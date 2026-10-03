#!/usr/bin/env python3
"""Fail if the repo holds a private key a live network trusts.

Live networks (testnet, mainnet) name peer ids as bootstrappers and
validators. Their keys live only on the hosts that run them. This check
fails when any tracked file holds a passfile whose id is one of those, and
when a passfile appears outside the paths that hold shared local-devnet and
test keys.
"""
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
LIVE = ["testnet", "mainnet"]
SHARED_KEY_PATHS = (
    "rust/modality-networks/templates/devnet",
    "rust/modality-node/fixtures/",
    "js/packages/",
    "fixtures/passfiles/",
    "rust/modality-devnet/",
    "tests/",
)
PEER_ID = re.compile(r"/p2p/([1-9A-HJ-NP-Za-km-z]+)")


def live_ids():
    ids = {}
    for name in LIVE:
        info = ROOT / "rust/modality-networks/networks" / name / "info.json"
        if not info.exists():
            continue
        data = json.loads(info.read_text())
        for addr in data.get("bootstrappers", []):
            for peer in PEER_ID.findall(addr):
                ids[peer] = f"{name} bootstrapper"
        for peer in data.get("validators", []) or []:
            ids[peer] = f"{name} validator"
    return ids


def tracked():
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True)
    return out.stdout.split()


def main():
    ids = live_ids()
    problems = []
    for rel in tracked():
        path = ROOT / rel
        if not path.is_file() or path.stat().st_size > 2_000_000:
            continue
        try:
            text = path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        if '"private_key"' not in text:
            continue
        for peer, role in ids.items():
            if peer in text:
                problems.append(f"{rel}: holds a private key alongside the {role} {peer}")
        if "passfile" in pathlib.PurePath(rel).name and not rel.startswith(SHARED_KEY_PATHS):
            problems.append(f"{rel}: a passfile outside the shared test-key paths")
    if problems:
        print("Private keys a live network trusts must not be in the repo:")
        for p in problems:
            print("  " + p)
        sys.exit(1)
    print(f"No live-network keys in the repo ({len(ids)} live ids checked).")


if __name__ == "__main__":
    main()
