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

```modality
model members_only {
  initial active
  
  // Modifying /data/* requires any member signature
  active -> active [+any_signed(/members) +modifies(/data)]
  
  // Modifying /members/* requires ALL member signatures  
  active -> active [+all_signed(/members) +modifies(/members)]
}
```

### How the predicates work

| Predicate | Meaning |
|-----------|---------|
| `any_signed(/members)` | At least one signer is in `/members/*.id` |
| `all_signed(/members)` | All signers from `/members/*.id` are present |
| `modifies(/data)` | Commit touches paths under `/data/` |
| `modifies(/members)` | Commit touches paths under `/members/` |

Transitions combine predicates: the commit must satisfy ALL predicates on the transition.

## Walkthrough

### 1. Alice creates the contract

Alice is the founder. She creates the contract and adds herself as the first member:

```bash
modal contract create --id shared_notes

# Alice adds herself (no existing members, so allowed)
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
  --value 'model members_only {
    initial active
    active -> active [+any_signed(/members) +modifies(/data)]
    active -> active [+all_signed(/members) +modifies(/members)]
  }' \
  --sign alice
```

### 3. Alice adds Bob

Alice is the only member, so only she needs to sign:

```bash
modal contract commit \
  --method post \
  --path /members/bob.id \
  --value "$(modal identity show bob --public-key)" \
  --sign alice
```

✓ Passes: `modifies(/members)` + `all_signed(/members)` where members = [alice]

### 4. Alice and Bob add Carol

Now BOTH must sign:

```bash
modal contract commit \
  --method post \
  --path /members/carol.id \
  --value "$(modal identity show carol --public-key)" \
  --sign alice \
  --sign bob
```

✓ Passes: `all_signed(/members)` where members = [alice, bob]

### 5. Any member can post data

```bash
modal contract commit \
  --method post \
  --path /data/meeting-notes.md \
  --value "# Meeting Notes\n\nDiscussed roadmap..." \
  --sign bob
```

✓ Passes: `modifies(/data)` + `any_signed(/members)` where bob ∈ members

### 6. Non-members are rejected

```bash
modal contract commit \
  --method post \
  --path /data/hack.md \
  --value "Unauthorized!" \
  --sign stranger
```

✗ Rejected: `any_signed(/members)` fails — stranger ∉ members

### 7. Partial signatures rejected for /members

```bash
modal contract commit \
  --method post \
  --path /members/dave.id \
  --value "$(modal identity show dave --public-key)" \
  --sign alice \
  --sign bob
  # Missing carol!
```

✗ Rejected: `all_signed(/members)` fails — carol didn't sign

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

## How Evaluation Works

When a commit arrives:

1. **Extract modified paths** from commit body
2. **Find matching transition** where `modifies(X)` matches
3. **Check signature predicates** for that transition
4. **Accept or reject** based on predicate evaluation

```rust
// Commit modifies /members/dave.id
let body = json!([{"method": "post", "path": "/members/dave.id", ...}]);

// Matches: active -> active [+all_signed(/members) +modifies(/members)]
// Check: all_signed(/members) with current signers
// Result: pass if all members signed, reject otherwise
```

## Variations

### Read-only for non-members

```modality
model public_read {
  initial active
  
  // Anyone can read (no signature needed for queries)
  // Only members can write
  active -> active [+any_signed(/members) +modifies(/)]
}
```

### Tiered permissions

```modality
model tiered {
  initial active
  
  // Admins can modify anything
  active -> active [+any_signed(/admins)]
  
  // Members can modify /data
  active -> active [+any_signed(/members) +modifies(/data)]
  
  // Adding members needs admin
  active -> active [+any_signed(/admins) +modifies(/members)]
}
```

### Majority vote for membership

```modality
model majority {
  initial active
  
  // Data: any member
  active -> active [+any_signed(/members) +modifies(/data)]
  
  // Members: majority (using threshold)
  active -> active [+threshold_signed(2, /members) +modifies(/members)]
}
```
