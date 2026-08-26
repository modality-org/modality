#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/hub-commands.md"

required_patterns=(
  "# Hub Commands (\`modal hub\`)"
  "full Rust wrapper"
  "not in the lean first-contract onboarding wrapper"
  "The current CLI surface exposes server startup only"
  "modal hub start [OPTIONS]"
  "\`--host <HOST>\` | Bind address"
  "\`--port <PORT>\` | REST API port"
  "\`--rpc-port <RPC_PORT>\` | JSON-RPC port"
  "\`--data-dir <DATA_DIR>\` | Data directory for stored contracts"
  "\`--cors <BOOL>\` | Enable browser CORS headers"
  "modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0"
  "\`/health\` | \`GET\` | Health check"
  "\`/contracts\` | \`POST\` | Create a contract"
  "\`/contracts/synthesize\` | \`POST\` | Synthesize a contract draft"
  "\`/contracts/:id/log\` | \`GET\` | Get the commit log"
  "\`/contracts/:id/commits\` | \`POST\` | Submit a commit"
  "\`/templates\` | \`GET\` | List built-in templates"
  "modal c remote add origin http://127.0.0.1:8080/contracts/<contract-id>"
  "\`--hub-creds <HUB_CREDS>\`"
  "command group does not create that credentials"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "hub command reference is missing current full-wrapper text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "\`--detach\`" \
  "\`--data <PATH>\`" \
  "modal hub stop" \
  "modal hub status" \
  "modal hub register [OPTIONS]" \
  "modal hub create <NAME>" \
  "modal hub grant <CONTRACT_ID>" \
  "modal hub revoke <CONTRACT_ID>" \
  "modal hub list [OPTIONS]" \
  "modal hub info <CONTRACT_ID>" \
  "modal hub auth" \
  "\`MODAL_HUB_URL\`" \
  "\`MODAL_PASSFILE\`"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "hub command reference still contains stale full-wrapper text: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "hub command doc check passed"
