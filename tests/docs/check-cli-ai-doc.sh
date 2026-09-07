#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/ai-commands.md"
MODAL_MAIN="$ROOT_DIR/rust/modal/src/main.rs"
AI_LIB="$ROOT_DIR/rust/modal-cli-ai/src/lib.rs"
SET_SOURCE="$ROOT_DIR/rust/modal-cli-ai/src/set.rs"
SHOW_SOURCE="$ROOT_DIR/rust/modal-cli-ai/src/show.rs"
UNSET_SOURCE="$ROOT_DIR/rust/modal-cli-ai/src/unset.rs"
AI_SOURCE="$ROOT_DIR/rust/modal-cli-contract/src/ai.rs"

required_patterns=(
  "# AI Commands (\`modal ai\`)"
  "modal ai set --provider openai|anthropic|grok|bedrock|ollama"
  "\`--provider <PROVIDER>\`"
  "\`--model <MODEL>\`"
  "\`--base-url <BASE_URL>\`"
  "\`--region <REGION>\`"
  "\`--api-key <API_KEY>\`"
  "\`--save-key\`"
  "modal ai set --provider openai"
  "modal ai set --provider anthropic"
  "modal ai set --provider grok"
  "modal ai set --provider bedrock --region us-east-1"
  "modal ai set --provider ollama"
  "modal ai show"
  "modal ai unset"
  "modal ai suggest-rule <PROMPT>"
  "modal ai suggest-rule \"after this commit either alice or bob must sign\""
  "yours may differ"
  "MODAL_AI_API_KEY"
  "OPENAI_API_KEY"
  "ANTHROPIC_API_KEY"
  "XAI_API_KEY"
  "Do not"
  "persist AWS secrets in \`ai.json\`"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "AI command reference is missing current help-surface text: $pattern" >&2
    exit 1
  fi
done

if ! grep -Fq -- 'modal_cli_ai::Commands' "$MODAL_MAIN"; then
  echo "modal wrapper no longer wires documented modal ai command group" >&2
  exit 1
fi

for source_guard in \
  'Set(set::Opts)' \
  'Show(show::Opts)' \
  'Unset(unset::Opts)'; do
  if ! grep -Fq -- "$source_guard" "$AI_LIB"; then
    echo "modal-cli-ai no longer exposes documented command: $source_guard" >&2
    exit 1
  fi
done

for source_guard in \
  'pub provider: Provider' \
  'pub model: Option<String>' \
  'pub base_url: Option<String>' \
  'pub region: Option<String>' \
  'pub api_key: Option<String>' \
  'pub save_key: bool'; do
  if ! grep -Fq -- "$source_guard" "$SET_SOURCE"; then
    echo "modal ai set source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
done

if ! grep -Fq -- 'format_show' "$SHOW_SOURCE"; then
  echo "modal ai show source no longer redacts configured keys" >&2
  exit 1
fi

if ! grep -Fq -- 'config::unset' "$UNSET_SOURCE"; then
  echo "modal ai unset source no longer removes ai.json" >&2
  exit 1
fi

for source_guard in \
  'prompt: String' \
  '#[command(name = "suggest-rule")]' \
  'modal_cli_ai::suggest_rule'; do
  if ! grep -Fq -- "$source_guard" "$AI_SOURCE"; then
    echo "contract ai source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
done

echo "AI command doc check passed"
