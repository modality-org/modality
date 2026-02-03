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

## The Rules

```modality
// Any commit must be signed by at least one member
rule member_required {
  formula {
    always (any_signed(/members))
  }
}

// Adding a member requires ALL current members to sign
rule add_member_unanimous {
  formula {
    always ([+ADD_MEMBER] implies all_signed(/members))
  }
}
```

### How the formulas work

- `any_signed(/members)` — scans `/members/*.id`, requires at least ONE signer
- `all_signed(/members)` — scans `/members/*.id`, requires ALL signers
- `[+ADD_MEMBER] implies ...` — when ADD_MEMBER action, the condition must hold

## Walkthrough

### 1. Alice creates the contract

Alice is the founder. She creates the contract and adds herself as the first member:

```bash
modal contract create --id shared_notes

# Alice adds herself
modal contract commit \
  --method post \
  --path /members/alice.id \
  --value "$(modal identity show alice --public-key)" \
  --sign alice
```

### 2. Add the rules

```bash
# Rule: only members can commit
modal contract commit \
  --method rule \
  --value 'rule member_required { formula { always (any_signed(/members)) } }' \
  --sign alice

# Rule: adding members needs everyone
modal contract commit \
  --method rule \
  --value 'rule add_member_unanimous { formula { always ([+ADD_MEMBER] implies all_signed(/members)) } }' \
  --sign alice
```

### 3. Alice adds Bob

Alice is the only member, so only she needs to sign:

```bash
# Add Bob's identity
modal contract commit \
  --method post \
  --path /members/bob.id \
  --value "$(modal identity show bob --public-key)" \
  --sign alice

# Record the ADD_MEMBER action
modal contract commit \
  --method action \
  --action ADD_MEMBER \
  --params '{"name": "bob"}' \
  --sign alice
```

✓ Passes: `all_signed(/members)` = just alice, and alice signed.

### 4. Alice and Bob add Carol

Now BOTH must sign:

```bash
# Add Carol's identity (both sign)
modal contract commit \
  --method post \
  --path /members/carol.id \
  --value "$(modal identity show carol --public-key)" \
  --sign alice \
  --sign bob

# Record the action (both sign)
modal contract commit \
  --method action \
  --action ADD_MEMBER \
  --params '{"name": "carol"}' \
  --sign alice \
  --sign bob
```

✓ Passes: `all_signed(/members)` = alice + bob, both signed.

### 5. Any member can post

Regular posts just need one member:

```bash
modal contract commit \
  --method post \
  --path /notes/meeting-2024-01.md \
  --value "# Meeting Notes\n\nDiscussed roadmap..." \
  --sign bob
```

✓ Passes: `any_signed(/members)` includes bob.

### 6. Non-members are rejected

```bash
modal contract commit \
  --method post \
  --path /notes/hack.md \
  --value "Unauthorized!" \
  --sign stranger
```

✗ Rejected: `any_signed(/members)` fails — stranger not in `/members/*.id`

### 7. Partial signatures rejected for ADD_MEMBER

```bash
# Try to add Dave with only 2 of 3 signatures
modal contract commit \
  --method post \
  --path /members/dave.id \
  --value "$(modal identity show dave --public-key)" \
  --sign alice \
  --sign bob
  # Missing carol!

modal contract commit \
  --method action \
  --action ADD_MEMBER \
  --params '{"name": "dave"}' \
  --sign alice \
  --sign bob
```

✗ Rejected: `all_signed(/members)` = alice + bob + carol, missing carol.

## Final State

```
/members/
  alice.id → "abc123..."
  bob.id → "def456..."
  carol.id → "ghi789..."
/notes/
  meeting-2024-01.md → "# Meeting Notes..."
```

## Key Concepts

| Formula | Meaning |
|---------|---------|
| `any_signed(/members)` | At least one member signed |
| `all_signed(/members)` | Every member signed |
| `[+ACTION] implies X` | If ACTION occurs, X must hold |
| `always (...)` | Must hold for all commits |

## Why This Works

1. **Members as files** — `/members/*.id` pattern lets the formula scan dynamically
2. **Persistent rules** — Added via RULE commits, apply to all future commits
3. **Formula evaluation** — Resolves `/members` to current member list each time
4. **Unanimous consent** — `all_signed` grows stricter as members are added

## Variations

### 2-of-3 Multisig

```modality
rule two_of_three {
  formula {
    always (signed_by_n(2, [/members/alice.id, /members/bob.id, /members/carol.id]))
  }
}
```

### Threshold for Adding Members

```modality
// Majority can add new members (not unanimous)
rule add_member_majority {
  formula {
    always ([+ADD_MEMBER] implies signed_by_n(2, /members))
  }
}
```

### Admin Override

```modality
rule admin_or_members {
  formula {
    always (signed_by(/admin.id) | any_signed(/members))
  }
}
```
