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

# Get their IDs
ALICE=$(cat alice.passfile | jq -r '.id')
BOB=$(cat bob.passfile | jq -r '.id')
```

## Step 2: Set Up Users via State Directory

```bash
# Initialize state directory
modal c checkout

# Create the users directory and add IDs
mkdir -p state/users
echo "$ALICE" > state/users/alice.id
echo "$BOB" > state/users/bob.id

# Check status
modal c status
```

Output:
```
Changes in state/:
  + /users/alice.id
  + /users/bob.id

  Run 'modal c commit --all' to commit changes.
```

## Step 3: Commit the Users

```bash
modal c commit --all
```

## Step 4: Add Authorization Rule

```bash
# Add a rule requiring signatures (temporal modal logic)
# $PARENT is automatically replaced with the parent commit's hash
mkdir -p rules
cat > rules/auth.modality << 'EOF'
export default rule {
  starting_since $PARENT
  formula {
    always must (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
EOF

# Commit the rule
modal c commit --all
```

## Step 5: Make Signed Changes

```bash
# Alice adds a message
mkdir -p state/data
echo "Hello from Alice" > state/data/message.text
modal c commit --all --sign alice.passfile

# Bob responds
echo "Hello from Bob" > state/data/response.text
modal c commit --all --sign bob.passfile
```

## Step 6: View Status & Log

```bash
modal c status
```
```
Contract Status
═══════════════

  Contract ID: 12D3KooW...
  Total commits: 5
  ✅ state/ matches committed state.
```

```bash
modal c log
```
```
Contract: 12D3KooW...
Commits: 5

commit 833e8119...
Actions:
  post /data/response.text

commit bf68ec27...
Actions:
  post /data/message.text

commit dee1abd8...
Actions:
  rule /rules/auth.modality

commit 18634bc4...
Actions:
  post /users/alice.id
  post /users/bob.id

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
ALICE=$(cat alice.passfile | jq -r '.id')
BOB=$(cat bob.passfile | jq -r '.id')

echo "Alice: $ALICE"
echo "Bob: $BOB"

# Initialize and add users to state/
modal c checkout
mkdir -p state/users state/data
echo "$ALICE" > state/users/alice.id
echo "$BOB" > state/users/bob.id

# Commit users
modal c commit --all

# Add authorization rule to rules/ (sibling of state/)
# $PARENT is automatically replaced with the parent commit's hash
mkdir -p rules
cat > rules/auth.modality << 'EOF'
export default rule {
  starting_since $PARENT
  formula {
    always must (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
EOF
modal c commit --all

# Alice sends a message (signed)
echo "Hello from Alice" > state/data/message.text
modal c commit --all --sign alice.passfile

# Bob responds (signed)
echo "Hello from Bob" > state/data/response.text
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
│       ├── message.text
│       └── response.text
├── rules/               # Rule files (RULE method)
│   └── auth.modality
├── alice.passfile
└── bob.passfile
```

## Methods

| Directory | Method | Purpose |
|-----------|--------|---------|
| `state/`  | `post` | Data files (.id, .text, .json, etc.) |
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
