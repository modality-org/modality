---
sidebar_position: 10
title: Gotchas
---

# Gotchas

Common mistakes when writing Modality contracts.

## 1. Rules Use Predicates, Not Action Labels

**Wrong:** Referencing model action labels in rules
```modality
// DON'T DO THIS
always([+ADD_MEMBER -all_signed(/members)] false)
```

**Right:** Use predicates that describe the effect
```modality
// DO THIS
always([+modifies(/members) -all_signed(/members)] false)
```

Rules should describe *what* a commit does (modifies paths, requires
signatures), not *how* it is labeled in the model. The explicit Boolean form
also avoids formula implication sugar, which `modality model lint` reports as
`modality/implication-sugar`.

## 2. Negative Predicates Are Required for Exclusion

If a transition should NOT satisfy a predicate, you must explicitly negate it.

**Wrong:** Assuming one transition excludes another
```modality
model members_only {
  initial active
  // ← Can still modify /members!
  active -> active [+any_signed(/members)]
  active -> active [+modifies(/members) +all_signed(/members)]
}
```

**Right:** Explicitly negate with `-`
```modality
model members_only {
  initial active
  // ← CAN'T modify /members
  active -> active [+any_signed(/members) -modifies(/members)]
  // ← CAN modify, needs all sigs
  active -> active [+modifies(/members) +all_signed(/members)]
}
```

The `-modifies(/members)` ensures that path is protected on the first transition.

## 3. A Rule Commit Needs a Model That Meets It

A rule commit is accepted only if the contract's model (or the model posted in
the same commit) meets every rule, old and new, from the state the commit
reaches. For the rule `always([-any_signed(/members)] false)`:

**Wrong:** the unlabeled step lets an unsigned commit through, so the rule is
refused.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1
  }
}
```

**Right:** every step after the bootstrap needs a member's signature.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members)
  }
}
```

A model that meets the rules shows they can be met. It does not show that
commits can keep coming: `always([] false)` is met by a model with no moves
after the rule commit, and then every later commit is refused.

## 4. Models Can Be Replaced, Rules Cannot

```modality
// A model alone provides NO protection
// Can be replaced with anything!
model foo { active -> active }

// Rules make protections permanent
rule protect {
  formula { always([+modifies(/x) -signed_by(/admin.id)] false) }
}
```

If no rules exist, a user can post a new model with no guards. Rules make
protections permanent by constraining which replacement models are acceptable;
the current model's transition predicates still decide whether each commit is
accepted.

## 5. Predicate Syntax in Formulas

Predicates in formulas need the `+` prefix:
```modality
// In formulas
always([-any_signed(/members)] false)                    // ✓
always([+modifies(/path) -all_signed(/members)] false)  // ✓

// In transition labels  
active -> active [+any_signed(/members) -modifies(/members)]  // ✓ + for required, - for prohibited
```

## 6. Members Path Convention

Dynamic membership predicates (`any_signed`, `all_signed`) enumerate `.id` files under the path:
```
/members/alice.id   ← ed25519 public key
/members/bob.id     ← ed25519 public key
```

The predicates find all `*.id` files and check signatures against them.
