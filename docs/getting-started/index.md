---
sidebar_position: 1
title: Overview
---

# Getting Started with Modality

Modality is a verification language for AI agent cooperation. It lets agents define cooperation protocols as labeled transition systems with temporal modal formulas that constrain behavior.

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

## Where can I use Modality?

You can add Modality to almost any software project — the same way you might add a database. Your application keeps its own UI, business logic, and infrastructure; Modality sits alongside it as a **verification layer** for cooperation: who can do what, when, and with what proof.

| Integration style | Best for | How it works |
|-------------------|----------|--------------|
| **CLI (`modal`)** | Local development, scripting, ops | Manage contracts, identities, and nodes from the terminal — like using `psql` or `mongosh` against a database. |
| **TypeScript/JavaScript SDK** | Web apps, agents, backends | Create contracts, sign commits, and verify rules programmatically via [`@modality-org/sdk`](https://www.npmjs.com/package/@modality-org/sdk). |
| **Contract hub (HTTP)** | Multi-party collaboration | Run or connect to a hub server for push/pull workflows — similar to using a hosted database instead of a local file. See the [Contract Hub tutorial](../tutorials/contract-hub). |
| **Rust libraries** | Native services, validators, nodes | Embed `modality-lang`, `modal-common`, and related crates directly in Rust binaries. |
| **WASM / browser** | Client-side verification | Parse and check models in the browser without a round trip to a server. |

**Local-first, network-optional.** A contract starts as files on disk (`state/`, `model/`, `rules/`, commit history) — comparable to SQLite or a git repo. When you're ready, you sync to a hub or the network, the same way you'd point an app at Postgres or a cloud API.

**Typical embedding patterns:**

- **Agent frameworks** — wrap tool calls in Modality rules so agents can only take verified actions (see [For Agents](../for-agents)).
- **Backend services** — validate incoming requests against contract state before executing side effects.
- **Multi-party workflows** — escrow, multisig treasuries, membership-gated contracts (see [Tutorials](../tutorials/contract-hub)).
- **DevOps / CI** — commit and push contract changes as part of a deployment pipeline.

You do not need to rewrite your stack. Pick the surface that fits: CLI for exploration, SDK for application code, hub or network when parties need to share verified state.
