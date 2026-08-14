#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/getting-started/installation.md"

required_patterns=(
  "# Installation"
  "cargo build --release -p modal --no-default-features --features contract-onboarding"
  "modal --help"
  "modality model --help"
  "Use \`modal\` for contract logs, identities, commits, status, and the first-contract"
  "Use \`modality\` for model and rule authoring tasks"
  "\`modality model synthesize\`"
  "\`modality model validate\`"
  "\`modality model lint\`"
  "A successful onboarding install should make both command surfaces visible"
  "External crates.io-style packaging is tracked separately"
  "tests/cli/check-modal-package-readiness.sh"
  "workspace CLI crates that \`modal\` depends on are available from the"
  "registry"
  "For the language CLI, \`modality --help\` should only expose the model command"
  "group, and \`modality model --help\` should expose the parser and review tools"
  "check      Check a formula against a model"
  "synthesize Synthesize a model from a template"
  "validate   Validate a contract model"
  "lint       Lint governance formulas"
  "You should not see the full runtime command groups"
  "hub"
  "node"
  "predicate"
  "program"
  "chain"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "installation guide is missing onboarding CLI split text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- '(^|[^[:alnum:]_])(->|implies)([^[:alnum:]_]|$)' "$DOC"; then
  echo "installation guide should avoid implication sugar" >&2
  exit 1
fi

echo "installation doc check passed"
