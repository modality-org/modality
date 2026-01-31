# Multi-Party Contract Tutorial

This tutorial shows how to create a contract with multiple authorized signers. We'll set up a contract where only Alice or Bob can make commits.

## Step 1: Create an Empty Contract

```bash
mkdir my-contract && cd my-contract
modal contract create
```

Output:
```
✅ Contract created successfully!
   Contract ID: 12D3KooW...
   Directory: /path/to/my-contract
   Genesis commit: abc123...
```

## Step 2: Create Identity Passfiles

Each party needs a passfile (keypair):

```bash
# Create Alice's identity
modal id create --path alice.passfile
# Output: Modality ID: 12D3KooWPaH8gkE...

# Create Bob's identity  
modal id create --path bob.passfile
# Output: Modality ID: 12D3KooWE1g9YPC...
```

Get the public keys:
```bash
ALICE=$(cat alice.passfile | jq -r '.id')
BOB=$(cat bob.passfile | jq -r '.id')
```

## Step 3: Register the Users

Add both users' public keys to the contract:

```bash
# Add Alice
modal c commit --path /users/alice.id --value "\"$ALICE\""

# Add Bob
modal c commit --path /users/bob.id --value "\"$BOB\""
```

## Step 4: Add the Authorization Rule

Add a rule requiring all future commits to be signed by Alice or Bob:

```bash
modal c commit --method rule --path /rules/authorized_signers.json --value '{
  "description": "All commits must be signed by Alice or Bob",
  "formula": "SIGNED_BY_ALICE | SIGNED_BY_BOB",
  "signers": {
    "ALICE": "/users/alice.id",
    "BOB": "/users/bob.id"
  }
}'
```

## Step 5: View the Contract Log

```bash
modal c log
```

Output:
```
Contract: 12D3KooW...
Commits: 4

commit d4e5f6... (d4e5f6...)
Parent: c3d4e5...
Actions:
  rule /rules/authorized_signers.json

commit c3d4e5... (c3d4e5...)
Parent: b2c3d4...
Actions:
  post /users/bob.id

commit b2c3d4... (b2c3d4...)
Parent: a1b2c3...
Actions:
  post /users/alice.id

commit a1b2c3... (a1b2c3...)
Actions:
  genesis /
```

## Step 6: Make Signed Commits

Now all commits must be signed:

```bash
# Alice signs a commit
modal c commit --path /data/message.text --value '"Hello from Alice"' --sign alice.passfile

# Bob signs a commit
modal c commit --path /data/response.text --value '"Hello from Bob"' --sign bob.passfile
```

## Step 7: Push to Network

```bash
modal c push --remote "/ip4/127.0.0.1/tcp/4040/ws/p2p/12D3KooW..."
```

## What Happens Next?

Once the authorization rule is active:

1. **Unsigned commits are rejected** by validators
2. **Commits signed by unauthorized keys are rejected**
3. **Only Alice or Bob can advance the contract state**

## Full Script

```bash
#!/bin/bash
set -e

# Setup
mkdir -p tutorial-contract && cd tutorial-contract

# Create contract
modal contract create

# Create identities
modal id create --output alice.passfile
ALICE_KEY=$(modal id create --output alice.passfile 2>&1 | grep "Peer ID" | awk '{print $3}')

modal id create --output bob.passfile  
BOB_KEY=$(modal id create --output bob.passfile 2>&1 | grep "Peer ID" | awk '{print $3}')

# Register users
modal c commit --path /users/alice.id --value "\"$ALICE_KEY\""
modal c commit --path /users/bob.id --value "\"$BOB_KEY\""

# Add authorization rule
modal c commit --method rule --path /rules/auth.json --value "{
  \"require\": \"SIGNED_BY_ALICE | SIGNED_BY_BOB\",
  \"signers\": {
    \"ALICE\": \"/users/alice.id\",
    \"BOB\": \"/users/bob.id\"
  }
}"

# Make signed commits
modal c commit --path /data/test.text --value '"Signed by Alice"' --sign alice.passfile
modal c commit --path /data/test2.text --value '"Signed by Bob"' --sign bob.passfile

# View log
modal c log
```

## Summary

| Step | Command | Purpose |
|------|---------|---------|
| 1 | `modal contract create` | Initialize empty contract |
| 2 | `modal id create` | Create keypairs for parties |
| 3 | `modal c commit --path /users/X.id` | Register authorized signers |
| 4 | `modal c commit --method rule` | Add authorization rule |
| 5 | `modal c commit --sign X.passfile` | Make signed commits |
| 6 | `modal c push` | Sync with network |

This pattern enables:
- **Multi-sig contracts** (require N of M signatures)
- **Role-based access** (different rules for different paths)
- **Governance** (voting on rule changes)
