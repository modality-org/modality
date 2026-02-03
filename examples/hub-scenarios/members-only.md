# Members-Only Contract

A contract where only members can post, and adding new members requires unanimous consent.

## State Structure

```
/members.json → ["alice_pubkey", "bob_pubkey", "carol_pubkey"]
```

## Hub Validation

1. **Any commit** → must be signed by at least one member
2. **ADD_MEMBER action** → must be signed by ALL members

## Setup

```bash
modal hub start --port 3000 --data-dir ./members-hub
```

## 1. Create Contract (First Member)

```bash
mkdir contract && cd contract
modal contract create --id members_contract

# Create founder identity
modal identity create alice
ALICE_KEY=$(modal identity show alice --public-key)

# Initialize with first member (anyone can add first member)
modal contract commit --method post \
  --path /members.json \
  --value "[\"$ALICE_KEY\"]" \
  --sign alice

modal contract remote add origin http://localhost:3000
modal contract push
```

## 2. Add Second Member (Alice Signs)

```bash
# Alice adds Bob - only Alice needs to sign (she's the only member)
modal identity create bob
BOB_KEY=$(modal identity show bob --public-key)

modal contract commit --method post \
  --path /members.json \
  --value "[\"$ALICE_KEY\", \"$BOB_KEY\"]" \
  --sign alice

modal contract commit --method action --action ADD_MEMBER \
  --params "{\"new_member\": \"$BOB_KEY\"}" \
  --sign alice

modal contract push
```

## 3. Add Third Member (Alice + Bob Sign)

```bash
modal identity create carol
CAROL_KEY=$(modal identity show carol --public-key)

# Both Alice AND Bob must sign
modal contract commit --method post \
  --path /members.json \
  --value "[\"$ALICE_KEY\", \"$BOB_KEY\", \"$CAROL_KEY\"]" \
  --sign alice --sign bob

modal contract commit --method action --action ADD_MEMBER \
  --params "{\"new_member\": \"$CAROL_KEY\"}" \
  --sign alice --sign bob

modal contract push
```

## 4. Members Can Post Data

```bash
# Bob posts some data (only his signature needed)
modal contract commit --method post \
  --path /data/hello.text \
  --value "Hello from Bob" \
  --sign bob

modal contract push  # ✓ Accepted (Bob is a member)
```

## 5. Non-Member Rejected

```bash
modal identity create stranger
STRANGER_KEY=$(modal identity show stranger --public-key)

# Stranger tries to post
modal contract commit --method post \
  --path /data/hack.text \
  --value "Unauthorized!" \
  --sign stranger

modal contract push
# Error: -32060 - Commit must be signed by a member
```

## 6. Partial Signatures Rejected for ADD_MEMBER

```bash
modal identity create dave
DAVE_KEY=$(modal identity show dave --public-key)

# Only Alice and Bob sign (Carol missing)
modal contract commit --method post \
  --path /members.json \
  --value "[\"$ALICE_KEY\", \"$BOB_KEY\", \"$CAROL_KEY\", \"$DAVE_KEY\"]" \
  --sign alice --sign bob

modal contract commit --method action --action ADD_MEMBER \
  --params "{\"new_member\": \"$DAVE_KEY\"}" \
  --sign alice --sign bob

modal contract push
# Error: -32061 - ADD_MEMBER requires all 3 members to sign
```

## Key Points

1. **`/members.json`** is the source of truth for membership
2. **First member** can be added by anyone (bootstrap)
3. **Subsequent members** require ALL existing members to sign
4. **Regular posts** need just one member's signature
5. **No path interpolation** - hub reads /members.json directly

## Error Codes

| Code | Error | Description |
|------|-------|-------------|
| -32060 | Not a member | Commit signer not in /members.json |
| -32061 | Incomplete signatures | ADD_MEMBER missing required signatures |
