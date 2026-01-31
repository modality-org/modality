# Step 1: Rule Generation (NL → Formulas)

LLM translates natural language contract requirements into temporal modal logic formulas.

## Examples

### Example 1: Simple ordering
**NL:** "Alice pays after Bob delivers"
**Formula:**
```modality
always([+RELEASE] implies eventually(<+DELIVER> true))
```

### Example 2: Authorization
**NL:** "Only Alice can release the funds"
**Formula:**
```modality
always([+RELEASE] implies <+signed_by(/users/alice.id)> true)
```

### Example 3: Mutual commitment
**NL:** "Both parties must sign before the contract is active"
**Formula:**
```modality
[+ACTIVATE] implies (
  eventually(<+signed_by(/users/alice.id)> true) & 
  eventually(<+signed_by(/users/bob.id)> true)
)
```

### Example 4: No defection
**NL:** "Neither party can defect once committed"
**Formula:**
```modality
always([+COMMIT] implies always([-DEFECT] true))
```

### Example 5: Atomicity
**NL:** "Neither party can claim until both have committed"
**Formula:**
```modality
always([+CLAIM] implies (
  eventually(<+COMMIT_A> true) & eventually(<+COMMIT_B> true)
))
```

### Example 6: Revocable delegation
**NL:** "Agent can act on behalf of principal until revoked"
**Formula:**
```modality
always([+ACT_ON_BEHALF] implies (
  eventually(<+DELEGATE> true) & !eventually(<+REVOKE> true)
))
```

### Example 7: Quorum
**NL:** "Execution requires 2 of 3 signatures"
**Formula:**
```modality
always([+EXECUTE] implies (
  (eventually(<+signed_by(/users/m1.id)> true) & eventually(<+signed_by(/users/m2.id)> true)) |
  (eventually(<+signed_by(/users/m1.id)> true) & eventually(<+signed_by(/users/m3.id)> true)) |
  (eventually(<+signed_by(/users/m2.id)> true) & eventually(<+signed_by(/users/m3.id)> true))
))
```

## Common Patterns

| NL Pattern | Formula Pattern |
|------------|-----------------|
| "X after Y" | `[+X] implies eventually(<+Y> true)` |
| "Only A can X" | `[+X] implies <+signed_by(A)> true` |
| "X requires Y and Z" | `[+X] implies (eventually(<+Y> true) & eventually(<+Z> true))` |
| "Never X after Y" | `[+Y] implies always([-X] true)` |
| "X or Y must happen" | `eventually(<+X> true) \| eventually(<+Y> true)` |
| "X before Y" | `[+Y] implies eventually(<+X> true)` |

## LLM Prompt Template

```
You are a formal verification expert. Convert the following natural language 
contract requirement into a temporal modal logic formula using Modality syntax.

Syntax reference:
- always(φ) — φ holds forever
- eventually(φ) — φ holds at some future point  
- [+A] φ — all +A transitions lead to φ
- <+A> φ — some +A transition leads to φ
- [<+A>] φ — committed to A (can do, cannot refuse)
- +signed_by(/path) — requires signature

Requirement: {NL_REQUIREMENT}

Output only the formula, no explanation.
```
