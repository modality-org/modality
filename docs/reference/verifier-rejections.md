# Verifier Rejection Explanations

Runtime verification should reject a commit from the accepted governing model and
then explain that rejection from the same model. Synthesis can help authors find
a witness model, but it is not part of commit-time acceptance.

## What A Good Rejection Shows

When no transition matches the pending commit, the explanation should include:

- The current governing-model state reached by replaying accepted commits.
- The closest candidate transition from that current state.
- The predicates that failed on that candidate.
- Other candidate transitions from the current state, ranked behind the closest
  candidate.
- Similar transitions from other states when the current state has no candidate
  for the pending action.

For the first-contract path, an unsigned steady-state update after bootstrap
should fail at `q1`. The useful rejection is not just "commit rejected"; it
points at the two signed `+POST` candidates and reports the missing signatures:

```text
current states {"q1"}
Closest candidate transition: candidate from current state q1: q1 to q1 [+POST +signed_by(/parties/alice.id)]; failed predicates: missing +signed_by(/parties/alice.id)
candidate from current state q1: q1 to q1 [+POST +signed_by(/parties/bob.id)]; failed predicates: missing +signed_by(/parties/bob.id)
```

That tells the user both where replay landed and what evidence would have made
the commit valid.

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
- Reports a closest signed `+POST` candidate.
- Lists the second signed `+POST` candidate.
- Names the missing `signed_by` predicates for Alice and Bob.

The contract evolution smoke preserves the same shape after model replacement.
Focused local model-governance regressions cover the same explanation classes:

- `explains_similar_transitions_when_current_state_has_no_candidates` preserves
  the non-current transition fallback.
- `explains_signed_by_identity_bootstrap_ordering` preserves bootstrap-order
  evidence for identity paths.
- `explains_action_modal_rule_failure_with_transition_witness` preserves
  labelled transition witnesses for action-modal failures.
- `explains_lfp_rule_failure_with_unfolding_witness_set` preserves
  fixed-point unfolding witness sets.

Hub-side `model_validator` regressions cover the shared server path:

- `test_action_rejection_explains_candidate_transition_predicates` preserves
  current-state candidate ranking and missing predicate evidence.
- `test_action_rejection_explains_similar_non_current_transitions` preserves
  similar transitions outside the current witness state.
- `test_model_replacement_rule_rejection_explains_formula_failure`,
  `test_model_replacement_rule_rejection_explains_action_modal_witness`, and
  `test_model_replacement_rule_rejection_explains_fixed_point_unfolding`
  preserve recursive formula, action-modal, and fixed-point model-replacement
  counterexamples.

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
