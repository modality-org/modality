# Members-Only Contract

A contract where only members can post, and adding new members requires unanimous consent.

Rules are encoded in commits via `rule_for_this_commit`, not in hub handlers.

## State Structure

```
/members.json → ["alice_key", "bob_key", "carol_key"]
```

## Formula Types

- `any_signed(/members.json)` - at least one member must sign
- `all_signed(/members.json)` - ALL members must sign

## Commit Structure

Every commit specifies its signature requirement:

```json
{
  "head": {
    "rule_for_this_commit": "any_signed(/members.json)",
    "signatures": [{"signer": "bob_key", "sig": "..."}]
  },
  "body": [...]
}
```

Hub evaluates `rule_for_this_commit` against signatures and contract state.

## Example: Initialize Contract

First commit has no members yet, so anyone can create:

```json
{
  "head": {
    "signatures": [{"signer": "alice_key", "sig": "..."}]
  },
  "body": [
    {"method": "post", "path": "/members.json", "value": ["alice_key"]}
  ]
}
```

## Example: Add Second Member

Alice is the only member, so she alone can add Bob:

```json
{
  "head": {
    "rule_for_this_commit": "all_signed(/members.json)",
    "signatures": [{"signer": "alice_key", "sig": "..."}]
  },
  "body": [
    {"method": "post", "path": "/members.json", "value": ["alice_key", "bob_key"]}
  ]
}
```

## Example: Add Third Member

Now both Alice and Bob must sign:

```json
{
  "head": {
    "rule_for_this_commit": "all_signed(/members.json)",
    "signatures": [
      {"signer": "alice_key", "sig": "..."},
      {"signer": "bob_key", "sig": "..."}
    ]
  },
  "body": [
    {"method": "post", "path": "/members.json", "value": ["alice_key", "bob_key", "carol_key"]}
  ]
}
```

## Example: Member Posts Data

Any member can post (using `any_signed`):

```json
{
  "head": {
    "rule_for_this_commit": "any_signed(/members.json)",
    "signatures": [{"signer": "bob_key", "sig": "..."}]
  },
  "body": [
    {"method": "post", "path": "/data/message.txt", "value": "Hello from Bob"}
  ]
}
```

## Example: Non-Member Rejected

Stranger tries to post:

```json
{
  "head": {
    "rule_for_this_commit": "any_signed(/members.json)",
    "signatures": [{"signer": "stranger_key", "sig": "..."}]
  },
  "body": [
    {"method": "post", "path": "/data/hack.txt", "value": "Unauthorized!"}
  ]
}
```

**Hub rejects:** `stranger_key` not in `/members.json`

## Example: Partial Signatures Rejected

Adding Dave but Carol didn't sign:

```json
{
  "head": {
    "rule_for_this_commit": "all_signed(/members.json)",
    "signatures": [
      {"signer": "alice_key", "sig": "..."},
      {"signer": "bob_key", "sig": "..."}
    ]
  },
  "body": [
    {"method": "post", "path": "/members.json", "value": ["alice_key", "bob_key", "carol_key", "dave_key"]}
  ]
}
```

**Hub rejects:** `all_signed(/members.json)` requires carol_key

## Key Points

1. **Rules in commits, not hub** - each commit declares its signature requirement
2. **Formulas resolve state** - `all_signed(/members.json)` reads current members
3. **Hub evaluates formulas** - `validate_rule_for_this_commit_with_state()`
4. **Dynamic membership** - list grows/shrinks, formula always checks current state
5. **No custom handlers** - generic formula evaluation handles all cases

## Formula Reference

| Formula | Meaning |
|---------|---------|
| `signed_by(key)` | Must be signed by specific key |
| `signed_by_n(n, [k1, k2, ...])` | At least n of listed keys |
| `any_signed(/path.json)` | At least one value from array at path |
| `all_signed(/path.json)` | All values from array at path |
| `f1 & f2` | Both formulas must hold |
| `f1 \| f2` | Either formula must hold |
