#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEST_README="$ROOT_DIR/tests/README.md"
CLI_README="$ROOT_DIR/tests/cli/README.md"
ONBOARDING_SMOKE="$ROOT_DIR/tests/run-onboarding-smokes.sh"
FIRST_CONTRACT_SMOKE="$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
EVOLUTION_SMOKE="$ROOT_DIR/tests/cli/run-contract-evolution-cli-smoke.sh"

required_test_readme_patterns=(
  "## Runtime Workflow Checkpoint"
  "The local runtime checkpoint is intentionally file-backed and offline:"
  "It is the burn-down evidence for the runtime-workflow path"
  "network demos are treated as required."
  "create a contract"
  "install"
  "named identity evidence"
  "synthesize and validate the governing witness"
  "commit"
  "accepted artifacts"
  "replay accepted state with checkout"
  "inspect status and"
  "log output"
  "reject unsigned,"
  "wrong-signer, wrong-state, and wrong-action successors with verifier"
  "explanations"
  "accept a signed witness replacement"
  "prove additive rule"
  "evolution plus bounded replacement behavior"
  "Hub, node, network, predicate, program,"
  "surfaces are outside this local checkpoint unless"
  "MODAL_RUNTIME_WORKFLOW_CHECK=1"
  'missing `modality` language CLI'
  '`modal` wrapper is a failure instead of a skipped first-contract and'
  "contract-evolution replay."
  "MODAL_RUNTIME_WORKFLOW_CHECK=1 MODALITY_ONBOARDING_BUILD=1 MODAL_ONBOARDING_BUILD=1 tests/run-onboarding-smokes.sh"
  "tests/cli/run-first-contract-cli-smoke.sh"
  "tests/cli/run-contract-evolution-cli-smoke.sh"
  "Before the direct"
  "first-contract smoke runs any contract commands"
  "swapped helper binary cannot anchor"
)

for pattern in "${required_test_readme_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$TEST_README"; then
    echo "tests README is missing runtime workflow checkpoint text: $pattern" >&2
    exit 1
  fi
done

required_cli_readme_patterns=(
  "The first-contract and contract-evolution smokes are the canonical local"
  "runtime workflow checkpoint."
  "They cover create, identity setup,"
  "Before contract setup starts"
  "modal --version"
  "modality --version"
  "swapped binaries"
  "checkout replay"
  "synthesis-backed witness validation, commit,"
  "status, log,"
  "rejection explanation, witness replacement, and accumulated-rule evolution"
  "requiring hub or network services."
  "Use the separate hub/network"
  "examples only"
  "when testing remote push/pull or validator behavior."
)

for pattern in "${required_cli_readme_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$CLI_README"; then
    echo "CLI smoke README is missing runtime workflow checkpoint text: $pattern" >&2
    exit 1
  fi
done

required_smoke_patterns=(
  'tests/cli/run-first-contract-cli-smoke.sh'
  'tests/cli/run-contract-evolution-cli-smoke.sh'
  'MODAL_ONBOARDING_FEATURES="${MODAL_ONBOARDING_FEATURES:-contract-onboarding}"'
  'MODAL_HELP_SURFACE="${MODAL_HELP_SURFACE:-$DEFAULT_MODAL_HELP_SURFACE}"'
  'MODAL_BIN="$MODAL_BIN" "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"'
  'MODAL_BIN="$MODAL_BIN" "$ROOT_DIR/tests/cli/run-contract-evolution-cli-smoke.sh"'
  'MODAL_RUNTIME_WORKFLOW_CHECK=1'
  'runtime workflow checkpoint requested, but modality binary not found at $MODALITY_BIN'
  'runtime workflow checkpoint requested, but modal binary not found at $MODAL_BIN'
  'MODAL_RUNTIME_WORKFLOW_CHECK=1 MODALITY_ONBOARDING_BUILD=1 MODAL_ONBOARDING_BUILD=1 $0'
  'MODAL_ONBOARDING_FEATURES=full'
)

for pattern in "${required_smoke_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$ONBOARDING_SMOKE" "$TEST_README"; then
    echo "onboarding smoke or tests README is missing runtime workflow wiring: $pattern" >&2
    exit 1
  fi
done

required_first_contract_patterns=(
  'contract create'
  'capture_version_line "$MODAL_BIN" modal modal'
  'capture_version_line "$MODALITY_BIN" modality modality'
  'set-named-id /parties/alice.id'
  'model synthesize'
  'model validate'
  'status --dir "$CONTRACT_DIR"'
  'log --dir "$CONTRACT_DIR"'
  'checkout --dir "$CONTRACT_DIR"'
  'expected unsigned post-bootstrap commit to fail'
  'expected wrong-state post to fail with no current transition candidates'
  'expected wrong-action post to fail with a closer non-current transition'
  'Let Bob replace the witness'
)

for pattern in "${required_first_contract_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$FIRST_CONTRACT_SMOKE"; then
    echo "first-contract smoke is missing local runtime workflow assertion: $pattern" >&2
    exit 1
  fi
done

required_evolution_patterns=(
  'contract create'
  'c commit \'
  'Add signed-post rule'
  'expected unsigned replacement model to fail'
  'Accept Bob in V2'
  'expected unsigned V2 post to fail'
  'Model state: active'
  'Model state: expired'
)

for pattern in "${required_evolution_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$EVOLUTION_SMOKE"; then
    echo "contract-evolution smoke is missing runtime evolution assertion: $pattern" >&2
    exit 1
  fi
done

echo "runtime workflow doc check passed"
