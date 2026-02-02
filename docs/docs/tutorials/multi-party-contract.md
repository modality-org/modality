---
sidebar_position: 1
title: Multi-Party Contracts
---

# Multi-Party Contract Tutorial

Create a contract where only Alice or Bob can make commits.

## Step 1: Create Contract & Identities

```bash
mkdir my-contract && cd my-contract

# Create the contract
modal contract create

# Create identities for Alice and Bob
modal id create --path alice.passfile
modal id create --path bob.passfile
```

## Step 2: Alice Sets Up Users, Model & Authorization Rule

```bash
# Initialize directories
modal c checkout
mkdir -p rules

# Add user IDs
modal c set /users/alice.id $(modal id get --path ./alice.passfile)
modal c set /users/bob.id $(modal id get --path ./bob.passfile)
```

Create `model/default.modality`:

```modality
export default model {
  initial idle
  
  idle -> committed [+signed_by(/users/alice.id)]
  idle -> committed [+signed_by(/users/bob.id)]
  committed -> committed [+signed_by(/users/alice.id)]
  committed -> committed [+signed_by(/users/bob.id)]
}
```

Create `rules/auth.modality`:

```modality
export default rule {
  starting_at $PARENT
  formula {
    signed_by(/users/alice.id) | signed_by(/users/bob.id)
  }
}
```

This rule requires every commit to be signed by either Alice or Bob.

## Step 3: Alice Commits the Setup (Signed)

```bash
modal c commit --all --sign alice.passfile
```

From this point on, all commits must be signed by Alice or Bob.

## Step 4: Make Signed Changes

```bash
# Alice posts a message
mkdir -p state/data
echo "Hello from Alice" > state/data/message.text
modal c commit --all --sign alice.passfile

# Bob updates the message
echo "Hello from Bob" > state/data/message.text
modal c commit --all --sign bob.passfile
```

## Step 5: View Status & Log

```bash
modal c status
modal c log
```

## Directory Structure

```
my-contract/
├── .contract/           # Internal storage
├── state/               # Data files (POST method)
│   ├── users/
│   │   ├── alice.id
│   │   └── bob.id
│   └── data/
│       └── message.text
├── model/               # Model files
│   └── default.modality
├── rules/               # Rule files
│   └── auth.modality
├── alice.passfile
└── bob.passfile
```
