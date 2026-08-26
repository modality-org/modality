#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/network-commands.md"

required_patterns=(
  "# Network Commands (\`modal net\` / \`modal network\`)"
  "full Rust wrapper"
  "modal net info [NETWORK]"
  "\`NETWORK\` | Network name"
  "modal net storage --config <CONFIG> [OPTIONS]"
  "\`--config <CONFIG>\` | Path to the node configuration file"
  "\`--detailed\` | Show a detailed block list"
  "\`--epoch <EPOCH>\` | Filter blocks by epoch"
  "\`--limit <LIMIT>\` | Maximum detailed blocks to show"
  "modal net mining sync --config <CONFIG> --target <MULTIADDR> [OPTIONS]"
  "\`--target <MULTIADDR>\` | Source node multiaddress"
  "\`--mode <MODE>\` | Sync mode: \`all\`, \`epoch\`, or \`range\`"
  "\`--from-index <INDEX>\` | Start block index"
  "\`--to-index <INDEX>\` | End block index"
  "\`--format <FORMAT>\` | Output format: \`summary\` or \`json\`"
  "\`--persist\` | Persist synced blocks"
  "modal local nodes [OPTIONS]"
  "\`--verbose\` / \`-v\` | Show verbose output with full paths"
  "\`--network <FILTER>\` | Filter by network config path"
  "\`--devnet\` | Shorthand for \`--network devnet*\`"
  "\`--dir <DIR>\` | Only show nodes"
  "modal local killall-nodes [OPTIONS]"
  "\`--force\` / \`-f\` | Use \`SIGKILL\`"
  "\`--dry-run\` | Show what would be killed"
  "modal chain validate [OPTIONS]"
  "\`--test <TEST>\` / \`-t <TEST>\`"
  "\`--datastore <PATH>\` / \`-d <PATH>\`"
  "\`--json\` | Output results as JSON"
  "modal chain heal --datastore <PATH> [OPTIONS]"
  "modal run miner --dir ./my-node"
  "modal run validator --dir ./my-node"
  "modal run observer --dir ./my-node"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "network command reference is missing current full-wrapper text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "modal net info [OPTIONS]" \
  "\`--network <NAME>\` | Network name" \
  "\`--peer <ADDR>\`" \
  "\`--node <PATH>\`" \
  "\`--verbose\` | Show detailed breakdown" \
  "\`--from <PEER>\`" \
  "\`--from <HEIGHT>\`" \
  "\`--to <HEIGHT>\`" \
  "\`--backup\`" \
  "modal run miner --path" \
  "modal run validator --path" \
  "modal run observer --path"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "network command reference still contains stale full-wrapper text: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "network command doc check passed"
