# Pipeline Example: Escrow Contract

Status: archived experiment notes.

Current onboarding examples avoid formula implication sugar such as `A -> B`.
Use explicit Boolean conditionals such as `!<+ACTION> true | REQUIRED_FORMULA`,
and avoid `[+ACTION] true` as a conditional antecedent because `[+ACTION] true`
is a vacuous box formula.

## Input (Natural Language)

"Alice wants to buy from Bob. Alice deposits payment into escrow. Bob delivers the goods. Alice releases the payment to Bob. Only Alice can deposit and release. Only Bob can deliver."

---

## Step 1: Rule Generation

### Extracted Requirements

1. "Alice deposits payment" -> DEPOSIT action, signed by Alice
2. "Bob delivers the goods" -> DELIVER action, signed by Bob
3. "Alice releases the payment" -> RELEASE action, signed by Alice
4. "deposits before delivers before releases" -> ordering constraints
5. "Only Alice can deposit/release" -> authorization
6. "Only Bob can deliver" -> authorization

### Generated Formulas

```modality
// Ordering candidate: release requires delivery evidence in the witness path
F1: always(!<+RELEASE> true | eventually(<+DELIVER> true))

// Ordering candidate: delivery requires deposit evidence in the witness path
F2: always(!<+DELIVER> true | eventually(<+DEPOSIT> true))

// Authorization: only Alice can deposit
F3: always(!<+DEPOSIT> true | <+DEPOSIT +signed_by(/users/alice.id)> true)

// Authorization: only Bob can deliver
F4: always(!<+DELIVER> true | <+DELIVER +signed_by(/users/bob.id)> true)

// Authorization: only Alice can release
F5: always(!<+RELEASE> true | <+RELEASE +signed_by(/users/alice.id)> true)
```

---

## Step 2: Model Synthesis

### Analysis

1. **Actions:** DEPOSIT, DELIVER, RELEASE
2. **Ordering graph:** DEPOSIT → DELIVER → RELEASE
3. **States needed:** 4 (init, deposited, delivered, released)
4. **Auth requirements:**
   - DEPOSIT: +signed_by(/users/alice.id)
   - DELIVER: +signed_by(/users/bob.id)
   - RELEASE: +signed_by(/users/alice.id)

### Synthesized Model

```modality
model Escrow {
  part flow {
    init --> deposited: +DEPOSIT +signed_by(/users/alice.id)
    deposited --> delivered: +DELIVER +signed_by(/users/bob.id)
    delivered --> released: +RELEASE +signed_by(/users/alice.id)
    released --> released
  }
}
```

---

## Step 3: Review

Review each generated candidate against the synthesized model:

| Formula | Result | Reason |
|---------|--------|--------|
| F1 | needs production review | Historic ordering approximation; production examples should use explicit phase/history evidence rather than claiming forward reachability proves prior occurrence |
| F2 | needs production review | Historic ordering approximation; production examples should use explicit phase/history evidence rather than claiming forward reachability proves prior occurrence |
| F3 | lint-clean candidate | DEPOSIT and Alice signature are required on the same transition label |
| F4 | lint-clean candidate | DELIVER and Bob signature are required on the same transition label |
| F5 | lint-clean candidate | RELEASE and Alice signature are required on the same transition label |

The authorization candidates are lint-clean. The ordering candidates are kept as
archived synthesis output and should be replaced with phase/history predicates
before becoming production examples.

---

## Final Output

### Model File: `escrow.modality`

```modality
model Escrow {
  part flow {
    init --> deposited: +DEPOSIT +signed_by(/users/alice.id)
    deposited --> delivered: +DELIVER +signed_by(/users/bob.id)
    delivered --> released: +RELEASE +signed_by(/users/alice.id)
    released --> released
  }
}
```

### Rules File: `escrow.rules.modality`

```modality
// Candidate protection: release requires delivery evidence in the witness path
export default rule {
  starting_at $PARENT
  formula {
    always(!<+RELEASE> true | eventually(<+DELIVER> true))
  }
}
```

```modality
// Candidate protection: delivery requires deposit evidence in the witness path
export default rule {
  starting_at $PARENT
  formula {
    always(!<+DELIVER> true | eventually(<+DEPOSIT> true))
  }
}
```

---

## Protections Achieved

- **Alice (buyer):** Release action must carry Alice's same-transition signature evidence (F5)
- **Bob (seller):** Deliver action must carry Bob's same-transition signature evidence (F4)
- **Both:** Ordering intent is extracted, but this archived pipeline still needs phase/history evidence before it can be treated as a production proof (F1, F2)
