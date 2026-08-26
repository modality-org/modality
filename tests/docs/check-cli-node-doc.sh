#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/node-commands.md"

required_patterns=(
  "# Node Commands (\`modal node\`)"
  "full Rust wrapper"
  "not in the lean first-contract onboarding wrapper"
  "modal node create [OPTIONS]"
  "\`--dir <DIR>\` | Node directory to create"
  "\`--node-id <NODE_ID>\` | Existing peer ID to record"
  "\`--data-dir <DATA_DIR>\` | Data directory written into config"
  "\`--from-config <CONFIG>\` | Merge settings from an existing config file"
  "\`--from-passfile <PASSFILE>\` | Import an existing node identity passfile"
  "\`--from-template <TEMPLATE>\` | Load a bundled template"
  "\`--use-mnemonic\` | Generate or import the node key from a BIP39 mnemonic"
  "node.modal_passfile"
  "modal node start [OPTIONS]"
  "\`--config <CONFIG>\` | Path to \`config.json\`"
  "\`--node-type <TYPE>\` | \`miner\`, \`observer\`, \`validator\`, or \`server\`"
  "modal node stop [OPTIONS]"
  "modal node restart [OPTIONS]"
  "modal node kill [OPTIONS]"
  "modal node pid [OPTIONS]"
  "\`--force\` / \`-f\`"
  "modal node run [OPTIONS]"
  "modal node run-miner [OPTIONS]"
  "modal node run-validator [OPTIONS]"
  "modal node run-observer [OPTIONS]"
  "modal node run-noop [OPTIONS]"
  "\`--enable-consensus\`, which is deprecated"
  "modal run miner --dir ./my-node"
  "modal node info [OPTIONS]"
  "modal node stats [OPTIONS]"
  "\`--sample-recent-blocks <COUNT>\`"
  "modal node address [OPTIONS]"
  "\`--one\` / \`-1\`"
  "\`--prefer-public\`"
  "\`--prefer-local\`"
  "modal node inspect [COMMAND] [KEY|INDEX] [OPTIONS]"
  "\`block <INDEX>\`"
  "\`datastore-get <KEY>\`"
  "\`--level <LEVEL>\`"
  "modal node compare <PEER> [OPTIONS]"
  "\`--timeout-secs <SECS>\`"
  "\`--precise\`"
  "modal node logs [OPTIONS]"
  "\`--lines <COUNT>\` / \`-n <COUNT>\`"
  "\`--offline\`"
  "modal node ping --target <MULTIADDR> [OPTIONS]"
  "\`--target <MULTIADDR>\` | Peer multiaddr to ping"
  "\`--times <COUNT>\`"
  "modal node sync [OPTIONS]"
  "\`--block-height-minus <COUNT>\`"
  "\`--max-peers <COUNT>\`"
  "modal node config [OPTIONS]"
  "\`--show\` | Show current configuration"
  "\`--set-listeners <ADDRS>\`"
  "\`--add-listener <ADDR>\`"
  "\`--set-bootstrappers <ADDRS>\`"
  "\`--add-bootstrapper <ADDR>\`"
  "\`--merge-in <FILE>\`"
  "\`--dry-run\`"
  "modal node clear [OPTIONS]"
  "modal node clear-storage [OPTIONS]"
  "\`--yes\` / \`-y\`"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "node command reference is missing current full-wrapper text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "\`--path <PATH>\`" \
  "node.passfile" \
  "modal node ping <PEER>" \
  "\`--peer <ADDR>\`" \
  "\`--from <HEIGHT>\`" \
  "\`--threads <N>\`" \
  "\`--chain\` | Show chain info" \
  "\`--peers\` | Show peer info" \
  "\`--config\` | Show configuration" \
  "\`--get <KEY>\`" \
  "\`--set <KEY=VALUE>\`" \
  "\`--confirm\`" \
  "modal hub start [OPTIONS]"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "node command reference still contains stale full-wrapper text: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "node command doc check passed"
