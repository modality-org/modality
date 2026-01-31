# Model Synthesis Tutorial

Create a contract where you write the rules first, then synthesize a model that satisfies them.

## The Idea

Traditional flow: write model → write rules → verify rules hold on model.

**Synthesis flow**: write rules → synthesize model → verify & refine.

This is useful when you know *what* you want to guarantee but not *how* to structure the state machine.

---

## Step 1: Create Contract & Identities

```bash
mkdir synthesis-demo && cd synthesis-demo

modal contract create
modal id create --path alice.passfile
modal id create --path bob.passfile

modal c checkout
modal c set /users/alice.id $(modal id get --path ./alice.passfile)
modal c set /users/bob.id $(modal id get --path ./bob.passfile)
```

## Step 2: Write the Rule First

Instead of designing the model upfront, describe what you want:

```bash
mkdir -p rules
cat > rules/auth.modality << 'EOF'
export default rule {
  starting_at $PARENT
  formula {
    always (
      [<+signed_by(/users/alice.id)>] true | [<+signed_by(/users/bob.id)>] true
    )
  }
}
EOF
```

This says: "Every commit must be signed by Alice or Bob."

## Step 3: Synthesize the Model

Now ask the system to generate a model that satisfies this rule:

```bash
modal model synthesize --rule rules/auth.modality --output model/default.modality
```

The synthesizer analyzes the formula and generates:

```
export default model {
  initial idle
  
  idle -> idle [+signed_by(/users/alice.id)]
  idle -> idle [+signed_by(/users/bob.id)]
}
```

**How it works:**
- `always (A | B)` → need transitions with A or B from every reachable state
- Simplest satisfying model: single state with self-loops for each alternative

## Step 4: Review & Refine

The synthesized model is minimal. You might want richer states:

```bash
cat > model/default.modality << 'EOF'
export default model {
  initial idle
  
  idle -> active [+signed_by(/users/alice.id)]
  idle -> active [+signed_by(/users/bob.id)]
  active -> active [+signed_by(/users/alice.id)]
  active -> active [+signed_by(/users/bob.id)]
}
EOF
```

This adds an `active` state after the first commit.

## Step 5: Verify & Commit

```bash
# Verify the model satisfies the rule
modal model check --model model/default.modality --rule rules/auth.modality

# Commit everything
modal c commit --all --sign alice.passfile
```

---

## Synthesis Patterns

The synthesizer recognizes common patterns:

| Rule Pattern | Generated Model |
|--------------|-----------------|
| `always [<+A>] true` | Self-loop requiring +A |
| `[<+A>] true` | Linear: start → after with +A |
| `[+B] implies <+A> true` | A precedes B |
| `eventually <+A> true` | Path to state with +A |
| Alternating parties | Cycle between parties |

### Example: Escrow

**Rule:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    [release] implies <deliver> true
  }
}
```

**Synthesized model:**
```
export default model {
  initial init
  
  init -> deposited [+deposit]
  deposited -> delivered [+deliver]
  delivered -> released [+release]
}
```

The synthesizer infers: release requires deliver to have happened first → sequential states.

### Example: Multi-sig

**Rule:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    [execute] implies (
      [<+signed_by(/users/alice.id)>] true &
      [<+signed_by(/users/bob.id)>] true
    )
  }
}
```

**Synthesized model:**
```
export default model {
  initial init
  
  init -> alice_signed [+signed_by(/users/alice.id)]
  init -> bob_signed [+signed_by(/users/bob.id)]
  alice_signed -> both [+signed_by(/users/bob.id)]
  bob_signed -> both [+signed_by(/users/alice.id)]
  both -> executed [+execute]
}
```

Both signatures required before execute → branching then merge.

---

## AI-Assisted Synthesis

For complex rules, the synthesizer may ask for clarification:

```bash
modal model synthesize --rule rules/complex.modality --interactive
```

```
? Rule pattern unclear. Please clarify:
  
  Your rule: eventually(paid) & always(can refund)
  
  Options:
  1. Sequential: deposit → paid (with refund possible at any point)
  2. Branching: deposit → (paid | refunded)
  3. Let me describe in natural language
  
  Choice: 
```

The `--interactive` flag enables AI-assisted disambiguation.

---

## When to Use Synthesis

**Use synthesis when:**
- You know the invariants but not the state structure
- Exploring what models satisfy your requirements
- Bootstrapping a complex contract

**Write models directly when:**
- You have a clear mental model of states
- The state machine is domain-specific
- Performance matters (synthesis can be slow for complex rules)

---

## Full Workflow

```bash
#!/bin/bash
set -e

# Setup
mkdir -p /tmp/synthesis-demo && cd /tmp/synthesis-demo
modal contract create
modal id create --path alice.passfile
modal id create --path bob.passfile

# Initialize
modal c checkout
modal c set /users/alice.id $(modal id get --path ./alice.passfile)
modal c set /users/bob.id $(modal id get --path ./bob.passfile)

# Write rule first
mkdir -p rules
cat > rules/auth.modality << 'EOF'
export default rule {
  starting_at $PARENT
  formula {
    always (
      [<+signed_by(/users/alice.id)>] true | [<+signed_by(/users/bob.id)>] true
    )
  }
}
EOF

# Synthesize model from rule
modal model synthesize --rule rules/auth.modality --output model/default.modality

# Show what was generated
echo "=== Synthesized Model ==="
cat model/default.modality

# Verify
modal model check --model model/default.modality --rule rules/auth.modality

# Commit
modal c commit --all --sign alice.passfile

# Status
modal status
```

---

## Next Steps

1. Try synthesizing models from different rule patterns
2. Use `--interactive` for complex rules
3. Read [MULTI_PARTY_CONTRACT.md](./MULTI_PARTY_CONTRACT.md) for the traditional model-first approach
