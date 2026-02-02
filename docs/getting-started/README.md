# Getting Started with Modality

Modality is a verification language for AI agent cooperation. It lets agents define cooperation protocols as state machines with temporal modal formulas that constrain behavior.

## Why Modality?

In a world of AI agents making deals, "trust me" isn't good enough. Agents need to:
- **Negotiate** cooperation terms formally
- **Prove** their commitments mathematically
- **Verify** that all parties will behave as agreed

Modality makes this possible through:
- **State machines** that model allowed behaviors
- **Temporal logic** that expresses constraints over time
- **Cryptographic predicates** that bind real-world identity to actions
- **Append-only contracts** where rules accumulate and potential shrinks to mutual agreement

## Installation

```bash
# Clone the repo
git clone https://github.com/modality-org/modality.git
cd modality/rust

# Build
cargo build --release

# Add to path
export PATH="$PATH:$(pwd)/target/release"

# Verify installation
modal --version
```

## Your First Contract in 5 Minutes

### 1. Create a Contract

```bash
mkdir my-escrow && cd my-escrow
modal contract create
```

This creates a `.contract/` directory to track commits.

### 2. Create Identities

```bash
modal id create --path alice.passfile
modal id create --path bob.passfile
```

### 3. Initialize State

```bash
modal c checkout
mkdir -p state rules

# Add party identities
modal c set /parties/alice.id $(modal id get --path ./alice.passfile)
modal c set /parties/bob.id $(modal id get --path ./bob.passfile)
```

### 4. Define the Model

Create `model/escrow.modality`:

```modality
model escrow {
  states { pending, funded, delivered, released, refunded }
  initial pending
  terminal released, refunded
  
  transition DEPOSIT: pending -> funded
    +signed_by(/parties/alice.id)
  
  transition DELIVER: funded -> delivered
    +signed_by(/parties/bob.id)
  
  transition RELEASE: delivered -> released
    +signed_by(/parties/alice.id)
  
  transition REFUND: funded -> refunded
    +signed_by(/parties/bob.id)
}
```

### 5. Add Protection Rules

Create `rules/alice-protection.modality`:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [<+RELEASE>] eventually delivered
    )
  }
}
```

This rule says: Alice can only release funds if delivery has happened.

### 6. Commit and Verify

```bash
# Commit all changes
modal c commit --all --sign alice.passfile -m "Initial escrow setup"

# Check status
modal c status
```

## What's Next?

- **[Core Concepts](../concepts/README.md)** — Understand the theory
- **[CLI Reference](../cli/README.md)** — All commands explained
- **[Language Reference](../language/README.md)** — Model and rule syntax
- **[Tutorials](../tutorials/)** — Step-by-step guides
