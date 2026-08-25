#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/index.md"

required_patterns=(
  "# CLI Reference"
  "cargo build --release -p modal --no-default-features --features contract-onboarding"
  "cargo build --release -p modality"
  "The lean onboarding wrapper exposes"
  "\`modal contract\`, \`modal c\`, \`modal id\`"
  "It omits the"
  "runtime-heavy hub, node, network, predicate, program, chain, local, run,"
  "\`killall\`, and upgrade surfaces"
  "Build the full wrapper with \`cargo build --release -p modal\`"
  "Use \`modality\` for model and rule authoring tasks"
  "\`modality model lint\`"
  "\`modality model synthesize\`"
  "\`modality model validate\`"
  "full wrapper only"
  "modal c create"
  "modal c set-named-id /parties/alice.id alice.passfile"
  "modal c commit --all --sign alice.passfile"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "CLI index is missing onboarding/help-surface text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "curl -fsSL https://get.modality.org | sh" \
  "cd rust && cargo build --release" \
  "modal c set /parties/alice.id --named alice"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "CLI index still contains stale install text: $forbidden_pattern" >&2
    exit 1
  fi
done

echo "CLI index doc check passed"
