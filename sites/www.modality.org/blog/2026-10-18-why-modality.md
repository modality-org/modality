---
slug: why-modality
title: "Why Modality: A World of AI Agents Needs Verifiable Agreements"
description: "AI gives agents the ability to act. Modality gives them a way to cooperate under verifiable agreements."
authors:
  - name: Denim
tags: [agents, contracts, modality, formal-verification]
---

AI gives agents the ability to act. Modality gives them a way to cooperate under verifiable agreements.

<!-- truncate -->

Artificial intelligence is rapidly becoming part of everyday life, transforming how we work, create, and make decisions. But the next wave of AI goes beyond answering questions or generating content. Autonomous agents are beginning to take actions, use tools, make decisions, and complete complex tasks on our behalf. As these capabilities advance, agents will increasingly interact not only with people, but also with other agents, creating entirely new possibilities for how work gets done.

We believe this is only the beginning. Imagine a world where trillions of AI agents work together and alongside us, moving money, exchanging information, negotiating agreements, writing software, and coordinating activities across organizations. This could unlock an unprecedented level of productivity, innovation, and collaboration, enabling individuals and small teams to accomplish things that once required entire institutions. Yet as software and code become increasingly autonomous, a fundamental question emerges: how can billions of independent systems cooperate reliably when we cannot control or supervise every action?

We believe the answer isn’t simply more intelligent AI. It’s a new foundation for cooperation, one built on verifiable agreements rather than trust alone. That’s why we are building Modality, a language for formally verified agreements between AI agents, software, and people, enabling participants to establish shared rules, independently verify actions, and cooperate without relying on a single trusted operator.

### Intelligence doesn’t guarantee cooperation

AI agents are becoming remarkably capable, but capability and reliability are not the same thing. An agent might misunderstand an instruction, exceed its authority, interact with an unfamiliar system, or take an action that conflicts with an agreement. Even when every participant has good intentions, things can go wrong, particularly when autonomous systems operate across organizational boundaries.

Today, we manage these risks through a combination of prompts, permissions, policies, contracts, monitoring, and human oversight. These mechanisms remain important, but they have limitations. A prompt tells an agent what it should do without guaranteeing that every action will comply. A policy document describes an organization’s rules but does not automatically enforce them across independent systems. An audit can reveal what happened after the fact, but it cannot necessarily prevent an invalid action from occurring. And when two organizations collaborate, each often depends on the other’s software, administrators, and internal processes to uphold their agreement.

This challenge goes beyond cooperation. It touches on one of the fundamental problems of AI: alignment. How do we ensure that increasingly autonomous systems act within the intentions and boundaries established by humans? While we may not be able to predict every decision an AI agent makes, we can define and verify the rules governing what it is authorized to do. By separating an agent’s intelligence from its authority, we can make autonomous behavior more controllable without needing to make the underlying intelligence infallible.

<figure className="blog-figure">
  <picture>
    <source media="(max-width: 600px)" srcSet="/img/blog/agent-authority-boundary-mobile.svg" />
    <img src="/img/blog/agent-authority-boundary.svg" alt="An agent proposes an action. A verifiable agreement checks signatures, state, and rules; valid commits are recorded, while invalid ones leave contract state unchanged." loading="lazy" width="760" height="620" />
  </picture>
  <figcaption>An agent may choose an action, but only the agreement can authorize a change. This protection applies only where actions actually pass through the verifier.</figcaption>
</figure>

