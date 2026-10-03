#!/usr/bin/env python3
"""Write manifest.json for a release directory: the version, the source
commit, and each binary with its sha256. The release contract is what makes
a release trusted; the manifest says where things are.

usage: write-manifest.py BUILD_DIR VERSION CHANNEL GIT_COMMIT
"""
import datetime
import hashlib
import json
import pathlib
import sys

build, version, channel, commit = pathlib.Path(sys.argv[1]), *sys.argv[2:5]
platforms = {
    "linux-x86_64": ("linux", "x86_64"),
    "darwin-aarch64": ("darwin", "aarch64"),
}
binaries = {}
for name, (os_name, arch) in platforms.items():
    path = build / "binaries" / name / "modal"
    if path.exists():
        binaries[name] = {
            "name": "modal",
            "path": f"binaries/{name}/modal",
            "platform": os_name,
            "arch": arch,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }
manifest = {
    "version": version,
    "timestamp": datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%d_%H%M%S"),
    "git_branch": channel,
    "git_commit": commit,
    "release_log": f"https://get.modality.org/{channel}/release-contract/log.json",
    "packages": {"binaries": binaries},
}
(build / "manifest.json").write_text(json.dumps(manifest, indent=4) + "\n")
print(f"manifest.json: {version}, {len(binaries)} binaries")
