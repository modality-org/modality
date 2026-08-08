---
sidebar_position: 3
title: Your First Contract
---

# Your First Contract

This page walks through creating a small local contract with the `modal`
contract CLI. If you only have a `modality` command, install `modal` from the
[installation guide](./installation.md) first.

## 1. Create a Contract

```bash
mkdir my-first-contract
cd my-first-contract
modal contract create
```

This creates a `.contract/` directory and a starter `model/default.modality`
file.

## 2. Create Identities

```bash
modal id create --path alice.mod_passfile
modal id create --path bob.mod_passfile
```

The passfiles contain private keys. Keep them local and do not commit them.

## 3. Add Identities to Contract State

```bash
modal c checkout
modal c set-named-id /parties/alice.id alice.mod_passfile
modal c set-named-id /parties/bob.id bob.mod_passfile
```

This records Alice and Bob's public identities in the contract state.

## 4. Define the Witness Model

Replace `model/default.modality` with the following Modality code. This is file
content, not a terminal command.

```modality
export default model {
  initial q0

  q0 -> q1 [+signed_by(/parties/alice.id)]
  q1 -> q2 [+signed_by(/parties/bob.id)]
  q2 -> q2 [+signed_by(/parties/alice.id)]
  q2 -> q2 [+signed_by(/parties/bob.id)]
}
```

`q0`, `q1`, and `q2` are witness nodes. The predicates on each transition say
which signatures are required as the contract evolves.

## 5. Add Protection Rules

Create the `rules/` directory:

```bash
mkdir -p rules
```

Then create `rules/authorized.modality` with the following Modality code. This
is also file content, not a terminal command.

```modality
export default rule {
  starting_at $PARENT
  formula {
    always(+signed_by(/parties/alice.id) | +signed_by(/parties/bob.id))
  }
}
```

This rule says every valid model must require commits to be signed by Alice or
Bob. At commit time, the transition predicates in `model/default.modality` are
what the verifier checks against the signed commit.

## 6. Commit and Verify

```bash
modal c commit --all --sign alice.mod_passfile -m "Initial contract setup"
modal c status
modal c log
```

`modal c log` should show Alice's signer ID and the commit message.

## What's Next?

- [Core Concepts](/docs/concepts) — Understand the theory
- [CLI Reference](/docs/cli) — All commands explained
- [Language Reference](/docs/language) — Model and rule syntax
