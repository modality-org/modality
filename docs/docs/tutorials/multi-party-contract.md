---
sidebar_position: 1
title: Multi-Party Contracts
---

# Multi-Party Contract Tutorial

Create a contract where only Alice or Bob can make commits.

## Step 1: Create Identities

Create named identities (stored automatically in `~/.modality/`):

```bash
modal id create --name alice
modal id create --name bob
```

This creates:
- `~/.modality/alice.mod_passfile`
- `~/.modality/bob.mod_passfile`

:::tip
Named passfiles are stored in `~/.modality/` by default — never in the contract directory!
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
# Add user IDs using named passfiles
modal c set-named-id /users/alice.id --named alice
modal c set-named-id /users/bob.id --named bob
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
modal c commit --all --sign alice
```

From this point on, all commits must be signed by Alice or Bob.

## Step 5: Make Signed Changes

```bash
# Alice posts a message
mkdir -p state/data
echo "Hello from Alice" > state/data/message.text
modal c commit --all --sign alice

# Bob updates the message
echo "Hello from Bob" > state/data/message.text
modal c commit --all --sign bob
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
├── state/               # Data files
│   ├── users/
│   │   ├── alice.id     # Public key only
│   │   └── bob.id       # Public key only
│   └── data/
│       └── message.text
├── model/
│   └── default.modality
└── rules/
    └── auth.modality
```

Private keys stay in your home directory:

```
~/.modality/
├── alice.mod_passfile   # Private key (never share!)
└── bob.mod_passfile     # Private key (never share!)
```
