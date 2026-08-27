#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/index.md"
MODAL_MAIN="$ROOT_DIR/rust/modal/src/main.rs"

required_patterns=(
  "# CLI Reference"
  "cargo build --release -p modal --no-default-features --features contract-onboarding"
  "cargo build --release -p modality"
  "The lean onboarding wrapper exposes"
  "\`modal contract\`, \`modal c\`, \`modal id\`"
  "\`modal pull\`, \`modal commit\`, \`modal diff\`"
  "\`modal set\`, \`modal repost\`, \`modal add-rule\`, and \`modal download\`"
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
  "modal pull http://hub.example.com/contracts/my-contract"
  "modal diff"
  "modal repost source-contract-id /source/path /local/path"
  "modal add-rule rules/member-protection.modality"
  "modal download http://hub.example.com/contracts/my-contract.pack"
  "\`modal contract pull\` | \`modal pull\`"
  "\`modal contract commit\` | \`modal commit\`"
  "\`modal contract diff\` | \`modal diff\`"
  "\`modal contract set\` | \`modal set\`"
  "\`modal contract repost\` | \`modal repost\`"
  "\`modal contract add-rule\` | \`modal add-rule\`"
  "\`modal contract download\` | \`modal download\`"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "CLI index is missing onboarding/help-surface text: $pattern" >&2
    exit 1
  fi
done

lean_source_patterns=(
  '#[command(alias = "c")]'
  'Contract {'
  'Id {'
  'Passfile {'
  'Status(modal_cli_contract::status::Opts)'
  'Pull(modal_cli_contract::pull::Opts)'
  'Commit(modal_cli_contract::commit::Opts)'
  'Diff(modal_cli_contract::diff::Opts)'
  'Set(modal_cli_contract::set::Opts)'
  'Repost(modal_cli_contract::repost::Opts)'
  'AddRule(modal_cli_contract::add_rule::Opts)'
  'Download(modal_cli_contract::download::Opts)'
)

for pattern in "${lean_source_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$MODAL_MAIN"; then
    echo "modal source is missing lean CLI surface expected by docs: $pattern" >&2
    exit 1
  fi
done

full_source_patterns=(
  'Node {'
  'Local {'
  'Net {'
  'Hub {'
  'Run {'
  'Predicate {'
  'Program {'
  'Chain {'
  'Killall(modal_cli_node::local::killall_nodes::Opts)'
  'Upgrade(modality::cmds::upgrade::Opts)'
)

for pattern in "${full_source_patterns[@]}"; do
  if ! grep -B4 -F -- "$pattern" "$MODAL_MAIN" | grep -Fq '#[cfg(feature = "full")]'; then
    echo "modal source no longer gates full-only CLI surface as documented: $pattern" >&2
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
