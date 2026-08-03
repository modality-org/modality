---
sidebar_position: 3
title: Your First Contract
---

# Your First Contract

This page tracks the canonical first-contract path. Today, the verified path is
source-built and uses the language/model CLI against a parser-backed witness
model. The full contract-log workflow below is still being brought into line
with the installed `modal` CLI.

## 1. Run the Verified Source Check

```bash
git clone https://github.com/modality-org/modality.git
cd modality/tests/language
./run-onboarding-tests.sh
```

The smoke test builds the Rust language CLI from source and validates the
first-contract fixture. Passing output includes:

```text
Parts: 1
Transitions: 4
Contract is valid!
All properties are predicates (verifiable).
```

## 2. Inspect the Witness Model

The smoke test validates `tests/language/03-first-contract/first-contract.modality`:

```modality
model FirstContract {
  initial q0

  part flow {
    q0 --> q1: +signed_by(/parties/alice.id)
    q1 --> q2: +signed_by(/parties/bob.id)
    q2 --> q2: +signed_by(/parties/alice.id)
    q2 --> q2: +signed_by(/parties/bob.id)
  }
}
```

`q0`, `q1`, and `q2` are opaque witness nodes. The contract meaning lives on
the labelled transitions and predicates, here the required signatures for Alice
and Bob.

## 3. Run the Underlying CLI Command

```bash
cd modality/rust
cargo run -q -p modality -- model validate \
  ../tests/language/03-first-contract/first-contract.modality \
  --verbose
```

This is the source-built language/model CLI, not the installed contract CLI.

## Contract CLI Path

The intended contract-log flow uses `modal` commands to create a contract,
create identities, commit state, and verify the log. That path is still pending
help-output verification and should not be treated as the canonical copy-paste
onboarding path yet.

The target shape is:

```bash
mkdir my-contract && cd my-contract
modal contract create
modal id create --path alice.passfile
modal id create --path bob.passfile
modal c checkout
modal c commit --all --sign alice.passfile -m "Initial contract setup"
modal c status
```

Until this flow has a regression test, use the source-built validation command
above as the verified first-contract check.

## What's Next?

- [Core Concepts](/docs/concepts) — Understand the theory
- [CLI Reference](/docs/cli) — All commands explained
- [Language Reference](/docs/language) — Model and rule syntax
