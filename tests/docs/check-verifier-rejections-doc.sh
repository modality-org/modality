#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/reference/verifier-rejections.md"
LOCAL_GOVERNANCE="$ROOT_DIR/rust/modal-cli-contract/src/model_governance.rs"
HUB_VALIDATOR="$ROOT_DIR/rust/modal-cli-hub/src/model_validator.rs"
COMMON_DIAGNOSTICS="$ROOT_DIR/rust/modal-common/src/model_diagnostics.rs"

required_patterns=(
  "# Verifier Rejection Explanations"
  "explain that rejection from the same model"
  "current governing-model state"
  "closest candidate transition"
  "predicates that failed"
  "Similar transitions from other states"
  'current states {"q1"}'
  "Candidate transitions: none from current states"
  "non-current transition from q1 to q2 [+POST]; current states: q0; failed predicates: none"
  "still be unavailable"
  "current witness state"
  "missing +signed_by(/parties/alice.id)"
  "missing +signed_by(/parties/bob.id)"
  'missing +threshold("2", /treasury/signers)'
  "forbidden -modifies(/members) matched"
  "tests/cli/run-first-contract-cli-smoke.sh"
  "explains_similar_transitions_when_current_state_has_no_candidates"
  "explains_signed_by_identity_bootstrap_ordering"
  "explains_action_modal_rule_failure_with_transition_witness"
  "explains_lfp_rule_failure_with_unfolding_witness_set"
  "test_action_rejection_explains_candidate_transition_predicates"
  "test_action_rejection_ranks_closest_candidate_by_failed_predicates"
  "test_action_rejection_explains_similar_non_current_transitions"
  "test_model_replacement_rule_rejection_explains_formula_failure"
  "test_model_replacement_rule_rejection_explains_action_modal_witness"
  "test_model_replacement_rule_rejection_explains_fixed_point_unfolding"
  'Shared `modal-common::model_diagnostics` formatter regressions'
  "summarizes_candidate_transition_with_stable_key_and_failures"
  "summarizes_non_current_transition_with_current_states"
  "renders_recursive_formula_failure_diagnostic"
  "renders_action_modal_transition_witness_diagnostic"
  "renders_least_fixed_point_unfolding_diagnostic"
  "shared formatter drift"
  "Model-Replacement Rule Failures"
  "failed anchor state"
  "satisfying states in the candidate model"
  "recursive formula counterexample"
  "Action-modal witnesses"
  "Fixed-point unfolding witnesses"
  "fixed-point witness set"
  "model-checking counterexample"
  "They do not prove that an"
  "external party should have signed"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "verifier rejection doc is missing: $pattern" >&2
    exit 1
  fi
done

local_regressions=(
  "explains_similar_transitions_when_current_state_has_no_candidates"
  "explains_signed_by_identity_bootstrap_ordering"
  "explains_action_modal_rule_failure_with_transition_witness"
  "explains_lfp_rule_failure_with_unfolding_witness_set"
)

for regression in "${local_regressions[@]}"; do
  if ! grep -Fq -- "fn $regression" "$LOCAL_GOVERNANCE"; then
    echo "verifier rejection local regression is missing from source: $regression" >&2
    exit 1
  fi
done

hub_regressions=(
  "test_action_rejection_explains_candidate_transition_predicates"
  "test_action_rejection_ranks_closest_candidate_by_failed_predicates"
  "test_action_rejection_explains_similar_non_current_transitions"
  "test_model_replacement_rule_rejection_explains_formula_failure"
  "test_model_replacement_rule_rejection_explains_action_modal_witness"
  "test_model_replacement_rule_rejection_explains_fixed_point_unfolding"
)

for regression in "${hub_regressions[@]}"; do
  if ! grep -Fq -- "fn $regression" "$HUB_VALIDATOR"; then
    echo "verifier rejection hub regression is missing from source: $regression" >&2
    exit 1
  fi
done

common_regressions=(
  "summarizes_candidate_transition_with_stable_key_and_failures"
  "summarizes_non_current_transition_with_current_states"
  "renders_recursive_formula_failure_diagnostic"
  "renders_action_modal_transition_witness_diagnostic"
  "renders_least_fixed_point_unfolding_diagnostic"
)

for regression in "${common_regressions[@]}"; do
  if ! grep -Fq -- "fn $regression" "$COMMON_DIAGNOSTICS"; then
    echo "verifier rejection shared diagnostic regression is missing from source: $regression" >&2
    exit 1
  fi
done

if grep -Eq -- '(^|[^[:alnum:]_])(->|implies)([^[:alnum:]_]|$)' "$DOC"; then
  echo "verifier rejection doc should avoid implication sugar" >&2
  exit 1
fi

echo "verifier rejection doc check passed"
