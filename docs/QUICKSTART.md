# Modality Quickstart

Get from zero to verified contract in 5 minutes.

---

## Install

```bash
# Clone the repo
git clone https://github.com/modality-org/modality.git
cd modality/rust

# Build
cargo build --release

# Add to path
export PATH="$PATH:$(pwd)/target/release"
```

---

## Your First Contract

### 1. Create Contract & Identities

```bash
mkdir my-contract && cd my-contract

# Create the contract
modal contract create

# Create identities
modal id create --path alice.passfile
modal id create --path bob.passfile
```

### 2. Set Up State, Model & Rules

```bash
# Initialize directories
modal c checkout
mkdir -p rules

# Add identities to state
modal c set-named-id /users/alice.id alice
modal c set-named-id /users/bob.id bob
```

Create **model/default.modality** — proves the rules can be satisfied:
```
export default model {
  initial idle
  
  idle -> committed [+signed_by(/users/alice.id)]
  idle -> committed [+signed_by(/users/bob.id)]
  committed -> committed [+signed_by(/users/alice.id)]
  committed -> committed [+signed_by(/users/bob.id)]
}
```

Create **rules/auth.modality** — the constraints:
```modality
export default rule {
  starting_at $PARENT
  formula {
    always must (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
```

### 3. Commit (Signed)

```bash
modal c commit --all --sign alice.passfile
```

From now on, all commits must be signed by Alice or Bob.

### 4. Make Changes

```bash
# Alice writes a message
mkdir -p state/data
echo "Hello from Alice" > state/data/message.text
modal c commit --all --sign alice.passfile

# Bob updates it
echo "Hello from Bob" > state/data/message.text
modal c commit --all --sign bob.passfile
```

### 5. View Status

```bash
modal c status
modal c log
```

---

## Directory Structure

```
my-contract/
├── .contract/           # Internal storage
│   ├── config.json
│   ├── commits/
│   └── HEAD
├── state/               # Data files (POST method)
│   └── users/
│       ├── alice.id
│       └── bob.id
├── model/               # State machines (MODEL method)
│   └── default.modality
├── rules/               # Formulas (RULE method)
│   └── auth.modality
```

---

## Key Concepts

| Concept | Meaning |
|---------|---------|
| `state/` | Data files — identities, messages, balances |
| `model/` | State machines proving rules are satisfiable |
| `rules/` | Formulas that must hold over the model |
| `->` | Transition from one state to another |
| `[+predicate]` | Predicate that must be satisfied |
| `signed_by(/path)` | Cryptographic signature verification |
| `starting_at $PARENT` | Rule applies from this commit forward |

---

## Workflow

| Command | Purpose |
|---------|---------|
| `modal c checkout` | Populate state/, model/, rules/ from commits |
| `modal c status` | Show contract info + changes |
| `modal c diff` | Show only changes |
| `modal c commit --all` | Commit all changes |
| `modal c commit --all --sign X.passfile` | Commit with signature |
| `modal c log` | Show commit history |

---

## The Model Requirement

When you add a rule, you must provide a **model** that proves all rules are satisfiable:

```
model/default.modality → The state machine (proof of satisfiability)
rules/auth.modality    → The formula
```

No valid model = commit rejected. This prevents contradictory or impossible rules.

---

## Next Steps

1. Read [FOR_AGENTS.md](FOR_AGENTS.md) - Why verification matters for agents
2. Follow [MULTI_PARTY_CONTRACT.md](./tutorials/MULTI_PARTY_CONTRACT.md) - Full tutorial
3. Join Discord - Get help, share ideas

---

*Questions? Issues? Open a GitHub issue or ask on Discord.* 🔐
