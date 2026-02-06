# Members-Only Contract

A contract where only members can make changes, and adding new members requires unanimous consent.

## The Problem

You want a shared contract where:
- Only approved members can post
- Adding a new member requires ALL existing members to agree

## Key Concepts

### Rules + Model Together

When you add a RULE commit, you must also have a MODEL that satisfies it. The model defines the state machine structure; rules add constraints that must always hold.

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

```modality
model members_only {
  initial active
  
  active -> active [+POST]
  active -> active [+ADD_MEMBER]
  active -> active [+REMOVE_MEMBER]
}
```

Simple: one state, self-loops for each action type. The model doesn't enforce permissions — that's what rules are for.

## The Rules

Rules are immutable once added. They enforce constraints:

```modality
// Any commit requires at least one member signature
rule member_required {
  formula {
    always (+any_signed(/members))
  }
}

// Adding members requires ALL current members
rule add_member_unanimous {
  formula {
    always ([+ADD_MEMBER] implies +all_signed(/members))
  }
}

// Removing members also requires unanimous consent
rule remove_member_unanimous {
  formula {
    always ([+REMOVE_MEMBER] implies +all_signed(/members))
  }
}
```

### How rules work

| Predicate | Meaning |
|-----------|---------|
| `+any_signed(/members)` | At least one member under /members/ has signed |
| `+all_signed(/members)` | ALL members under /members/ have signed |
| `[+ACTION] implies X` | IF taking +ACTION THEN X must be true |

### Why rules, not just model?

The model can be replaced. A malicious user could post a new model with no guards. But **rules are immutable** — once added, they apply to all future commits, including model changes.

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

### 2. Add the model

```bash
modal c commit \
  --method model \
  --file members_only.modality \
  --sign alice.key
```

### 3. Add the rules

Each rule commit is validated against the model:

```bash
# Rule: any commit requires a member signature
modal c commit \
  --method rule \
  --value 'rule member_required { formula { always (+any_signed(/members)) } }' \
  --sign alice.key

# Rule: adding members requires unanimous consent
modal c commit \
  --method rule \
  --value 'rule add_member_unanimous { formula { always ([+ADD_MEMBER] implies +all_signed(/members)) } }' \
  --sign alice.key
```

**These rules are now permanent.**

### 4. Alice adds Bob

Alice is the only member, so only she needs to sign:

```bash
modal c commit \
  --method post \
  --path /members/bob.id \
  --action ADD_MEMBER \
  --value "$(modal identity show bob --public-key-hex)" \
  --sign alice.key
```

✓ Passes: `+all_signed(/members)` = [alice], alice signed ✓

### 5. Alice and Bob add Carol

Now BOTH must sign:

```bash
modal c commit \
  --method post \
  --path /members/carol.id \
  --action ADD_MEMBER \
  --value "$(modal identity show carol --public-key-hex)" \
  --sign alice.key \
  --sign bob.key
```

✓ Passes: `+all_signed(/members)` = [alice, bob], both signed ✓

### 6. Any member can post data

```bash
modal c commit \
  --method post \
  --path /data/meeting-notes.md \
  --action POST \
  --value "# Meeting Notes..." \
  --sign bob.key
```

✓ Passes: `+any_signed(/members)` = true, bob ∈ members ✓

### 7. Non-members rejected

```bash
modal c commit \
  --method post \
  --path /data/hack.md \
  --action POST \
  --value "Unauthorized!" \
  --sign stranger.key
```

✗ Rejected: `+any_signed(/members)` = false — stranger ∉ members

### 8. Partial signatures rejected

```bash
modal c commit \
  --method post \
  --path /members/dave.id \
  --action ADD_MEMBER \
  --value "$(modal identity show dave --public-key-hex)" \
  --sign alice.key \
  --sign bob.key
  # Missing carol!
```

✗ Rejected: `+all_signed(/members)` requires alice, bob, AND carol

## How Membership Evolves

The key insight: rules don't change, but their interpretation does.

| Step | Members | `+all_signed(/members)` requires |
|------|---------|--------------------------------|
| Initial | [alice] | [alice] |
| +bob | [alice, bob] | [alice, bob] |
| +carol | [alice, bob, carol] | [alice, bob, carol] |

The rule `always ([+ADD_MEMBER] implies +all_signed(/members))` stays constant. But as the member set grows, more signatures are required.

## Variations

### Admin override

```modality
rule admin_can_bypass {
  formula {
    always (signed_by(/admin.id) implies true)
  }
}

rule members_unless_admin {
  formula {
    always (not signed_by(/admin.id) implies +any_signed(/members))
  }
}
```

### Majority for membership changes

```modality
rule members_majority {
  formula {
    always ([+ADD_MEMBER] implies threshold(2, /members))
  }
}
```

### Lock certain paths

```modality
rule config_immutable {
  formula {
    always (not modifies(/config))
  }
}
```

## Summary

1. **Model** defines structure (can be replaced)
2. **Rules** add constraints (immutable once added)
3. **Rule commits require model** — validator checks formula against model
4. **Dynamic predicates** (`any_signed`, `all_signed`) evolve with state
5. **Rules never change** — their interpretation does
