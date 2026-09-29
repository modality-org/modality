---
sidebar_position: 7
title: FAQ
---

# Frequently Asked Questions

## What is Modality?

Modality is a verification language for agential cooperation. A contract is
state, a model of possible moves, and accumulating rules. Every accepted
commit is signed. Anyone can replay the log.

Cooperation at the scale of many agents requires shared rules built on
formally verified agreements, in place of trust. Formal verification already
made computers, clouds, and medical devices reliable. This language points
that reliability at agreements.

The language, verifier, CLI (`modal`), hub, and SDK are open source.

## What are verifiable contracts?

A verifiable contract is an append-only log of signed commits. Rules are
formal specifications. The next commit is accepted only if it is a valid
model transition and satisfies every rule already on the log.

That is different from a prompt an agent can ignore, and different from a
program that later monitoring might flag.

## Why do agents need contracts?

Agents call tools, spawn other agents, and forget. They deal with strangers.
Reputation and system prompts do not survive that.

A counterparty needs a log they can replay: who may do what, when, and with
what evidence — including after the process that promised it is gone.

## How is this different from a smart contract?

Smart contracts are programs, usually on a chain. Audits look for bugs in
those programs.

A Modality contract is specified as a model and formulas. The verifier
rejects a commit that has no valid witness. You can start locally, or share
through a [Contract Hub](/docs/tutorials/contract-hub). You do not need a
chain to check an agreement.

## How do verifiable contracts work?

Each commit may post state, change the witness model, or add a rule.

When a rule is added, a governing model must still satisfy **all** accumulated
rules. When a commit is added, it must match a valid transition in that model.
Predicates on the transition (signatures, evidence, state) are checked at
commit time. Time predicates such as `after` are not evaluated yet, so they
never hold.

## What do rules look like?

Modality rules constrain who can commit, based on signatures and posted state:

```modality
// Every commit after this one is signed by alice or bob
always([-signed_by(/users/alice.id) -signed_by(/users/bob.id)] false)

// Membership changes require every member signature
always([+modifies(/members) -all_signed(/members)] false)

// Writes under /treasury need two of the keys under /treasury
always([+modifies(/treasury) -threshold("2", /treasury)] false)
```

Each rule forbids the commits that lack the evidence. `threshold("2",
/treasury)` counts the keys in the `*.id` files under `/treasury`.

See the [formula cookbook](/docs/language/formula-cookbook).

## Do I need a network to use Modality?

No. A contract is files on disk. Start locally, or use a
[Contract Hub](/docs/tutorials/contract-hub) when several parties need to
push and pull.

## How do I get started?

Install `modal`, then create [your first contract](/docs/getting-started/first-contract).
Tutorials cover escrow, membership, and hub workflows.

## Is Modality open source?

Yes. The language, verifier, CLI, hub, SDK, and node are open source:
[github.com/modality-org/modality](https://github.com/modality-org/modality).

## Who started this project?

Modality was initially conceptualized by
[Bud Mishra](https://scholar.google.com/citations?user=kXVBr20AAAAJ&hl=en&oi=ao)
and [Foy Savas](https://foysavas.com).

[Bud was among the first to use formal verification to identify a hardware
bug](https://discuss.modality.org/t/the-birth-of-model-checking/14/2). When
formal verification for hardware was being developed, almost everyone
considered it impossible or impractical. Today it is a standard part of
hardware development.

Watch Foy's presentation:
[**Verifiable Contracts for AI Agent Cooperation**](https://www.youtube.com/watch?v=poOqWdh10BQ)
