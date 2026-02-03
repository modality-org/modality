# Members-Only Contract

A contract where only members can make changes, and adding new members requires unanimous consent.

## The Problem

You want a shared contract where:
- Only approved members can post
- Adding a new member requires ALL existing members to agree

## State Structure

Members are stored as identity files:

```
/members/
  alice.id
  bob.id
  carol.id
```

Each `.id` file contains that member's public key.

## The Model

The model defines the state machine structure:

```modality
model members_only {
  initial active
  active -> active []
}
```

Simple: one state, self-loop. The model doesn't enforce permissions - that's what rules are for.

## The Rules

Rules are immutable once added. They enforce the constraints:

```modality
rule data_requires_member {
  formula {
    always (modifies(/data) implies any_signed(/members))
  }
}

rule members_requires_all {
  formula {
    always (modifies(/members) implies all_signed(/members))
  }
}
```

### How rules work

| Rule | Meaning |
|------|---------|
| `modifies(/data) implies any_signed(/members)` | IF touching /data THEN need member signature |
| `modifies(/members) implies all_signed(/members)` | IF touching /members THEN need ALL signatures |

Rules use `implies`: the predicate on the left triggers the requirement on the right.

### Why rules, not just model?

The model can be replaced. A malicious user could post a new model with no guards. But **rules are immutable** - once added, they apply to all future commits, including model changes.

## Walkthrough

### 1. Alice creates the contract

```bash
modal contract create --id shared_notes

# Alice adds herself as first member
modal contract commit \
  --method post \
  --path /members/alice.id \
  --value "$(modal identity show alice --public-key)" \
  --sign alice
```

### 2. Add the model

```bash
modal contract commit \
  --method post \
  --path /model.modality \
  --value 'model members_only { initial active; active -> active [] }' \
  --sign alice
```

### 3. Add the rules

```bash
# Rule: modifying /data requires any member
modal contract commit \
  --method rule \
  --value 'rule data_requires_member { formula { always (modifies(/data) implies any_signed(/members)) } }' \
  --sign alice

# Rule: modifying /members requires ALL members
modal contract commit \
  --method rule \
  --value 'rule members_requires_all { formula { always (modifies(/members) implies all_signed(/members)) } }' \
  --sign alice
```

**These rules are now permanent.**

### 4. Alice adds Bob

Alice is the only member, so only she needs to sign:

```bash
modal contract commit \
  --method post \
  --path /members/bob.id \
  --value "$(modal identity show bob --public-key)" \
  --sign alice
```

✓ Passes: `modifies(/members)` triggers `all_signed(/members)` = [alice] ✓

### 5. Alice and Bob add Carol

Now BOTH must sign:

```bash
modal contract commit \
  --method post \
  --path /members/carol.id \
  --value "$(modal identity show carol --public-key)" \
  --sign alice \
  --sign bob
```

✓ Passes: `all_signed(/members)` = [alice, bob] ✓

### 6. Any member can post data

```bash
modal contract commit \
  --method post \
  --path /data/meeting-notes.md \
  --value "# Meeting Notes..." \
  --sign bob
```

✓ Passes: `modifies(/data)` triggers `any_signed(/members)`, bob ∈ members ✓

### 7. Non-members rejected

```bash
modal contract commit \
  --method post \
  --path /data/hack.md \
  --value "Unauthorized!" \
  --sign stranger
```

✗ Rejected: `any_signed(/members)` fails — stranger ∉ members

### 8. Partial signatures rejected

```bash
modal contract commit \
  --method post \
  --path /members/dave.id \
  --value "$(modal identity show dave --public-key)" \
  --sign alice \
  --sign bob
  # Missing carol!
```

✗ Rejected: `all_signed(/members)` requires carol

## Final State

```
/model.modality
/members/
  alice.id → "abc123..."
  bob.id → "def456..."
  carol.id → "ghi789..."
/data/
  meeting-notes.md → "# Meeting Notes..."
```

Plus two immutable rules enforcing the membership requirements.

## Rule Evaluation

When a commit arrives:

1. **Extract modified paths** from commit body
2. **Evaluate each rule's formula**
3. **Check implications**: if left side true, right side must be true
4. **Reject** if any rule fails

```
Commit: POST /members/dave.id

Rule: modifies(/members) implies all_signed(/members)
  - modifies(/members) = true (touching /members/dave.id)
  - all_signed(/members) = ? (check signers vs members)
  - If signers include all members: PASS
  - If missing any member: REJECT
```

## Variations

### Admin override

```modality
rule admin_can_do_anything {
  formula {
    always (signed_by(/admin.id) implies true)
  }
}

rule members_requires_all_unless_admin {
  formula {
    always (modifies(/members) implies (all_signed(/members) | signed_by(/admin.id)))
  }
}
```

### Majority for membership changes

```modality
rule members_majority {
  formula {
    always (modifies(/members) implies threshold_signed(2, /members))
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
