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
  "Git URL installs are measured by"
  "tests/cli/check-modal-git-install-readiness.sh"
  "installs \`modal\` into a"
  "temporary Cargo root"
  "runs the"
  "first-contract CLI smoke when a built \`modality\` binary is supplied"
  "MODAL_ONBOARDING_GIT_REV=<commit>"
  "pin the exact Git revision"
  "Release-archive-shaped binary bundles are measured by"
  "tests/cli/check-modal-release-archive-readiness.sh"
  "modal-<version>-<os>-<arch>-<profile>.tar.gz"
  "containing \`bin/modal\` and"
  "\`README.txt\`"
  "plus \`SHA256SUMS\`"
  "unpacks it"
  "verifies the checksum manifest"
  "manifest covers exactly \`bin/modal\` and \`README.txt\`"
  "checks the unpacked help surface"
  "External crates.io-style packaging is tracked separately"
  "tests/cli/check-modal-package-readiness.sh"
  "workspace CLI crates that \`modal\` depends on are available from the"
  "registry"
  "selected direct workspace dependencies"
  "\`modal-cli-contract\`, \`modal-common\`, and \`modality\`"
  "selected package"
  "\`modal-cli-common\`, \`modal-cli-contract\`, \`modal-common\`, \`modality\`"
  "and \`modality-lang\`"
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
