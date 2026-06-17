# Members-Only Contract

A contract where only members can make changes, and modifying membership requires unanimous consent.

## The Problem

You want a shared contract where:
- Only approved members can post
- Changing membership requires ALL existing members to agree

## Key Concepts

### Rules Constrain Models

Rules are permanent formulas over predicates. They do not validate commits directly; instead, each governing model must satisfy the accumulated rules. Commits are accepted or rejected by matching the current governing model's transition predicates.

```modality
// WRONG - references model structure
always ([+ADD_MEMBER] implies +all_signed(/members))

// RIGHT - constrains acceptable witness models with predicates
always([+modifies(/members)] true -> <+all_signed(/members)> true)
```

### Dynamic Membership

The predicates `+any_signed(/members)` and `+all_signed(/members)` enumerate keys at runtime:
- As members are added/removed, the interpretation changes
- The RULES never change, but their MEANING evolves with state

## State Structure

Members are stored as identity files:

```
/members/
  alice.id → "abc123..."  (hex pubkey)
  bob.id → "def456..."
  carol.id → "ghi789..."
```

Each `.id` file contains that member's public key (hex-encoded).

## The Model

Use transition predicates to encode the permissions that actually gate commits:

```modality
model members_only {
  initial active
  
  // Non-membership commits: any member can sign, but CAN'T touch /members
  active -> active [+any_signed(/members) -modifies(/members)]
  
  // Membership commits: CAN modify /members, needs unanimous consent
  active -> active [+modifies(/members) +all_signed(/members)]
}
```

**Key insight:** The `-modifies(/members)` on the first transition is required. Without it, that transition could be used to modify membership with just one signature — the model wouldn't enforce protection.

The two transitions partition the action space:
- First: any commit that doesn't modify `/members` → any single member signature
- Second: any commit that modifies `/members` → all member signatures

## The Rules

Rules are immutable once added. They constrain future witness models using predicates, so a replacement model cannot forget the protections:

```modality
// Any commit requires at least one member signature
rule member_required {
  formula {
    always(<+any_signed(/members)> true)
  }
}

// Modifying /members/ requires ALL current members
rule membership_unanimous {
  formula {
    always([+modifies(/members)] true -> <+all_signed(/members)> true)
  }
}
```

### How rules work

| Predicate | Meaning |
|-----------|---------|
| `+any_signed(/members)` | At least one member under /members/ has signed |
| `+all_signed(/members)` | ALL members under /members/ have signed |
| `+modifies(/members)` | Commit writes to a path under /members/ |

### Why predicates, not action labels?

- **Models validate commits** — each commit must match a valid transition from the current model state
- **Rules validate models** — they constrain which witness models are acceptable
- **Decoupling** — rules shouldn't depend on model action names

## Walkthrough

### 1. Create contract and add first member

```bash
modal c create members_only

# Alice adds herself as first member
modal c commit \
  --method post \
  --path /members/alice.id \
  --value "$(modal identity show alice --public-key-hex)" \
  --sign alice.key
```

### 2. Add rules with satisfying model

Each rule commit must include a **model that witnesses satisfiability**. The model proves the rule can be satisfied.

```bash
# Rule: any commit requires a member signature
# Witness model must satisfy the rule
modal c commit \
  --method rule \
  --rule 'rule member_required { formula { always(<+any_signed(/members)> true) } }' \
  --model 'model witness { initial s; s -> s [+any_signed(/members)] }' \
  --sign alice.key

# Rule: modifying members requires unanimous consent
# Witness must show the implication is satisfiable
modal c commit \
  --method rule \
  --rule 'rule membership_unanimous { formula { always([+modifies(/members)] true -> <+all_signed(/members)> true) } }' \
  --model 'model witness { initial s; s -> s [+any_signed(/members) -modifies(/members)]; s -> s [+modifies(/members) +all_signed(/members)] }' \
  --sign alice.key
```

**Why require a model with each rule?**
- The model acts as a **witness** proving the rule is satisfiable
- Without a satisfying model, the rule commit is rejected
- This prevents adding unsatisfiable rules that would deadlock the contract

**These rules are now permanent.**

### 4. Alice adds Bob

Alice is the only member, so only she needs to sign:

```bash
modal c commit \
  --method post \
  --path /members/bob.id \
  --value "$(modal identity show bob --public-key-hex)" \
  --sign alice.key
```

✓ Passes: `+modifies(/members)`=true, `+all_signed([alice])`=true ✓

### 5. Alice and Bob add Carol

Now BOTH must sign (commit modifies /members/):

```bash
modal c commit \
  --method post \
  --path /members/carol.id \
  --value "$(modal identity show carol --public-key-hex)" \
  --sign alice.key \
  --sign bob.key
```

✓ Passes: `+modifies(/members)`=true, `+all_signed([alice,bob])`=true ✓

### 6. Any member can post data

```bash
modal c commit \
  --method post \
  --path /data/meeting-notes.md \
  --value "# Meeting Notes..." \
  --sign bob.key
```

✓ Passes: `+any_signed(/members)`=true, `+modifies(/members)`=false ✓

### 7. Non-members rejected

```bash
modal c commit \
  --method post \
  --path /data/hack.md \
  --value "Unauthorized!" \
  --sign stranger.key
```

✗ Rejected: `+any_signed(/members)`=false — stranger ∉ members

### 8. Partial signatures rejected

```bash
modal c commit \
  --method post \
  --path /members/dave.id \
  --value "$(modal identity show dave --public-key-hex)" \
  --sign alice.key \
  --sign bob.key
  # Missing carol!
```

✗ Rejected: `+all_signed(/members)` requires alice, bob, AND carol

## How Membership Evolves

The key insight: predicates are evaluated against current state, so the same transition labels become stricter as membership changes.

| Step | Members | `+all_signed(/members)` requires |
|------|---------|--------------------------------|
| Initial | [alice] | [alice] |
| +bob | [alice, bob] | [alice, bob] |
| +carol | [alice, bob, carol] | [alice, bob, carol] |

The transition `+modifies(/members) +all_signed(/members)` stays constant. But as the member set grows, more signatures are required for membership changes.

## Variations

### Admin bypass for member-required commits

This variation lets either an admin or a member authorize ordinary commits. It
does not weaken the separate membership-change rule above.

```modality
rule admin_bypass {
  formula {
    always(<+signed_by(/admin.id)> true | <+any_signed(/members)> true)
  }
}
```

### Protect config paths

```modality
rule config_protected {
  formula {
    always([+modifies(/config)] true -> <+signed_by(/admin.id)> true)
  }
}
```

### Majority for membership

```modality
model membership_majority {
  initial active
  active -> active [+any_signed(/members) -modifies(/members)]
  active -> active [+modifies(/members) +threshold(2, /members)]
}
```

## Summary

1. **Model** enforces commit permissions through transition predicates
2. **Rules** constrain acceptable witness models via **predicates**
3. Rules should NOT reference action labels from the model
4. **Dynamic predicates** (`+any_signed`, `+all_signed`) evolve with state
5. **Path predicates** (`+modifies`) check what the commit touches
