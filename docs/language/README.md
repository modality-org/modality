# Language Reference

This document covers the complete syntax for Modality's model and rule definitions.

## File Types

| Extension | Purpose | Location |
|-----------|---------|----------|
| `.modality` | Model or rule definitions | `model/` or `rules/` |
| `.id` | Public identity (ed25519 pubkey) | `state/` |
| `.passfile` | Private key (for signing) | Project root |
| `.hash` | SHA256 hash commitment | `state/` |
| `.datetime` | ISO 8601 timestamp | `state/` |

---

## Model Syntax

Models define **state machines** — the allowed behaviors of a contract.

### Basic Structure

```modality
model <name> {
  states { <state1>, <state2>, ... }
  initial <state>
  terminal <state1>, <state2>, ...
  
  transition <ACTION>: <from_state> -> <to_state>
    +<predicate1>
    +<predicate2>
}
```

### Complete Example

```modality
model escrow {
  states { pending, funded, delivered, released, refunded, disputed }
  initial pending
  terminal released, refunded
  
  // Buyer deposits funds
  transition DEPOSIT: pending -> funded
    +signed_by(/parties/buyer.id)
    +num_gte(/deposit/amount.num, /terms/price.num)
  
  // Seller delivers goods
  transition DELIVER: funded -> delivered
    +signed_by(/parties/seller.id)
  
  // Buyer releases payment
  transition RELEASE: delivered -> released
    +signed_by(/parties/buyer.id)
  
  // Seller refunds (before delivery)
  transition REFUND: funded -> refunded
    +signed_by(/parties/seller.id)
  
  // Either party disputes
  transition DISPUTE: funded -> disputed
    +signed_by(/parties/buyer.id)
  transition DISPUTE: funded -> disputed
    +signed_by(/parties/seller.id)
  
  // Arbiter resolves dispute
  transition RESOLVE_RELEASE: disputed -> released
    +signed_by(/parties/arbiter.id)
  transition RESOLVE_REFUND: disputed -> refunded
    +signed_by(/parties/arbiter.id)
}
```

### State Declarations

```modality
// List of all states
states { state1, state2, state3 }

// Initial state (required)
initial state1

// Terminal states (optional) - self-loop implied
terminal state2, state3
```

### Transitions

```modality
// Basic transition
transition ACTION_NAME: from_state -> to_state

// With predicates (all must be satisfied)
transition ACTION_NAME: from_state -> to_state
  +predicate1(args)
  +predicate2(args)

// Multiple transitions with same action (non-deterministic)
transition DISPUTE: funded -> disputed
transition DISPUTE: delivered -> disputed
```

---

## Rule Syntax

Rules express **temporal constraints** using modal mu-calculus.

### Basic Structure

```modality
rule <name> {
  starting_at <commit_ref>
  formula {
    <modal_formula>
  }
}

// Or as default export
export default rule {
  starting_at $PARENT
  formula {
    <modal_formula>
  }
}
```

### Anchoring (`starting_at`)

Rules are anchored to specific commits:

```modality
starting_at $PARENT           // Parent of this commit
starting_at $ROOT             // Genesis commit
starting_at abc123...         // Specific commit hash
```

### Modal Operators

#### Box (Necessity)
```modality
[ACTION] φ      // After ALL ACTION transitions, φ holds
[] φ            // After ALL transitions, φ holds (unlabeled)
```

#### Diamond (Possibility)
```modality
<ACTION> φ      // After SOME ACTION transition, φ holds
<> φ            // After SOME transition, φ holds (unlabeled)
```

#### Refusal
```modality
[-ACTION] φ     // If ACTION is refused/impossible, φ holds
```

#### DiamondBox (Commitment)
```modality
[<+ACTION>] φ   // Committed: CAN do ACTION and CANNOT refuse, φ holds after
                // Semantically: [-ACTION] false & <+ACTION> φ
```

### Temporal Operators (Syntactic Sugar)

```modality
always(φ)           // φ holds forever (invariant)
                    // = gfp(X, φ & []X)

eventually(φ)       // φ holds now or sometime later
                    // = lfp(X, φ | <>X)

until(p, q)         // p holds until q becomes true
                    // = lfp(X, q | (p & <>X))
```

### Fixed Points

For complex properties, use explicit fixed points:

```modality
// Greatest fixed point (νX) - invariants, safety
gfp(X, property & []X)

// Least fixed point (μX) - reachability, liveness
lfp(X, target | <>X)

// Unicode alternatives
νX. (property & []X)
μX. (target | <>X)
```

### Boolean Connectives