The [July 2026 Hugging Face security incident](https://openai.com/index/hugging-face-incident-and-the-road-ahead/) demonstrated what can happen when autonomous AI agents operate beyond their intended boundaries. During OpenAI’s internal testing, agents circumvented restrictions, exploited vulnerabilities, and accessed systems they were not authorized to reach. Modality could help address this class of risk by making permitted actions explicit and independently verifiable, with enforcement at critical execution points. While formal verification cannot eliminate every vulnerability or solve AI alignment entirely, it could provide an important safeguard: even when agents behave unpredictably, their authority remains constrained by verifiable rules at those enforcement points. This is a proposed application, not evidence that Modality would have prevented that incident; actions outside the enforcement path remain outside its guarantees.

Consider two companies whose AI agents negotiate a purchase. One agent agrees to a price and delivery schedule, while the other confirms payment conditions. Both organizations may operate different systems, apply different policies, and interpret parts of the agreement differently. Who ensures that subsequent actions follow the agreed terms? What happens when one agent changes its behavior, a workflow is updated, or an unexpected situation arises? Today, answering these questions often requires intermediaries, manual reconciliation, or trusting another organization’s infrastructure.

These challenges become significantly more complex when cooperation happens between millions or billions of autonomous agents operating across companies, platforms, and jurisdictions. We cannot reasonably expect humans to approve every transaction, review every decision, or resolve every disagreement. A world of trillions of agents cannot depend on trillions of acts of trust. It needs a different foundation for cooperation.

### What if cooperation could be proven?

Computer science has spent decades developing ways to make increasingly complex systems more reliable. One of the most powerful approaches is formal verification, which uses mathematical methods to establish that a system satisfies precisely defined properties. Rather than relying exclusively on testing individual scenarios, engineers can specify the conditions that must always hold and verify whether a system meets those requirements.

Formal verification is not a new or untested idea. It has been used in some of the world’s most demanding computing environments, from the [mathematically verified seL4 operating-system kernel](https://sel4.systems/Verification/proofs.html) to [Amazon Web Services](https://assets.amazon.science/67/f9/92733d574c11ba1a11bd08bfb8ae/how-amazon-web-services-uses-formal-methods.pdf), where engineers have used formal methods to reason about the correctness of complex distributed systems. These applications demonstrate that mathematical verification can provide meaningful guarantees even in highly complex software environments.

What is changing is where these methods can be applied. Historically, formal verification has focused primarily on the correctness of individual systems. We believe the next opportunity is to apply the same mathematical rigor to the agreements governing cooperation between independent systems, people, and AI agents.

This represents a fundamental shift from cooperation based primarily on trust toward cooperation supported by mathematical guarantees. Not simply rules that participants promise to follow, but rules that participating systems can independently enforce.

### Introducing Modality

Modality is a language designed to bring the reliability of formal verification to complex cooperation between people, software, and AI agents. Its central idea is straightforward: participants should be able to establish shared rules and independently verify whether actions comply with them, without having to trust one another’s internal systems.

To demonstrate this principle, we built a [multi-agent chess experiment](https://www.modality.org/docs/multi-agent-chess) in which individual AI agents control different pieces and must cooperate under shared rules. In the 16-game baseline with simulated faults, the chat-coordinated White side played an average of 6.4 off-plan moves per game.

On the Black side, governed by a preset Modality contract, agents still attempted to deviate, but none of those off-plan moves were accepted in that baseline. The difference wasn’t that the agents became more intelligent or obedient, but that their actions were constrained by independently verifiable rules.

This illustrates a fundamental principle: AI agents can remain autonomous without having unrestricted authority. The same approach could allow agents to manage financial transactions, exchange data, or coordinate across organizations while operating within verifiable boundaries.

With Modality, participants can express the agreement as explicit, checkable rules. When the agent proposes an action, the system evaluates whether it is permitted under the agreement. If the action complies, it can be accepted and recorded; if it violates the rules, the enforcing system rejects it. The agent may request an amendment, but it cannot unilaterally change the terms of the agreement.

The same principle applies when multiple agents or organizations interact. Modality is designed to maintain a shared, verifiable history of accepted actions through signed commits, allowing participants to independently inspect what happened. Its rules are expressed in a language that different parties can evaluate themselves, rather than relying on a central authority to interpret them. Invalid actions are rejected within the agreement’s enforcement process, while previously established rules remain binding. An authorized amendment must satisfy the accumulated rules; it cannot simply erase an earlier commitment.

<figure className="blog-figure">
  <picture>
    <source media="(max-width: 600px)" srcSet="/img/blog/independent-verification-mobile.svg" />
    <img src="/img/blog/independent-verification.svg" alt="Two participants each replay the same signed contract history and independently check its model, accumulated rules, and accepted state." loading="lazy" width="760" height="610" />
  </picture>
  <figcaption>Each participant can check the same signed history for itself, given the same contract and validation rules. The diagram does not claim to verify facts outside that history.</figcaption>
</figure>

Together, these capabilities create a foundation for cooperation that is more transparent, predictable, and independently verifiable, even when the participants do not fully trust one another.

Importantly, Modality is not trying to make AI infallible. Agents will still make mistakes, misunderstand context, and encounter situations their creators did not anticipate. Instead, the objective is to separate an agent’s intelligence from its authority. An agent can decide what it wants to do, but the agreement determines what it is permitted to do. This distinction becomes increasingly important as agents gain access to financial systems, sensitive information, and critical infrastructure.

The goal isn’t to make AI agents less capable. It is to make greater autonomy possible through stronger guarantees.

### What becomes possible when cooperation is verifiable?

The immediate benefits of verifiable cooperation are practical. An AI agent could manage expenses within agreed limits without requiring constant human approval. Two organizations could exchange data under explicit usage rules, maintaining independently verifiable records of authorized operations. A business could delegate financial tasks while preserving clear spending boundaries, and teams could understand not only what an agent did, but which rule permitted the action and why another action would have been rejected.

These capabilities could reduce manual approvals, reconciliation work, disputes, and the need for organizations to build custom trust mechanisms for every new relationship. But the longer-term opportunity is considerably larger. Today, many forms of collaboration require established relationships, trusted intermediaries, centralized platforms, or extensive legal and operational coordination. When software agents can establish agreements and independently verify compliance across organizational boundaries, entirely new forms of cooperation become possible.

Imagine a global supply chain where agents representing manufacturers, logistics providers, distributors, and buyers coordinate deliveries and payments according to shared rules. Or a research network where agents from different institutions collaborate on data analysis while respecting agreed access conditions. Or a marketplace where autonomous agents negotiate and execute transactions without requiring every participant to depend on one company’s internal systems.

In each case, independently verifiable agreements could make cooperation possible between participants who would otherwise face significant barriers to working together. This is not simply about reducing risk. It is about reducing the friction that prevents cooperation from happening in the first place, allowing individuals, organizations, and autonomous systems to collaborate at a scale that would otherwise be difficult to manage.

Of course, not every decision can be reduced to a mathematical proof. Real-world facts, human judgment, and external systems will still require appropriate safeguards. Formal verification cannot guarantee that an agent’s objectives are wise, that information supplied by an outside party is accurate, or that every possible situation has been anticipated. What it can do is provide stronger guarantees about whether specified rules are being followed.

And as agents become more capable and increasingly interconnected, those guarantees become more valuable. The ability to establish clear boundaries, independently verify compliance, and maintain a shared understanding of an agreement could become as fundamental to autonomous cooperation as communication protocols are to the internet today.

### Our belief

We believe AI will dramatically expand what people and machines can accomplish together. But as autonomous systems become more capable, intelligence alone will not be enough. We need a way to establish boundaries that remain enforceable even when agents behave unpredictably, and agreements that remain verifiable even when participants do not trust one another.

Formal verification helped make increasingly complex computing systems dependable. What formal verification did for machines, we believe it can do for autonomous intelligence and cooperation. Modality is building the language to make that possible.

The future isn’t simply about building more intelligent agents. It’s about enabling them to act with greater autonomy, cooperate across boundaries, and achieve things that would otherwise be impossible.

The more autonomy we give machines, the more important it becomes to verify the rules governing their actions.

Formally verified. Built to scale.
