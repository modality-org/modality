# Model Synthesis Experiments

## The Problem

Given a rule (temporal modal formula), generate a governing model (LTS) that satisfies it.

## Key Insight

The most permissive model is a **permissive looper**:
```modality
model Permissive:
  part p1:
    n1 --> n1
```

This satisfies any empty rule set. From here, we constrain.

## Synthesis Heuristics

### Pattern 1: `always must +A`
All future commits must include action A.

**Synthesis:** Add +A to all transitions.
```modality
model AlwaysMustA:
  part p1:
    n1 --> n1: +A
```

### Pattern 2: `must +A` (next commit only)
Equivalent to `[-A] false` — the next commit must include A.

**Synthesis:** Single transition with +A, then permissive.
```modality
model MustA:
  part p1:
    n1 --> n2: +A
    n2 --> n2
```

### Pattern 3: `can +A`
The right to do A at some point.

**Synthesis:** Permissive (neutral transitions satisfy this).
```modality
model CanA:
  part p1:
    n1 --> n1  // neutral to A, so +A is allowed
```

### Pattern 4: Alternating turns (Alice and Bob)
`always (must +SIGNED_BY_ALICE or must +SIGNED_BY_BOB)`

**Synthesis:** Two-state cycle.
```modality
model AlternatingTurns:
  part p1:
    alice_turn --> bob_turn: +SIGNED_BY_ALICE
    bob_turn --> alice_turn: +SIGNED_BY_BOB
```

### Pattern 5: Exclusive actions (only Alice can do X)
`always [-DO_X] false or [+SIGNED_BY_ALICE] true`

**Synthesis:** Any transition with +DO_X must also have +SIGNED_BY_ALICE.
```modality
model ExclusiveAction:
  part p1:
    n1 --> n1: +DO_X +SIGNED_BY_ALICE
    n1 --> n1: -DO_X  // or just neutral
```

## AI-Assisted Synthesis Approach

1. **Parse the rule** into its temporal structure
2. **Identify the pattern** (always, eventually, until, etc.)
3. **Generate candidate model** using heuristics
4. **Verify with model checker**
5. **Refine if needed** (add states, adjust transitions)

## Test Cases

### Test 1: Simple obligation
Rule: `must +COOPERATE`
Expected model:
```modality
model Test1:
  part p1:
    n1 --> n2: +COOPERATE
    n2 --> n2
```

### Test 2: Mutual signature requirement
Rule: `always must (+SIGNED_BY_ALICE or +SIGNED_BY_BOB)`
Expected model:
```modality
model Test2:
  part p1:
    n1 --> n1: +SIGNED_BY_ALICE
    n1 --> n1: +SIGNED_BY_BOB
```

### Test 3: Conditional obligation
Rule: `when +RECEIVED_PAYMENT always must +DELIVER`
Expected model: State machine with PENDING -> PAID -> DELIVERED

## Next Steps

1. Implement these heuristics in code
2. Test against the model checker
3. Identify failure cases
4. Iterate