```modality
φ & ψ           // Conjunction (and)
φ | ψ           // Disjunction (or)
!φ              // Negation (not)
φ -> ψ          // Implication
φ <-> ψ         // Bi-implication
true            // Always true
false           // Always false
```

### State Propositions

```modality
@state_name     // True when in state_name
@pending        // True when in pending state
```

### Complete Rule Examples

#### Buyer Protection
```modality
rule buyer_protection {
  starting_at $PARENT
  formula {
    always (
      @funded -> (
        eventually(@released) | eventually(@refunded)
      )
    )
  }
}
```

#### Seller Guarantee
```modality
rule seller_guarantee {
  starting_at $PARENT
  formula {
    always (
      @delivered -> eventually(@released)
    )
  }
}
```

#### No Double Spending
```modality
rule no_double_spend {
  starting_at $PARENT
  formula {
    always (
      @released -> []!(@refunded)
    )
  }
}
```

---

## Predicates Reference

### Signature Predicates

```modality
+signed_by(/path/to/identity.id)
// Commit must be signed by this ed25519 key

+threshold(/signers/list, n, m)
// n-of-m signatures from the list required

+signed_by_n(/signers/list, n)
// At least n signatures from the list
```

### Oracle Predicates

```modality
+oracle_attests(/oracle.id, "condition")
// Oracle attests to a condition

+oracle_attests_fresh(/oracle.id, "condition", max_age_seconds)
// Oracle attestation within time limit
```

### Time Predicates

```modality
+before(/deadlines/cutoff.datetime)
// Current time is before deadline

+after(/deadlines/start.datetime)
// Current time is after start time
```

### Comparison Predicates

```modality
// Numeric comparisons
+num_eq(/path/a.num, /path/b.num)     // a == b
+num_gt(/path/a.num, /path/b.num)     // a > b
+num_gte(/path/a.num, /path/b.num)    // a >= b
+num_lt(/path/a.num, /path/b.num)     // a < b
+num_lte(/path/a.num, /path/b.num)    // a <= b

// Text comparisons
+text_eq(/path/a.text, /path/b.text)
+text_contains(/path/a.text, "substring")

// Boolean
+bool_true(/path/flag.bool)
+bool_false(/path/flag.bool)
```

### Hash Predicates

```modality
+hash_matches(/commitments/secret.hash, /revealed/value.text)
// SHA256 of revealed value matches stored hash
```

### Custom WASM Predicates

```modality
+wasm(/predicates/my_predicate.wasm, arg1, arg2)
// Call custom WASM predicate with arguments
```

---

## Path Syntax

Paths reference data in the contract state:

```
/directory/subdirectory/file.type
```

### Path Types

| Extension | Type | Example |
|-----------|------|---------|
| `.id` | ed25519 public key | `/parties/alice.id` |
| `.num` | Numeric value | `/terms/price.num` |
| `.text` | Text string | `/metadata/description.text` |
| `.bool` | Boolean | `/flags/approved.bool` |
| `.datetime` | ISO 8601 timestamp | `/deadlines/expiry.datetime` |
| `.date` | Date (YYYY-MM-DD) | `/terms/start.date` |
| `.hash` | SHA256 hash | `/commitments/secret.hash` |
| `.json` | JSON data | `/config/settings.json` |
| `.md` | Markdown text | `/docs/terms.md` |
| `.wasm` | WASM module | `/predicates/custom.wasm` |
| `.modality` | Model/rule file | `/model/default.modality` |

---

## Comments

```modality
// Single-line comment

/* 
   Multi-line
   comment
*/

model escrow {
  // States for the escrow workflow
  states { pending, funded }
}
```

---

## Grammar Summary (ABNF)

```abnf
model       = "model" name "{" model-body "}"
model-body  = states initial [terminal] *transition

states      = "states" "{" name *("," name) "}"
initial     = "initial" name
terminal    = "terminal" name *("," name)

transition  = "transition" ACTION ":" name "->" name *predicate
predicate   = "+" predicate-name ["(" args ")"]

rule        = ["export" "default"] "rule" [name] "{" rule-body "}"
rule-body   = starting-at formula-block

starting-at = "starting_at" commit-ref
commit-ref  = "$PARENT" / "$ROOT" / hash

formula-block = "formula" "{" formula "}"
formula       = modal-formula / temporal-formula / boolean-formula
```

---

## Next Steps

- **[Predicates Deep Dive](../predicates/)** — Custom predicates
- **[Tutorials](../tutorials/)** — Hands-on examples
- **[RFC Specification](../RFC-0001-MODAL-CONTRACTS.md)** — Formal spec
