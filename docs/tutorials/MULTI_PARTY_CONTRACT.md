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

# Add the model (proves the rule is satisfiable)
cat > model/default.modality << 'EOF'
export default model {
  initial idle
  
  idle -> committed [+signed_by(/users/alice.id)]
  idle -> committed [+signed_by(/users/bob.id)]
  committed -> committed [+signed_by(/users/alice.id)]
  committed -> committed [+signed_by(/users/bob.id)]
}
EOF

# Add authorization rule (temporal modal logic)
cat > rules/auth.modality << 'EOF'
export default rule {
  starting_at $PARENT
  formula {
    always (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
EOF

# Check status
modal c status
```

Output:
```
Changes in state/:
  + /users/alice.id
  + /users/bob.id

Changes in model/:
  + /model/default.modality

Changes in rules/:
  + /rules/auth.modality

  Run 'modal c commit --all' to commit changes.
```

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
```
```
Contract Status
═══════════════

  Contract ID: 12D3KooW...
  Total commits: 4
  ✅ state/ matches committed state.
```

```bash
modal c log
```
```
Contract: 12D3KooW...
Commits: 4

commit 833e8119...
Actions:
  post /data/message.text

commit bf68ec27...
Actions:
  post /data/message.text

commit 18634bc4...
Actions:
  post /users/alice.id
  post /users/bob.id
  model /model/default.modality
  rule /rules/auth.modality

commit 490a2225...
Actions:
  genesis /
```

## Full Script

```bash
#!/bin/bash
set -e

# Setup
rm -rf /tmp/alice-bob-contract
mkdir -p /tmp/alice-bob-contract && cd /tmp/alice-bob-contract

# Create contract
modal contract create

# Create identities
modal id create --path alice.passfile
modal id create --path bob.passfile

# Initialize directories
modal c checkout
mkdir -p state/data rules

# Alice sets up users, model, and authorization rule
modal c set /users/alice.id $(modal id get --path ./alice.passfile)
modal c set /users/bob.id $(modal id get --path ./bob.passfile)

cat > model/default.modality << 'EOF'
export default model {
  initial idle
  
  idle -> committed [+signed_by(/users/alice.id)]
  idle -> committed [+signed_by(/users/bob.id)]
  committed -> committed [+signed_by(/users/alice.id)]
  committed -> committed [+signed_by(/users/bob.id)]
}
EOF

cat > rules/auth.modality << 'EOF'
export default rule {
  starting_at $PARENT
  formula {
    always (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
EOF

# Alice commits the setup (signed)
modal c commit --all --sign alice.passfile

# Alice posts a message (signed)
echo "Hello from Alice" > state/data/message.text
modal c commit --all --sign alice.passfile

# Bob updates the message (signed)
echo "Hello from Bob" > state/data/message.text
modal c commit --all --sign bob.passfile

# Show final state
echo ""
echo "=== Contract Status ==="
modal c status

echo ""
echo "=== Contract Log ==="
modal c log

echo ""
echo "=== Directory Structure ==="
find state rules -type f
```

## Directory Structure

```
my-contract/
├── .contract/           # Internal storage
│   ├── config.json
│   ├── commits/
│   └── HEAD
├── state/               # Data files (POST method)
│   ├── users/
│   │   ├── alice.id
│   │   └── bob.id
│   └── data/
│       └── message.text
├── model/               # Model files (MODEL method)
│   └── default.modality
├── rules/               # Rule files (RULE method)
│   └── auth.modality
├── alice.passfile
└── bob.passfile
```

## Methods

| Directory | Method | Purpose |
|-----------|--------|---------|
| `state/`  | `post` | Data files (.id, .text, .json, etc.) |
| `model/`  | `model` | State machines proving rule satisfiability |
| `rules/`  | `rule` | Modality formulas (.modality) |

## Workflow Summary

| Command | Purpose |
|---------|---------|
| `modal c checkout` | Populate state/ and rules/ from commits |
| `modal c status` | Show contract info + changes |
| `modal c diff` | Show only changes |
| `modal c commit --all` | Commit all changes |
| `modal c commit --all --sign X.passfile` | Commit with signature |
| `modal c log` | Show commit history |
| `modal c push` | Sync with network |
