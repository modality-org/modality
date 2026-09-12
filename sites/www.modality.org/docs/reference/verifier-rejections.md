# Verifier Rejection Explanations

Runtime verification should reject a commit from the accepted governing model and
then explain that rejection from the same model. Synthesis can help authors find
a witness model, but it is not part of commit-time acceptance.

## What A Good Rejection Shows

When no transition matches the pending commit, the explanation should include:

- The current governing-model state reached by replaying accepted commits, with
  deterministic ordering when replay leaves multiple possible current states.
  Duplicate replay states should not duplicate candidate transition lines.
  Duplicate identical transition inputs should not duplicate explanation lines.
- The closest candidate transition from that current state.
- The predicates that failed on that candidate.
- Other candidate transitions from the current state, ranked behind the closest
  candidate.
- Similar transitions from other states when the current state has no candidate
  for the pending action.
- Similar transitions from other states with fewer failed predicates when the
  current state has only unrelated candidates.

For the first-contract path, an unsigned steady-state update after bootstrap
should fail at `q1`. The useful rejection is not just "commit rejected"; it
points at the accepted Alice-only steady-state witness transition and reports
the missing signature evidence:

```text
current states {"q1"}
Closest candidate transition: candidate from current state q1: q1 to q1 [+signed_by(/parties/alice.id)]; failed predicates: missing +signed_by(/parties/alice.id)
```

That tells the user where replay landed and what evidence would have made the
commit valid, without suggesting Bob is authorized before the later model
evolution installs a Bob-signed transition.

When replay lands in a state with no matching action at all, the useful
fallback is a ranked list of similar transitions from other states. For example,
if replay is at `q0` but a pending `POST` only resembles transitions leaving
`q1`, the explanation should make the state mismatch explicit before naming the
nearby paths:

```text
Candidate transitions: none from current states
Similar transitions from other states ranked by predicate distance:
non-current transition from q1 to q2 [+POST]; current states: q0; failed predicates: none
non-current transition from q1 to q3 [+POST +signed_by(/parties/alice.id)]; current states: q0; failed predicates: missing +signed_by(/parties/alice.id)
```

That distinction matters because a transition with no failed predicates can
still be unavailable from the current witness state.

The same state-mismatch hint should appear when replay is in a state that has
outgoing transitions, but those transitions are less relevant than a non-current
match. For example, an attempted `POST` from `q1` should still expose a perfect
`POST` transition back at `q0` even when `q1` has only an unrelated `FINISH`
transition:

```text
Closest candidate transition: candidate from current state q1: q1 to q2 [+FINISH +signed_by(/parties/alice.id)]; failed predicates: missing +FINISH, missing +signed_by(/parties/alice.id)
Similar transitions from other states with fewer failed predicates:
non-current transition from q0 to q1 [+POST]; current states: q1; failed predicates: none
```

## Predicate Evidence

Failed predicate lines should name the predicate and the missing or forbidden
evidence. Current first-contract-local examples include:

- `missing +signed_by(/parties/alice.id)` when the commit lacks Alice's accepted
  identity signature.
- `missing +threshold("2", /treasury/signers)` with counts for authorized
  signatures observed, required signatures, accepted members, missing
  signatures, and ignored unauthorized signatures.
- `forbidden -modifies(/members) matched` when the pending commit changes a path
  that the candidate transition explicitly forbids.

These diagnostics should stay tied to parsed commit facts, accepted state, and
signature evidence rather than raw string guesses.

## Current Executable Coverage

The onboarding smoke preserves the first-contract rejection surface in
`tests/cli/run-first-contract-cli-smoke.sh`. It asserts that an unsigned
post-bootstrap update:

- Replays to `q1`.
- Reports a closest signed transition candidate.
- Reports diagnostics in current-state, closest-candidate, ranked-section, then
  missing-predicate order.
- Names the missing `signed_by` predicate for Alice.
- Asserts the unsigned Alice-only rejection does not mention Bob or `+POST`,
  because that would imply broader authority than the current witness grants.

The contract evolution smoke preserves the same shape after model replacement,
including `missing +signed_by(/parties/bob.id)` once the accepted replacement
model has installed a Bob-authorized transition.
Focused local model-governance regressions cover the same explanation classes:

- `explains_similar_transitions_when_current_state_has_no_candidates` preserves
  the non-current transition fallback.
- `explains_closer_similar_transition_when_current_transition_is_unrelated`
  preserves the state-mismatch hint when the current state has only unrelated
  outgoing transitions.
- `explains_multi_state_rejections_with_sorted_current_states` preserves stable
  current-state ordering for nondeterministic local replay.
- `explains_signed_by_identity_bootstrap_ordering` preserves bootstrap-order
  evidence for identity paths.
