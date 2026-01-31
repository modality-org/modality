# Event-Anchored Formulas

*Decidable temporal logic for agent contracts via hybrid logic semantics*

---

## The Problem

We need to express ordering constraints like "no release without prior delivery" but:
- Past-looking operators (`past`, `since`) break decidability
- Pure future LTL can't reference what already happened
- We need both sequence enforcement AND decidable model checking

## The Solution: Event-Anchored Assertions

Each formula is **bound to an event** and speaks only about the **future from that point**.

```modality
model Escrow {
  on init {
    assert eventually(deposited | cancelled)
  }
  
  on deposit {
    assert eventually(delivered | refunded)
  }
  
  on deliver {
    assert eventually(released | disputed)
  }
  
  on dispute {
    assert eventually(arbiter_resolves)
  }
}
```

**Semantics:**
- `on E { assert F }` means: "when event E occurs, formula F is added to the active obligation set"
- F is pure future-looking LTL: `eventually`, `always`, `until`, `next`
- The **event** provides the temporal anchor; the **formula** constrains the future

---

## Why This Works

### Sequence Enforcement Without Past Operators

Consider: "release requires prior delivery"

**Old way (undecidable):**
```
always(released → past(delivered))
```

**New way (decidable):**
```modality
on deliver {
  assert eventually(released | refunded)
}
```

The formula doesn't say "delivery happened before release." It says "from the moment of delivery, release (or refund) must eventually happen." 

Sequence is enforced by **when formulas fire**, not by looking backwards.

### Hybrid Logic Interpretation

This is essentially hybrid logic with:
- **Nominals**: Event occurrences (deposit, deliver, dispute) name points in time
- **@-operator**: `on E { F }` ≈ `@E → F` 
- **Pure future modalities**: ◇ (eventually), □ (always), U (until)

The event is the anchor. The formula looks forward. Decidability preserved.

---

## Formal Semantics

### Syntax

```
Model      ::= 'model' Name '{' Clause* '}'
Clause     ::= 'on' Event '{' Assertion* '}'
Assertion  ::= 'assert' Formula
Formula    ::= 'eventually' '(' Formula ')'
             | 'always' '(' Formula ')'
             | Formula 'until' Formula
             | 'next' '(' Formula ')'
             | Formula '|' Formula
             | Formula '&' Formula
             | 'not' '(' Formula ')'
             | Prop
Prop       ::= StateName | 'signed_by' '(' Key ')' | Predicate
Event      ::= 'init' | ActionName
```

### Semantics

A contract execution is a trace: `σ = s₀ →^a₁ s₁ →^a₂ s₂ → ...`

When action `aᵢ` occurs at time `i`:
1. Find all `on aᵢ { assert F }` clauses
2. Add obligation `(i, F)` to active set
3. Each obligation `(t, F)` is checked against suffix `σ[t:]`

**Satisfaction:** Trace σ satisfies model M iff all triggered obligations are met.

---

## Operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `eventually(P)` | P holds at some future point | `eventually(released)` |
| `always(P)` | P holds at all future points | `always(not(double_spend))` |
| `P until Q` | P holds until Q becomes true | `escrowed until (delivered \| refunded)` |
| `next(P)` | P holds at the next state | `next(awaiting_confirmation)` |

---

## Examples

### Escrow with Full Invariants

```modality
model Escrow {
  on init {
    // Contract must eventually terminate
    assert eventually(released | refunded | cancelled)
  }
  
  on deposit {
    // Once deposited, funds must resolve
    assert eventually(released | refunded)
    // No double-spend
    assert always(not(released & refunded))
  }
  
  on deliver {
    // Delivery triggers release eligibility
    assert eventually(released | disputed)
  }
  
  on dispute {
    // Disputes must resolve
    assert eventually(arbiter_rules)
    // During dispute, no unilateral release
    assert (not(released)) until arbiter_rules
  }
}
```

### Service Agreement

```modality
model ServiceAgreement {
  on offer {
    assert eventually(accepted | rejected | expired)
  }
  
  on accept {
    assert eventually(completed | cancelled_with_refund)
    assert always(signed_by(provider) | signed_by(consumer) | signed_by(arbiter))
  }
  
  on complete {
    assert eventually(paid)
  }
}
```

### Multi-Sig Treasury

```modality
model Treasury {
  on propose_spend {
    assert eventually(executed | rejected | expired)
    assert (not(executed)) until (approvals >= threshold)
  }
  
  on approve {
    assert always(signed_by(council_member))
  }
}
```

---

## Model Checking

Given a model M and a candidate state machine S:

1. Enumerate all traces of S (or use symbolic methods)
2. For each trace σ:
   - Simulate event firing, collect obligations
   - Check each obligation against trace suffix
3. If any obligation fails, return counterexample trace
4. If all pass, model is verified

**Complexity:** PSPACE-complete (same as LTL model checking)

---

## Synthesis

Given just the `on/assert` clauses, synthesize a minimal state machine:

1. Events become action labels
2. Formulas constrain which transitions are legal
3. Search for smallest automaton satisfying all formulas

**Key insight:** Formulas are the spec. State machine is an implementation. Multiple valid implementations may exist.

---

## Comparison

| Approach | Past Operators | Decidable | Sequence Enforcement |
|----------|---------------|-----------|---------------------|
| Full TL | Yes | No | Explicit |
| Pure LTL | No | Yes | Weak |
| Event-Anchored | No | Yes | Via assertion timing |

---

## Implementation Notes

### Parser Changes
- Add `on event { }` blocks
- Formulas inside are pure future LTL

### Runtime Changes
- Track active obligations as `Vec<(Timestamp, Formula)>`
- On each transition, check obligations against remaining trace
- Obligations satisfied when their formula becomes true
- Obligations violated if trace ends without satisfaction

### CLI Changes
```bash
modality check contract.modality      # verify formulas against model
modality obligations contract.modality # list what each event triggers
modality synthesize spec.modality     # generate model from formulas only
```
