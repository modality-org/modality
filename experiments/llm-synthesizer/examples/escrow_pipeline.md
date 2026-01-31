# Pipeline Example: Escrow Contract

## Input (Natural Language)

"Alice wants to buy from Bob. Alice deposits payment into escrow. Bob delivers the goods. Alice releases the payment to Bob. Only Alice can deposit and release. Only Bob can deliver."

---

## Step 1: Rule Generation

### Extracted Requirements

1. "Alice deposits payment" → DEPOSIT action, signed by Alice
2. "Bob delivers the goods" → DELIVER action, signed by Bob  
3. "Alice releases the payment" → RELEASE action, signed by Alice
4. "deposits → delivers → releases" → ordering constraints
5. "Only Alice can deposit/release" → authorization
6. "Only Bob can deliver" → authorization

### Generated Formulas

```modality
// Ordering: release requires prior delivery
F1: always([+RELEASE] implies eventually(<+DELIVER> true))

// Ordering: delivery requires prior deposit
F2: always([+DELIVER] implies eventually(<+DEPOSIT> true))

// Authorization: only Alice can deposit
F3: always([+DEPOSIT] implies <+signed_by(/users/alice.id)> true)

// Authorization: only Bob can deliver
F4: always([+DELIVER] implies <+signed_by(/users/bob.id)> true)

// Authorization: only Alice can release
F5: always([+RELEASE] implies <+signed_by(/users/alice.id)> true)
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

## Step 3: Verification

Check each formula against the model:

| Formula | Result | Reason |
|---------|--------|--------|
| F1 | ✓ | RELEASE only in `delivered→released`, DELIVER in prior transition |
| F2 | ✓ | DELIVER only in `deposited→delivered`, DEPOSIT in prior transition |
| F3 | ✓ | DEPOSIT transition has +signed_by(alice) |
| F4 | ✓ | DELIVER transition has +signed_by(bob) |
| F5 | ✓ | RELEASE transition has +signed_by(alice) |

**All formulas satisfied!**

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
// Protection: release requires delivery
export default rule {
  starting_at $PARENT
  formula {
    always([+RELEASE] implies eventually(<+DELIVER> true))
  }
}
```

```modality
// Protection: delivery requires deposit
export default rule {
  starting_at $PARENT
  formula {
    always([+DELIVER] implies eventually(<+DEPOSIT> true))
  }
}
```

---

## Protections Achieved

- **Alice (buyer):** Cannot release funds without receiving goods (F1)
- **Bob (seller):** Cannot be asked to deliver without deposit secured (F2)
- **Both:** Actions are properly authorized (F3, F4, F5)