- `explains_action_modal_rule_failure_with_transition_witness` preserves
  labelled transition witnesses for action-modal failures.
- `explains_lfp_rule_failure_with_unfolding_witness_set` preserves
  fixed-point unfolding witness sets.

Hub-side `model_validator` regressions cover the shared server path:

- `test_apply_action_advances_state` preserves basic accepted-action replay
  before the rejection-specific checks run, so the current-state diagnostics are
  anchored to real hub state advancement.
- `test_action_rejection_explains_candidate_transition_predicates` preserves
  current-state candidate ranking and missing predicate evidence.
- `test_action_rejection_ranks_closest_candidate_by_failed_predicates` preserves
  closest-candidate ordering when multiple current-state transitions share the
  pending action but have different predicate failures.
- `test_action_rejection_explains_similar_non_current_transitions` preserves
  similar transitions outside the current witness state.
- `test_action_rejection_surfaces_closer_non_current_transition` preserves the
  same state-mismatch hint on the shared hub validator path.
- `test_action_rejection_sorts_multi_current_state_header` preserves stable
  current-state ordering for nondeterministic hub replay.
- `test_model_replacement_rule_rejection_explains_formula_failure`,
  `test_model_replacement_rule_rejection_explains_action_modal_witness`, and
  `test_model_replacement_rule_rejection_explains_fixed_point_unfolding`
  preserve recursive formula, action-modal, and fixed-point model-replacement
  counterexamples.

The doc smoke also cross-checks these regression names against the local
governance and hub validator source files, so the reference cannot keep pointing
at a renamed or removed test without failing the no-build docs check.
The no-build doc smoke cross-checks the first-contract smoke for the promised
current-state, closest-candidate, missing-signature, and no-state-mutation
assertions too.

Shared `modal-common::model_diagnostics` formatter regressions preserve the
proof-fragment text both paths depend on:

- `summarizes_candidate_transition_with_stable_key_and_failures` preserves the
  ranked current-state candidate line and deterministic transition key.
- `summarizes_non_current_transition_with_current_states` preserves the
  non-current fallback line with explicit current states.
- `ranks_candidate_transitions_by_failures_then_stable_key` preserves the
  shared ordering helper used by local and hub candidate diagnostics.
- `ranks_candidate_transitions_by_summary_when_keys_match` preserves stable
  output when equal-distance transitions share the same source and target.
- `formats_state_sets_deterministically` preserves the sorted current-state
  header used by local and hub rejection diagnostics.
- `renders_recursive_formula_failure_diagnostic` preserves nested formula
  counterexample rendering.
- `renders_action_modal_transition_witness_diagnostic` preserves labelled
  action-modal transition witnesses.
- `renders_least_fixed_point_unfolding_diagnostic` preserves fixed-point
  witness-set and unfolding-count rendering.
- `renders_wildcard_state_transitions_as_current_candidates_only` preserves the
  wildcard current-state case, where every model transition is current and
  should not be duplicated as a non-current similar transition.
- `renders_mixed_wildcard_and_concrete_current_states_once` preserves the same
  wildcard current-state surface when replay reports `*` alongside a concrete
  state.
- `renders_duplicate_current_states_once` preserves the deduped current-state
  candidate surface when replay reports the same current state more than once.
- `renders_duplicate_transition_inputs_once` preserves the deduped explanation
  surface when model traversal reports the same transition more than once.

The no-build doc smoke cross-checks these names against
`rust/modal-common/src/model_diagnostics.rs` too, so shared formatter drift is
visible before a full Cargo build is available.

## Model-Replacement Rule Failures

When a pending `MODEL` replacement violates an accepted rule, the rejection
should explain the failed rule against the candidate model instead of falling
back to a generic rule violation. The current local and hub validators report:

- The failed anchor state where the accepted rule no longer holds.
- The satisfying states in the candidate model for the accepted formula.
- A recursive formula counterexample for common Boolean and temporal forms.
- Action-modal witnesses that name the matching transition, reached witness
  state, and nested reason the target state failed the formula.
- Fixed-point unfolding witnesses that show the final witness set, unfolding
  count, substituted variable set, and nested unfolded-body failure.

For example, a replacement that preserves replay history but breaks an accepted
least-fixed-point reachability rule should say that the failed anchor was never
added to the fixed-point witness set, then show the unfolded body that failed.
That makes the rejection reviewable as a model-checking counterexample rather
than a bare "replacement model violates rule" message.

## Boundaries

Rejection explanations prove why a pending commit did not match the accepted
model and evidence available to the verifier. They do not prove that an
external party should have signed, that off-chain evidence is true, or that a
different model would be a better contract. Those questions belong in review,
synthesis artifacts, or external evidence integrations.
