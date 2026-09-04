# Step 1: Rule Generation (NL -> Formulas)

Archived experiment notes for translating natural-language contract requirements into
temporal modal logic formulas.

Current onboarding examples avoid formula implication sugar. Prefer explicit Boolean
conditionals: `!A | B`. Also avoid `[+ACTION] true` as an antecedent; a box with
`true` is vacuous when no matching transition exists. Use action existence, such as
`<+ACTION> true`, or a labelled transition shape when teaching requirements.

## Examples

### Example 1: Simple ordering
**NL:** "Alice pays after Bob delivers"
**Formula:**
```modality
always(!<+RELEASE> true | eventually(<+DELIVER> true))
```

### Example 2: Authorization
**NL:** "Only Alice can release the funds"
**Formula:**
```modality
always(!<+RELEASE> true | <+RELEASE +signed_by(/users/alice.id)> true)
```

### Example 3: Mutual commitment
**NL:** "Both parties must sign before the contract is active"
**Formula:**
```modality
!<+ACTIVATE> true | (
  eventually(<+signed_by(/users/alice.id)> true) & 
  eventually(<+signed_by(/users/bob.id)> true)
)
```

### Example 4: No defection
**NL:** "Neither party can defect once committed"
**Formula:**
```modality
always(!<+COMMIT> true | always([-DEFECT] true))
```

### Example 5: Atomicity
**NL:** "Neither party can claim until both have committed"
**Formula:**
```modality
always(!<+CLAIM> true | (
  eventually(<+COMMIT_A> true) & eventually(<+COMMIT_B> true)
))
```

### Example 6: Revocable delegation
**NL:** "Agent can act on behalf of principal until revoked"
**Formula:**
```modality
always(!<+DELEGATE> true | <+DELEGATE +signed_by(/users/principal.id)> true) &
always(!<+ACT_ON_BEHALF> true | <+ACT_ON_BEHALF +signed_by(/users/agent.id)> true) &
always(!<+REVOKE> true | <+REVOKE +signed_by(/users/principal.id)> true) &
always(!<+REVOKE> true | always([-ACT_ON_BEHALF] true))
```

### Example 7: Quorum
**NL:** "Execution requires 2 of 3 signatures"
**Formula:**
```modality
always(!<+EXECUTE> true | (
  (eventually(<+signed_by(/users/m1.id)> true) & eventually(<+signed_by(/users/m2.id)> true)) |
  (eventually(<+signed_by(/users/m1.id)> true) & eventually(<+signed_by(/users/m3.id)> true)) |
  (eventually(<+signed_by(/users/m2.id)> true) & eventually(<+signed_by(/users/m3.id)> true))
))
```

## Common Patterns

| NL Pattern | Formula Pattern |
|------------|-----------------|
| "X after Y" | `!<+X> true \| eventually(<+Y> true)` |
| "Only A can X" | `!<+X> true \| <+X +signed_by(/users/a.id)> true` |
| "X requires Y and Z" | `!<+X> true \| (eventually(<+Y> true) & eventually(<+Z> true))` |
| "Never X after Y" | `!<+Y> true \| always([-X] true)` |
| "X or Y must happen" | `eventually(<+X> true) \| eventually(<+Y> true)` |
| "X before Y" | `!<+Y> true \| eventually(<+X> true)` |
| "Y cannot happen after X" | `!<+X> true \| always([-Y] true)` |

## LLM Prompt Template

```
You are a formal verification expert. Convert the following natural language 
contract requirement into a temporal modal logic formula using Modality syntax.

Syntax reference:
- always(phi) - phi holds forever
- eventually(phi) - phi holds at some future point
- !A | B - explicit Boolean conditional
- [+A] phi - all +A transitions lead to phi
- <+A> phi - some +A transition leads to phi
- [<+A>] phi - committed to A (can do, cannot refuse)
- +signed_by(/path) - requires signature evidence on a transition label

Requirement: {NL_REQUIREMENT}

Output only the formula, no explanation. Do not use `->`, `implies`, or `[+A] true`
as a condition.
```
