# Contract Log: The Append-Only Model

A Modality contract is an **append-only log of signed commits**. Every commit is a transition in the governing model. The contract's state is derived by replaying the log.

## Core Principles

1. **The log is the source of truth.** There is no separate "state" — state is derived by replaying commits.

2. **Every commit is a transition.** Each commit executes an action that must be a valid transition in the governing model.

3. **Rules are transitions too.** `add_rule` transitions as `+ADD_RULE` in the model. Rules accumulate and constrain the contract.

4. **The model defines what's possible.** Transitions specify required properties (e.g., `+by_A` for signer constraints).

5. **Signatures are implicit parties.** If you sign a commit, you're a party. No separate "add party" needed.

## Syntax

### Contract Structure

```modality
contract <name> {
  commit { ... }
  commit { ... }
  ...
}
```

### Commit Structure

```modality
commit {
  signed_by <party> "<signature>"   // Who signs + signature digest
  model { ... }                     // Optional: new/updated governing model
  add_rule { <formula> }            // Add a rule (transitions as +ADD_RULE)
  do +ACTION                        // Domain action
}
```

### Governing Model

The model defines valid transitions. Each commit must match a transition.

```modality
model {
  part flow {
    init --> state1: +ACTION +property
    state1 --> state2: +ACTION +property
    ...
  }
}
```

## Complete Example: Handshake

Two agents shake hands with enforced rules:

```modality
contract handshake {

  // Commit 0: A creates contract, provides model, adds rule
  commit {
    signed_by A "0xA_SIG_0"
    model {
      part flow {
        init --> a_ruled: +ADD_RULE +signed_by(A)
        a_ruled --> b_ruled: +ADD_RULE +signed_by(B)
        b_ruled --> a_ready: +READY +signed_by(A)
        a_ready --> done: +READY +signed_by(B)
      }
    }
    add_rule { eventually(done) }
  }

  // Commit 1: B adds their rule
  commit {
    signed_by B "0xB_SIG_1"
    add_rule { eventually(done) }
  }

  // Commit 2: A executes
  commit {
    signed_by A "0xA_SIG_2"
    do +READY
  }

  // Commit 3: B executes
  commit {
    signed_by B "0xB_SIG_3"
    do +READY
  }

}
```

### How It Works

1. **Commit 0**: A provides the governing model and adds a rule. This transitions `init --> a_ruled` via `+ADD_RULE +signed_by(A)`.

2. **Commit 1**: B adds their rule. This transitions `a_ruled --> b_ruled` via `+ADD_RULE +signed_by(B)`.

3. **Commit 2**: A executes `+READY`. This transitions `b_ruled --> a_ready` via `+READY +signed_by(A)`.

4. **Commit 3**: B executes `+READY`. This transitions `a_ready --> done` via `+READY +signed_by(B)`.

Both rules (`eventually(done)`) are now satisfied.

## Action Types

| Syntax | Model Transition | Purpose |
|--------|------------------|---------|
| `add_rule { formula }` | `+ADD_RULE` | Add a formula constraint |
| `do +ACTION` | `+ACTION` | Execute domain action |

## Validation

Each commit is validated:

1. **Transition exists**: The action must match a valid transition from current state
2. **Properties match**: Commit properties (e.g., `signed_by A` → `+by_A`) must match transition requirements
3. **Rules satisfied**: All accumulated rules must remain satisfiable

## Model Updates

Any commit can provide a new model:

```modality
commit {
  signed_by A
  model {
    // New/updated model
  }
  do +ACTION
}
```

The new model must satisfy all accumulated rules.

## Formula Syntax

```modality
// Temporal
eventually(P)       // P will be true
always(P)           // P is always true
P until Q           // P holds until Q
next(P)             // P in next state

// Modal
[+ACTION] P         // After every ACTION, P holds
<+ACTION> P         // After some ACTION, P holds

// Logical
P and Q             // Conjunction
P or Q              // Disjunction
not P               // Negation
P -> Q              // Implication
```

## Implementation

See:
- `rust/modality-lang/src/ast.rs` — Contract, ContractCommit, CommitStatement
- `rust/modality-lang/src/grammar.lalrpop` — Parser
- `rust/modality-lang/src/lalrpop_parser.rs` — parse_contract_content()
- `examples/handshake.modality` — Working example

## CLI

```bash
modality contract parse examples/handshake.modality
```

Output:
```
Parsing: examples/handshake.modality

✓ Contract: handshake
  Commits: 4

  Commit 0:
    signed_by: A
    model: (provided)
    add_rule: { <formula> }

  Commit 1:
    signed_by: B
    add_rule: { <formula> }

  Commit 2:
    signed_by: A
    do: +READY +by_A

  Commit 3:
    signed_by: B
    do: +READY +by_B

✓ Contract is valid.
```
