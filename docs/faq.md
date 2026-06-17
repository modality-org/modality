---
sidebar_position: 7
title: FAQ
---

# Frequently Asked Questions

## What is Modality?

Modality is an open source standard for verifiable contracts.

## What are verifiable contracts?

Verifiable contracts are a mechanism for ensuring the compliance of evolving constraints over data.

Unlike smart contracts — which are implemented as computer programs and can't truly be formally verified — verifiable contracts use formal specifications that are confirmed over all possible outcomes.

## Why do AI agents need contracts?

As AI agents increasingly cooperate, delegate tasks, and exchange value, they need a way to make enforceable commitments — not just promises. An agent delegating work to another agent needs guarantees: that funds won't disappear, that deadlines will be honored, that authority won't be misused.

Modality gives agents a shared language for expressing and verifying these commitments mathematically.

## How are verifiable contracts different from smart contracts?

Both smart contracts and verifiable contracts keep an append-only log of interactions. Both serve to restrict the nature of those interactions.

But only verifiable contracts provide native formal verification, ensuring that they work exactly as specified.

In contrast, smart contracts are implemented as computer programs and are not able to be formally verified. Any attempt at formal verification of smart contracts is critically limited because of this. Even the most expensive audits of smart contracts are known to miss critical bugs.

|                                       | Smart Contracts     | Verifiable Contracts       |
| :------------------------------------ | :-----------------: | :------------------------: |
| Does it need a blockchain?            | ✅                   | ❌                         |
| Does it keep an append-only log?      | ✅                   | ✅                         |
| Does it restrict interactions?        | ✅                   | ✅                         |
| Does it ensure correctness?           | ❌                   | ✅                         |
| Is it formally specified?             | ❌                   | ✅                         |

## How do verifiable contracts work?

Verifiable contracts are an append-only log of interactions called commits.

Each commit may contain values and rules. Values are recorded in the log. Rules are formal specifications that constrain future commits.

Whenever a new rule is added, a governing model is provided, proving that all rules remain satisfied.

Whenever a commit is added, it must match a valid transition in the governing model. The model's transition predicates are the commit-time enforcement mechanism; rules constrain which models are acceptable witnesses.

## What do rules look like?

Modality rules constrain who can commit based on signatures and state. They use predicates like:

```modality
// All commits must be signed by alice or bob
always(<+signed_by(/users/alice.id)> true | <+signed_by(/users/bob.id)> true)

// Membership changes require every member signature
always([+modifies(/members)] true -> <+all_signed(/members)> true)

// Time-gated: only after deadline
always(<+after(/deadlines/expiry.datetime)> true -> <+signed_by(/users/buyer.id)> true)
```

For fixed quorum examples, use explicit signer combinations in formulas. Governing models can use predicate guards such as `+threshold(2, /treasury/signers.json)` on transitions.

## Do I need a blockchain to use Modality?

No. Modality works without a blockchain. You can use a Hub — a lightweight server for contract collaboration — or even work entirely locally.

The Modality Network is an optional blockchain for applications that benefit from public, decentralized posting of verifiable contracts.

## How do I get started?

Check out the [Quickstart](/docs/quickstart) to create your first verifiable contract, or browse the [tutorials](/docs/category/tutorials) for step-by-step walkthroughs.

## Is Modality open source?

Yes. Modality is fully open source. You can find the code on [GitHub](https://github.com/modality-org/modality).

## Who started this project?

Modality was initially conceptualized by [Bud Mishra](https://scholar.google.com/citations?user=kXVBr20AAAAJ&hl=en&oi=ao) and [Foy Savas](https://foysavas.com).

Notably, [Bud was the first person to use formal verification to identify a hardware bug](https://discuss.modality.org/t/the-birth-of-model-checking/14/2). When formal verification for hardware was being initially developed, almost everyone considered it impossible or impractical. Today, formal verification is a standard part of the hardware development process.

Watch Foy's presentation on Modality: [**Verifiable Contracts for AI Agent Cooperation**](https://www.youtube.com/watch?v=poOqWdh10BQ)
