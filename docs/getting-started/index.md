---
sidebar_position: 1
title: Overview
---

# Getting Started with Modality

Modality is a verification language for agential cooperation. A contract is
**state + model + rules**. Commits are signed. Anyone can replay the log.
Invalid commits are rejected.

Formal verification already made machines at enormous scale reliable. This
language points that same discipline at agreements: who may do what, when,
and with what evidence — including after the process that promised it is
gone, and including with a counterparty you have never met.

## Why Modality?

Agents (and humans) need checkable terms, not prompts or policy PDFs:

- **Negotiate** cooperation as shared rules
- **Prove** a commit satisfied those rules
- **Replay** the log without asking the original host to be honest

Modality does that with:

- **Models** — labeled transition systems of possible moves
- **Rules** — temporal formulas that accumulate
- **Predicates** — signatures, evidence, and other commit-time checks
- **Append-only logs** — signed history anyone can verify

## Where can I use Modality?

You can add Modality to almost any software project — the same way you might add a database. Your application keeps its own UI, business logic, and infrastructure; Modality sits alongside it as a **verification layer** for cooperation: who can do what, when, and with what proof.

| Integration style | Best for | How it works |
|-------------------|----------|--------------|
| **CLI (`modal`)** | Local development, scripting, ops | Manage contracts and identities from the terminal — like using `psql` or `mongosh` against a database. |
| **TypeScript/JavaScript SDK** | Web apps, agents, backends | Create contracts, sign commits, and verify rules programmatically via [`@modality-org/sdk`](https://www.npmjs.com/package/@modality-org/sdk). |
| **Contract hub (HTTP)** | Multi-party collaboration | Run or connect to a hub server for push/pull workflows — similar to using a hosted database instead of a local file. See the [Contract Hub tutorial](../tutorials/contract-hub). |
| **Rust libraries** | Native services | Embed `modality-lang`, `modality-common`, and related crates directly in Rust binaries. |
| **WASM / browser** | Client-side verification | Parse and check models in the browser via `@modality-dev/wasm` (`modality-lang` compiled to WASM). |

**Local-first.** A contract starts as files on disk (`state/`, `model/`, `rules/`, commit history) — comparable to a git repo. When parties need to share, you sync to a hub.

**Typical embedding patterns:**

- **Agent frameworks** — wrap tool calls in Modality rules so agents can only take verified actions (see [For Agents](../for-agents)).
- **Backend services** — validate incoming requests against contract state before executing side effects.
- **Multi-party workflows** — escrow, multisig treasuries, membership-gated contracts (see [Tutorials](../tutorials/contract-hub)).
- **DevOps / CI** — commit and push contract changes as part of a deployment pipeline.

You do not need to rewrite your stack. Pick the surface that fits: CLI for exploration, SDK for application code, hub when parties need to share verified state.

## Next

1. [Install `modal`](./installation.md)
2. [Write your first contract](./first-contract.md)
