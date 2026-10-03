---
sidebar_position: 9
title: Release Commands
---

# Release Commands (`modal release`)

A `modal` package is trusted only when its channel's **release contract**
accepts the commit that names it. `modal` pins that contract (its id and
genesis commit) at build time. To accept a release it fetches the contract's
log and replays it from the pinned genesis: every commit id must hash to
its body and head, every commit must follow the one before it, every
signature must verify, and every commit must meet the contract's model and
rules. Then the file's sha256 must be the one the release names.

The contract's rules decide who may sign a release:

```modality
// Only the release key posts releases
always([+modifies(/releases) -signed_by(/keys/ci.id)] false)

// Changing the release key, the maintainers, the model or the rules takes every maintainer
always([+modifies(/keys) -all_signed(/maintainers)] false)
always([+modifies(/maintainers) -all_signed(/maintainers)] false)
always([+modifies(/rules) -all_signed(/maintainers)] false)
always([+modifies(/model) -all_signed(/maintainers)] false)
```

The release key lives only in CI. If it is lost or exposed, the maintainers
replace `/keys/ci.id` with a commit; no binary changes, and releases the
old key signs afterwards are refused.

A release posts `/releases/<channel>/<version>.json` (the version, the
source commit, and each file's sha256) and `/releases/<channel>/latest.text`.
The log is published at `https://get.modality.org/<channel>/release-contract/log.json`.

## Where it is checked

- `modal upgrade` and node autoupgrade refuse a binary the release contract
  does not accept.
- `install.sh` checks the binary against the release's `SHA256SUMS`, then
  has it check the release contract. It is served from the same place as
  the binary, so for an independent check, build `modal` from source and
  run `modal release verify`.
- Releases are built by the Release workflow from a `testnet-*` tag whose
  commit passes CI, and published after the same check.

## Verify

```bash
modal release verify --log https://get.modality.org/testnet/release-contract/log.json \
  --channel testnet --file binaries/darwin-aarch64/modal=$(which modal)
```

| Option | Description |
|--------|-------------|
| `--log <PATH or URL>` | The release log |
| `--channel <CHANNEL>` | `testnet` |
| `--version <VERSION>` | The release to check; default: the channel's latest |
| `--file <PATH>=<LOCAL>` | A package file and the local copy to check. Repeat |
| `--pin <CONTRACT>:<GENESIS>` | Trust this contract instead of the one this build pins |

## Maintain

| Command | What it does |
|---------|--------------|
| `modal release init --dir <DIR> --maintainer <PASSFILE> --ci <ID>` | Make a release contract |
| `modal release publish --log <IN> --out <OUT> --channel <C> --version <V> --git-commit <SHA> --file <PATH>=<LOCAL> --sign <PASSFILE>` | Append a release (CI) |
| `modal release export --dir <DIR> --out <LOG>` / `import --log <LOG> --dir <DIR>` | Move a log between a file and a contract directory |
