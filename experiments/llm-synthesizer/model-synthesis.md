# Step 2: Model Synthesis (Formulas → Model)

Given temporal modal logic formulas, synthesize a state machine that satisfies them.

## The Synthesis Problem

**Input:** Set of formulas F₁, F₂, ..., Fₙ
**Output:** Model M such that M ⊨ F₁ ∧ F₂ ∧ ... ∧ Fₙ

## Synthesis Heuristics

### Heuristic 1: Ordering Constraints

Formula: `[+X] implies eventually(<+Y> true)`
(X can only happen after Y has happened)

**Synthesis:** Create linear states where Y precedes X.

```
Input:  [+RELEASE] implies eventually(<+DELIVER> true)
Output:
  init --> delivered: +DELIVER
  delivered --> released: +RELEASE
  released --> released
```

### Heuristic 2: Authorization Constraints

Formula: `[+X] implies <+signed_by(A)> true`
(X requires A's signature)

**Synthesis:** Add signature requirement to all X transitions.

```
Input:  [+RELEASE] implies <+signed_by(/users/alice.id)> true
Output:
  state --> state: +RELEASE +signed_by(/users/alice.id)
```

### Heuristic 3: Mutual Commitment

Formula: `[+X] implies (eventually(<+A> true) & eventually(<+B> true))`
(X requires both A and B to have happened)

**Synthesis:** Create convergent paths.

```
Input:  [+ACTIVATE] implies (eventually(<+SIGN_A> true) & eventually(<+SIGN_B> true))
Output:
  init --> a_signed: +SIGN_A
  init --> b_signed: +SIGN_B
  a_signed --> both_signed: +SIGN_B
  b_signed --> both_signed: +SIGN_A
  both_signed --> active: +ACTIVATE
  active --> active
```

### Heuristic 4: Forbidden After

Formula: `[+X] implies always([-Y] true)`
(Once X happens, Y is forbidden forever)

**Synthesis:** Create absorbing state after X where Y is impossible.

```
Input:  [+COMMIT] implies always([-DEFECT] true)
Output:
  init --> init: +DEFECT -COMMIT
  init --> committed: +COMMIT -DEFECT
  committed --> committed: -DEFECT
```

### Heuristic 5: Disjunctive Requirements

Formula: `[+X] implies (<+A> true | <+B> true)`
(X requires A or B)

**Synthesis:** Create branching paths that converge.

```
Input:  [+PROCEED] implies (<+APPROVE> true | <+OVERRIDE> true)
Output:
  init --> approved: +APPROVE
  init --> overridden: +OVERRIDE
  approved --> done: +PROCEED
  overridden --> done: +PROCEED
  done --> done
```

## Algorithm Sketch

```python
def synthesize(formulas: List[Formula]) -> Model:
    # 1. Extract all actions mentioned in formulas
    actions = extract_actions(formulas)
    
    # 2. Build ordering graph from implies constraints
    ordering = build_ordering_graph(formulas)
    
    # 3. Create states based on ordering (topological sort)
    states = create_states_from_ordering(ordering)
    
    # 4. Create transitions with required properties
    transitions = []
    for action in actions:
        # Find which states this action can occur in
        valid_states = find_valid_states(action, formulas, states)
        # Add authorization requirements
        auth = extract_auth_requirements(action, formulas)
        # Create transition
        transitions.append(Transition(valid_states, action, auth))
    
    # 5. Add forbidden properties based on negative constraints
    add_forbidden_properties(transitions, formulas)
    
    # 6. Verify with model checker
    model = Model(states, transitions)
    if model_check(model, formulas):
        return model
    else:
        # Refine and retry
        return refine_and_retry(model, formulas)
```

## Example: Full Pipeline

### Input Formulas

```modality
// From NL: "Escrow where buyer deposits, seller delivers, buyer releases"
F1: [+RELEASE] implies eventually(<+DELIVER> true)
F2: [+DELIVER] implies eventually(<+DEPOSIT> true)
F3: [+DEPOSIT] implies <+signed_by(/users/buyer.id)> true
F4: [+DELIVER] implies <+signed_by(/users/seller.id)> true
F5: [+RELEASE] implies <+signed_by(/users/buyer.id)> true
```

### Synthesis Steps

1. **Extract actions:** DEPOSIT, DELIVER, RELEASE
2. **Build ordering:** DEPOSIT < DELIVER < RELEASE
3. **Create states:** init → deposited → delivered → released
4. **Add transitions with auth:**
   - init → deposited: +DEPOSIT +signed_by(/users/buyer.id)
   - deposited → delivered: +DELIVER +signed_by(/users/seller.id)
   - delivered → released: +RELEASE +signed_by(/users/buyer.id)
5. **Add terminal:** released → released

### Output Model

```modality
model Escrow {
  part flow {
    init --> deposited: +DEPOSIT +signed_by(/users/buyer.id)
    deposited --> delivered: +DELIVER +signed_by(/users/seller.id)
    delivered --> released: +RELEASE +signed_by(/users/buyer.id)
    released --> released
  }
}
```

### Verification

```
model_check(Escrow, F1) ✓
model_check(Escrow, F2) ✓
model_check(Escrow, F3) ✓
model_check(Escrow, F4) ✓
model_check(Escrow, F5) ✓
```

## Complexity

- General synthesis from temporal logic is **NP-complete** (or harder)
- But common patterns have efficient heuristics
- For patterns we can't handle, fall back to:
  1. Enumeration with pruning
  2. SMT solver
  3. Ask LLM for candidate, verify, refine

## Integration with Model Checker

The synthesized model should be verified:

```bash
modality model check escrow.modality --formula "always([+RELEASE] implies eventually(<+DELIVER> true))"
```

If verification fails, the synthesizer refines the model.
