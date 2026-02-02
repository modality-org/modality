---
sidebar_position: 1
title: Multi-Party Contracts
---

# Multi-Party Contract Tutorial

Create a contract where only Alice or Bob can make commits.

## Step 1: Create Identities

First, create identities in your user directory (not in the contract):

```bash
# Create passfile directory if it doesn't exist
mkdir -p ~/.modality/passfiles

# Create identities for Alice and Bob
modal id create --path ~/.modality/passfiles/alice.passfile
modal id create --path ~/.modality/passfiles/bob.passfile
```

:::tip
Store passfiles in `~/.modality/passfiles/` — never commit private keys to a contract directory!
:::

## Step 2: Create Contract

```bash
mkdir my-contract && cd my-contract

# Create the contract
modal contract create

# Initialize directories
modal c checkout
```

## Step 3: Alice Sets Up Users, Model & Authorization Rule

```bash
# Add user IDs (public keys only, not passfiles)
modal c set /users/alice.id $(modal id get --path ~/.modality/passfiles/alice.passfile)
modal c set /users/bob.id $(modal id get --path ~/.modality/passfiles/bob.passfile)
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

## Step 4: Alice Commits the Setup (Signed)

```bash
modal c commit --all --sign ~/.modality/passfiles/alice.passfile
```

From this point on, all commits must be signed by Alice or Bob.

## Step 5: Make Signed Changes

```bash
# Alice posts a message
mkdir -p state/data
echo "Hello from Alice" > state/data/message.text
modal c commit --all --sign ~/.modality/passfiles/alice.passfile

# Bob updates the message
echo "Hello from Bob" > state/data/message.text
modal c commit --all --sign ~/.modality/passfiles/bob.passfile
```

## Step 6: View Status & Log

```bash
modal c status
modal c log
```

## Directory Structure

Your contract directory contains only public data:

```
my-contract/
├── .contract/           # Internal storage
├── state/               # Data files (POST method)
│   ├── users/
│   │   ├── alice.id     # Public key only
│   │   └── bob.id       # Public key only
│   └── data/
│       └── message.text
├── model/               # Model files
│   └── default.modality
└── rules/               # Rule files
    └── auth.modality
```

Private keys stay in your home directory:

```
~/.modality/
└── passfiles/
    ├── alice.passfile   # Private key (never share!)
    └── bob.passfile     # Private key (never share!)
```

## Using Named Passfiles

If you configure passfile names, you can use the shorthand:

```bash
# Set named ID from passfile
modal c set-named-id /users/alice.id --named alice

# Commit with named passfile
modal c commit --all --sign alice
```

This looks for `~/.modality/passfiles/alice.passfile` automatically.
