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

## 4. Add Protection Rules

Create the `rules/` directory:

```bash
mkdir -p rules
```

Then create `rules/authorized.modality` with the following Modality code. This
is file content, not a terminal command.

```modality
export default rule {
  starting_at $PARENT
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
```

This rule starts after the bootstrap transition. The `[]` prefix matters:
`always(...)` includes the current witness node, while `[] always(...)` lets the
initial commit install the identity evidence and then constrains every successor
model so it does not expose an unsigned transition.

## 5. Synthesize the Witness Model

The governing model should be generated from the accumulated rules before you
commit to the contract. Until commit-time synthesis is fully automatic, review
the synthesized candidate and write it to `model/default.modality`.

```bash
mkdir -p model
mkdir -p review
modality model synthesize \
  --rule rules/authorized.modality \
  --verify \
  --review-bundle review/authorized.md \
  -o model/default.modality
modality model validate model/default.modality --verbose
```

For this first contract, the witness model should have this shape:

```modality
model Contract {
  part flow {
    q0 --> q1: +POST +MODEL
    q1 --> q1: +POST +signed_by(/parties/alice.id)
    q1 --> q1: +POST +signed_by(/parties/bob.id)
    q1 --> q1: +MODEL +signed_by(/parties/alice.id)
  }
}
```

`q0` and `q1` are witness nodes. The first transition admits the bootstrap
commit that installs identity evidence and the first model. After that, the
model exposes only signed `POST` and `MODEL` paths for ordinary state changes
and model replacement.

The validation command should report `Contract is valid!` and `Transitions: 4`.
The review bundle should preserve the rule file, parser-backed extracted facts,
passed verifier result, witness model, assumptions, and known gaps before the
model is committed.

## 6. Commit and Verify

```bash
modal c commit --all --sign alice.mod_passfile -m "Initial contract setup"
modal c status
modal c log
```

`modal c log` should show Alice's signer ID plus the commit message:
`Initial contract setup`.

At this point the accepted rule, witness model, and synthesis review bundle are
the local files that explain and govern the next transition. Keep
`rules/authorized.modality`, `model/default.modality`, and
`review/authorized.md` with the contract; the smoke test checks that a rejected
commit does not alter those accepted artifacts.

## 7. Prove the Rule Is Active

Now make one ordinary signed state update:

```bash
modal c commit \
  --path /notes.text \
  --value "signed update" \
  --sign alice.mod_passfile \
  -m "Signed update"
```

`modal c status` should still report `Model state: q1`, and `modal c log`
should show a new `Signed update` entry with Alice's signer ID.

Then try the same kind of update without a signature:

```bash
modal c commit \
  --path /unsigned.text \
  --value "unsigned update" \
  -m "Unsigned update"
```

The unsigned commit should be rejected from `q1` with missing `signed_by`
predicate diagnostics. That rejection is the contract enforcing the
post-bootstrap rule you added in `rules/authorized.modality`; `modal c status`
should still show `Total commits: 3` and `Model state: q1`, and `modal c log`
should still end at the last accepted signed update. After `modal c checkout`,
the replayed `state/notes.text` file should still contain `signed update`, and
there should be no `state/unsigned.text` file. The accepted
`rules/authorized.modality`, `model/default.modality`, and
`review/authorized.md` files should also be unchanged.

## What's Next?

- [Core Concepts](/docs/concepts) — Understand the theory
- [CLI Reference](/docs/cli) — All commands explained
- [Language Reference](/docs/language) — Model and rule syntax
