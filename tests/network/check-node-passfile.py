#!/usr/bin/env python3
"""Check that a created node uses its generated passfile, not a stale path."""

import json
import sys
from pathlib import Path


def main(node_dir: Path) -> None:
    config = json.loads((node_dir / "config.json").read_text())
    configured = config.get("passfile_path")
    if not isinstance(configured, str) or not configured:
        raise ValueError("config has no passfile_path")
    expected = node_dir / "node.modal_passfile"
    actual = Path(configured)
    if not actual.is_absolute():
        actual = node_dir / actual
    if actual.resolve() != expected.resolve():
        raise ValueError(f"passfile_path resolves to {actual}, expected {expected}")
    if not actual.is_file():
        raise ValueError(f"configured passfile does not exist: {actual}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: check-node-passfile.py NODE_DIR")
    main(Path(sys.argv[1]))
