# Members-Only Contract

Only members can post. Adding members requires unanimous consent.

**Rules are persistent** - defined in contract, enforced by hub on all commits.

## State Structure

```
/members/
  alice.id → "alice_pubkey"
  bob.id → "bob_pubkey"
  carol.id → "carol_pubkey"
```

## Contract Rules

```modality
rule member_required {
  formula { always (any_signed(/members)) }
}

rule add_member_unanimous {
  formula { always ([+ADD_MEMBER] implies all_signed(/members)) }
}
```

## How It Works

1. Rules added via RULE commits accumulate in contract
2. Hub evaluates EVERY new commit against ALL accumulated rules
3. `any_signed(/members)` scans `/members/*.id` for pubkeys
4. `all_signed(/members)` requires ALL those pubkeys as signers

## Example Workflow

### 1. Create Contract (First Member)

```bash
# Alice creates contract and adds herself as first member
modal contract create --id members_contract

modal contract commit --method post \
  --path /members/alice.id \
  --value "$ALICE_KEY" \
  --sign alice

# Add the rules
modal contract commit --method rule \
  --value 'rule member_required { formula { always (any_signed(/members)) } }' \
  --sign alice

modal contract commit --method rule \
  --value 'rule add_member_unanimous { formula { always ([+ADD_MEMBER] implies all_signed(/members)) } }' \
  --sign alice

modal contract push
```

### 2. Add Bob (Alice Signs)

```bash
# Alice is the only member, so only she needs to sign
modal contract commit --method post \
  --path /members/bob.id \
  --value "$BOB_KEY" \
  --sign alice

modal contract commit --method action --action ADD_MEMBER \
  --params '{"member": "bob"}' \
  --sign alice

modal contract push  # ✓ Passes: all_signed(/members) = [alice] ✓
```

### 3. Add Carol (Alice + Bob Sign)

```bash
# Now both must sign for ADD_MEMBER
modal contract commit --method post \
  --path /members/carol.id \
  --value "$CAROL_KEY" \
  --sign alice --sign bob

modal contract commit --method action --action ADD_MEMBER \
  --params '{"member": "carol"}' \
  --sign alice --sign bob

modal contract push  # ✓ Passes: all_signed(/members) = [alice, bob] ✓
```

### 4. Bob Posts Data

```bash
# Any member can post (any_signed)
modal contract commit --method post \
  --path /data/message.txt \
  --value "Hello from Bob" \
  --sign bob

modal contract push  # ✓ Passes: any_signed(/members) includes bob ✓
```

### 5. Stranger Rejected

```bash
# Stranger not in /members/*.id
modal contract commit --method post \
  --path /data/hack.txt \
  --value "Unauthorized!" \
  --sign stranger

modal contract push
# ✗ Rejected: any_signed(/members) fails - stranger not a member
```

### 6. Partial Signatures Rejected

```bash
# Adding Dave without Carol's signature
modal contract commit --method post \
  --path /members/dave.id \
  --value "$DAVE_KEY" \
  --sign alice --sign bob  # Missing carol

modal contract commit --method action --action ADD_MEMBER \
  --params '{"member": "dave"}' \
  --sign alice --sign bob

modal contract push
# ✗ Rejected: all_signed(/members) = [alice, bob, carol], missing carol
```

## Formula Resolution

`all_signed(/members)` resolves by:
1. Scanning contract state for keys matching `members/*.id`
2. Extracting the string values (pubkeys)
3. Checking ALL are present in commit signatures

```rust
// State:
// "members/alice.id": "alice_key"
// "members/bob.id": "bob_key"

resolve_path_as_strings(state, "/members")
// → ["alice_key", "bob_key"]
```

## Key Points

1. **Rules in contract** - not per-commit, not in hub code
2. **Directory pattern** - `/members/*.id` not `/members.json`
3. **Hub enforces** - evaluates all rules on every commit
4. **Dynamic membership** - formula resolves current state each time
5. **Unanimous add** - `[+ADD_MEMBER] implies all_signed(/members)`
