# 🤔 Modality FAQs

## What is Modality?

Modality is an open source standard for verifiable contracts.

## What are verifiable contracts?

Verifiable contracts are a mechanism for ensuring the compliance of evolving constraints over data.

## How are verifiable contracts different from smart contracts?

Both smart contracts and verifiable contracts keep an append-only log of interactions. Both serve to restrict the nature of those interactions.

But only verifiable contracts provide native formal verification, ensuring that they work exactly as specified.

In contrast, smart contracts are implemented as computer programs and are not able to be formally verified. Any attempt at formal verification of smart contracts is critical limited because of this. Even the most expensive audits of smart contracts are known to miss critical bugs.

|                                       | Smart Contracts     | Verifiable Contracts       |
| :------------------------------------ | :-----------------: | :------------------------: |
| Does it need a blockchain?            | ✅                   | ❌                         |
| Does it keep an append-only log?      | ✅                   | ✅                         |
| Does it restrict interactions?        | ✅                   | ✅                         |
| Does it ensure correctness?           | ❌                    | ✅                         |
| Is it formally specified?             | ❌                    | ✅                         |

## Who started this project?

Modality was initially conceptualized by [Bud Mishra](https://scholar.google.com/citations?user=kXVBr20AAAAJ&hl=en) and [Foy Savas](https://foysavas.com).

Notably, [Bud was the first person to use formal verification to identify a hardware bug](https://discuss.modality.org/t/the-birth-of-model-checking/14/2). When formal verification for hardware was being initially developed, almost everyone considered it impossible or impractical. Today, formal verification is a standard part of the hardware development process.

## What is formal verification?

Formal verification is a technique for verifying the behavior of a system using formal specifications. When a specification is formally verified, it is confirmed over all possible outcomes.

## How do verifiable contracts work?

Verifiable contracts are an append-only log of interactions called commits.

Each commit may contain values and rules. Values are recorded in the log. Rules are formal specifications that constrain future commits.

Whenever a new rule is added, a governing model is provided, proving that all rules remain satisfied.

Whenever a commit is added, its validity against the rules is confirmed using the governing model.

Sometimes the existing governing model over-constrains a contract beyond the specified rules. In such cases, the governing model may be replaced by a new commit.

<!-- ## Who does the verifying?

... -->

## What do rules look like?

Modality rules are written in temporal logic called MTL (Modality Temporal Logic).
MTL is based on the [modal-mu calculus](https://en.wikipedia.org/wiki/Modal_%CE%BC-calculus) but made to be more accessible.

Here's a simple example that requires all future commits to be signed by an admin:

`always must include_signature_from( /config/admin.id )`

## What's the Modality Network?

We're working on a specialized blockchain for verifiable contracts called the [Modality Network](./network.md).

You don't have to use the Modality Network to use Modality itself. However, if your application would benefit from the public posting of verifiable contracts in a decentralized environment, the Modality Network is a great fit.