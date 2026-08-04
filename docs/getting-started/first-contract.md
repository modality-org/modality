---
sidebar_position: 3
title: Your First Contract
---

# Your First Contract

This page tracks the canonical first-contract path. Today, the verified path is
source-built and uses the language/model CLI against a parser-backed witness
model. The local contract-log workflow is verified through the source-built
lean `modal` wrapper; the installed release wrapper path is still being brought
into line with that checked flow.

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
create identities, commit state, and inspect the log. The source-built command
modules and lean wrapper now have regression coverage for this local
identity-backed flow. The installed release wrapper path is still pending
runtime verification and should not be treated as the canonical copy-paste
onboarding path yet.

To run the current onboarding smoke bundle:

```bash
cd modality
tests/run-onboarding-smokes.sh
```

That bundle always runs the parser-backed first-contract language check. If a
source-built `rust/target/debug/modal` exists, it also runs the contract-log
wrapper smoke; otherwise it prints the exact build command for enabling that
second check. To build and run the lean wrapper path in one command:

```bash
MODAL_ONBOARDING_BUILD=1 tests/run-onboarding-smokes.sh
```

To run only the source-built wrapper smoke:

```bash
cd modality/rust
cargo build -p modal --no-default-features --features contract-onboarding
cd ..
tests/cli/run-first-contract-cli-smoke.sh
```

The target shape is:

```bash
mkdir my-contract && cd my-contract
modal contract create
modal id create --path alice.mod_passfile
modal id create --path bob.mod_passfile
modal c checkout
modal c set-named-id /parties/alice.id alice.mod_passfile
modal c set-named-id /parties/bob.id bob.mod_passfile
modal c commit --all --sign alice.mod_passfile -m "Initial contract setup"
modal c status
modal c log
```

Until the installed `modal` wrapper path has runtime verification, use the
source-built language check or lean wrapper smoke above as the verified
first-contract checks.

## What's Next?

- [Core Concepts](/docs/concepts) — Understand the theory
- [CLI Reference](/docs/cli) — All commands explained
- [Language Reference](/docs/language) — Model and rule syntax
