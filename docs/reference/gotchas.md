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
always ([+ADD_MEMBER] implies +all_signed(/members))
```

**Right:** Use predicates that describe the effect
```modality
// DO THIS
always (+modifies(/members) implies +all_signed(/members))
```

Rules should describe *what* a commit does (modifies paths, requires signatures), not *how* it's labeled in the model.

## 2. Negative Predicates Are Required for Exclusion

If a transition should NOT satisfy a predicate, you must explicitly negate it.

**Wrong:** Assuming one transition excludes another
```modality
model members_only {
  initial active
  active -> active [+any_signed(/members)]                    // ← Can still modify /members!
  active -> active [+modifies(/members) +all_signed(/members)]
}
```

**Right:** Explicitly negate with `-`
```modality
model members_only {
  initial active
  active -> active [+any_signed(/members) -modifies(/members)]  // ← CAN'T modify /members
  active -> active [+modifies(/members) +all_signed(/members)]  // ← CAN modify, needs all sigs
}
```

The `-modifies(/members)` ensures that path is protected on the first transition.

## 3. Rule Commits Require a Satisfying Model Witness

When submitting a rule, include a model that **actually satisfies** the rule:

**Wrong:** Empty witness doesn't prove anything
```bash
modal c commit --method rule \
  --rule 'rule X { formula { always (+any_signed(/members)) } }' \
  --model 'model Y { initial s; s -> s [] }' \  # ← doesn't satisfy the rule!
  --sign key
```

**Right:** Witness model includes required predicates
```bash
modal c commit --method rule \
  --rule 'rule X { formula { always (+any_signed(/members)) } }' \
  --model 'model Y { initial s; s -> s [+any_signed(/members)] }' \
  --sign key
```

The model acts as a witness proving the rule is satisfiable. The hub rejects rules without valid witnesses to prevent deadlock.

## 4. Models Can Be Replaced, Rules Cannot

```modality
// A model alone provides NO protection
model foo { active -> active [] }  // Can be replaced with anything!

// Rules make protections permanent
rule protect {
  formula { always (+modifies(/x) implies +signed_by(/admin.id)) }
}
```

If no rules exist, a user can post a new model with no guards. Rules are the enforcement mechanism.

## 5. CI Uses `-D warnings`

Always run before committing:
```bash
RUSTFLAGS="-D warnings" cargo check --all
```

CI treats warnings as errors. Local `cargo check` may pass while CI fails.

## 6. Predicate Syntax in Formulas

Predicates in formulas need the `+` prefix:
```modality
// In formulas
always (+any_signed(/members))              // ✓
always (+modifies(/path) implies +all_signed(/members))  // ✓

// In transition labels  
active -> active [+any_signed(/members) -modifies(/members)]  // ✓ + for required, - for prohibited
```

## 7. Members Path Convention

Dynamic membership predicates (`any_signed`, `all_signed`) enumerate `.id` files under the path:
```
/members/alice.id   ← ed25519 public key
/members/bob.id     ← ed25519 public key
```

The predicates find all `*.id` files and check signatures against them.
