# Modality Syntax Proposals

*Based on use case analysis - making the language more efficient for real agent cooperation.*

---

## Current Pain Points

1. **Symmetric patterns are verbose** - Escrow, trade, handshake all require many transitions
2. **No role abstraction** - `+SIGNED_BY_X` repeated everywhere
3. **No temporal deadlines** - Critical for real contracts
4. **Property lists are flat** - No grouping or composition

---

## Proposal 1: Signer Shorthand

**Problem:** `+SIGNED_BY_ALICE` is verbose and error-prone.

**Current:**
```modality
model Trade {
  part exchange {
    init --> done: +DELIVER +SIGNED_BY_ALICE
    init --> done: +DELIVER +SIGNED_BY_BOB
  }
}
```

**Proposed:**
```modality
model Trade {
  signers: [Alice, Bob]
  
  part exchange {
    init --> done: +DELIVER @Alice
    init --> done: +DELIVER @Bob
  }
}
```

**Benefit:** Cleaner syntax, signers declared upfront, typo-resistant.

---

## Proposal 2: Symmetric Shorthand

**Problem:** Many contracts are symmetric between parties.

**Current:**
```modality
model Handshake {
  part agreement {
    pending --> alice_signed: +SIGNED_BY_ALICE -SIGNED_BY_BOB
    pending --> bob_signed: +SIGNED_BY_BOB -SIGNED_BY_ALICE
    alice_signed --> active: +SIGNED_BY_BOB
    bob_signed --> active: +SIGNED_BY_ALICE
  }
}
```

**Proposed:**
```modality
model Handshake {
  signers: [Alice, Bob]
  
  symmetric handshake(Alice, Bob) {
    // Both must sign to activate
    pending --> active: both_sign
  }
}
```

Or even simpler with a primitive:

```modality
handshake Handshake(Alice, Bob)  // Generates the model automatically
```

**Benefit:** Common pattern becomes one line.

---

## Proposal 3: Property Groups

**Problem:** Related properties repeated across transitions.

**Current:**
```modality
part exchange {
  init --> step1: +DEPOSIT +SIGNED_BY_BUYER +TIMESTAMP
  step1 --> step2: +DELIVER +SIGNED_BY_SELLER +TIMESTAMP
  step2 --> done: +CONFIRM +SIGNED_BY_BUYER +TIMESTAMP
}
```

**Proposed:**
```modality
properties {
  BuyerAction = @Buyer +TIMESTAMP
  SellerAction = @Seller +TIMESTAMP
}

part exchange {
  init --> step1: +DEPOSIT BuyerAction
  step1 --> step2: +DELIVER SellerAction
  step2 --> done: +CONFIRM BuyerAction
}
```

**Benefit:** DRY, easier to update, clearer intent.

---

## Proposal 4: Sequence Shorthand

**Problem:** Linear progressions require explicit state names.

**Current:**
```modality
part flow {
  init --> deposited: +DEPOSIT @Buyer
  deposited --> delivered: +DELIVER @Seller
  delivered --> confirmed: +CONFIRM @Buyer
  confirmed --> released: +RELEASE
}
```

**Proposed:**
```modality
sequence flow {
  +DEPOSIT @Buyer -->
  +DELIVER @Seller -->
  +CONFIRM @Buyer -->
  +RELEASE
}
```

**Benefit:** Linear flows are extremely common and become trivial to write.

---

## Proposal 5: Guard Conditions

**Problem:** Can't express "only if X".

**Current:** Not expressible directly, requires formula constraints.

**Proposed:**
```modality
part exchange {
  escrowed --> delivered: +DELIVER @Seller when (amount > 0)
  delivered --> complete: +RELEASE when (delivered && confirmed)
}
```

**Benefit:** Conditions are explicit in the model, not separate formulas.

---

## Implementation Priority

| Proposal | Impact | Complexity | Priority |
|----------|--------|------------|----------|
| Signer shorthand (@) | High | Low | 1 |
| Sequence shorthand | High | Medium | 2 |
| Property groups | Medium | Low | 3 |
| Symmetric shorthand | High | Medium | 4 |
| Guard conditions | High | High | 5 |

---

## Next Steps

1. Implement `@Signer` shorthand in grammar
2. Add `signers:` declaration to model
3. Test with existing examples
4. Iterate based on ergonomics

---

*These are proposals - discussing with Foy before implementing.*
