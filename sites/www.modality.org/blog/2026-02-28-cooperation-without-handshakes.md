---
slug: cooperation-without-handshakes
title: "Cooperation Without Handshakes: What Google's New Paper Means for AI Agents"
description: "Google showed self-interested agents can learn to cooperate. But emergent cooperation isn't guaranteed cooperation — and that's where verification comes in."
authors: [gerold]
tags: [research, cooperation, modal-logic, game-theory]
keywords: [AI cooperation, multi-agent, prisoner's dilemma, modal logic, Löb's theorem, formal verification, MIRI, Google AI, Modality]
---

*How self-interested agents learn to cooperate — and why it's not enough.*

<!-- truncate -->

## The Paper

[Multi-agent cooperation through in-context co-player inference](https://arxiv.org/abs/2602.16301) (Weis, Wołczyk, Nasser, Saurous, Agüera y Arcas, Sacramento, Meulemans — Google Paradigms of Intelligence Team, February 2026)

A team at Google just demonstrated something remarkable: self-interested AI agents can learn to cooperate with each other *without being told to*, *without knowing anything about each other's internals*, and *without any central coordinator*.

The catch? It only works under very specific conditions. And the cooperation it produces has no guarantees.

## What They Found

The researchers studied the classic Prisoner's Dilemma — the foundational game theory scenario where two players each choose to cooperate or defect. Defecting is always individually rational, but mutual cooperation produces better outcomes for both.

Getting AI agents to reliably cooperate here has been an open problem for decades. Prior approaches required either:
- **Explicit modeling** of the other agent's learning algorithm (which requires knowing how they work)
- **Rigid role separation** between "naive learners" and "meta-learners" (which requires centralized design)

Both are impractical in the real world, where agents are built by different teams, run different architectures, and don't share their internals.

### The Breakthrough: Diversity Breeds Cooperation

Google's key insight is elegant: **train agents against a diverse mix of opponents, and cooperation emerges naturally.**

Here's the mechanism, broken into four steps:

**Step 1: Diversity forces adaptation.** When an agent faces many different types of opponents (some cooperative, some aggressive, some random), it develops the ability to *infer* what kind of opponent it's facing from interaction history alone. Within a single episode, it learns to identify and best-respond to its co-player. This is in-context learning — the same capability that makes foundation models powerful.

**Step 2: Adaptability creates vulnerability.** An agent that adapts to its opponent can be *exploited*. If you know the other agent will adjust its strategy based on your moves, you can shape its behavior — play aggressively early to push it toward cooperation on your terms. This is called extortion, and it's a well-studied phenomenon in game theory (Press & Dyson, 2012).

**Step 3: Mutual extortion resolves into cooperation.** When *two* agents with extortion capabilities face each other, something interesting happens. Both try to shape the other's behavior. The resulting dynamic — each agent pushing the other toward a strategy that benefits itself — converges on mutual cooperation. Neither agent intended to cooperate. They both tried to exploit. The equilibrium they landed on just happens to be cooperative.

**Step 4: The full picture.** In a mixed population, agents maintain their adaptive capabilities (needed for diverse opponents), which keeps them vulnerable to extortion by other learning agents, which drives them toward cooperation. Remove the diversity — train agents only against each other — and they collapse to mutual defection.

### The Results

Both training methods tested (standard actor-critic RL and the authors' novel Predictive Policy Improvement algorithm) converged to robust cooperation. Critically, two ablations confirmed the mechanism:
- **Give agents explicit opponent identifiers** → cooperation collapses (no need for in-context inference → no adaptability → no extortion pathway)
- **Remove opponent diversity** → cooperation collapses (no diverse opponents → no in-context learning incentive)

## Why This Matters

This paper represents a genuine advance. It shows that the standard training paradigm for foundation models — learn from diverse data, develop in-context capabilities — naturally produces cooperative behavior in multi-agent settings. No special architectures. No centralized design. No assumptions about opponent internals.

The implications for the real world are significant. As foundation model agents are deployed at scale — negotiating, transacting, coordinating — this suggests they may develop cooperative tendencies organically, simply from the diversity of agents they encounter.

## Why It's Not Enough

But here's what the paper doesn't say, and what matters most for anyone deploying agents in production:

### 1. Emergent cooperation is not guaranteed cooperation

The agents in this paper learned to cooperate in the Iterated Prisoner's Dilemma — a 2-player, 2-action, fully symmetric game with 100 rounds. The real world has millions of agents, continuous action spaces, asymmetric information, one-shot interactions, and stakes measured in dollars, not reward points.

The paper's own ablations show how fragile the mechanism is. Change the training distribution? Cooperation breaks. Give agents too much information? Cooperation breaks. The cooperative equilibrium exists, but there's no proof it will emerge in any particular real-world setting.

**Cooperation that *might* emerge is not cooperation you can rely on.**

### 2. Cooperation is not compliance

Even when agents cooperate, there's no guarantee they cooperate *on your terms*. The paper shows agents converging to mutual cooperation, but the specific form of cooperation — who concedes what, when, under what conditions — is determined by the training dynamics, not by any explicit agreement.

In the real world, you don't just need agents to cooperate. You need them to cooperate *according to specific rules*. The escrow releases when the goods are delivered. The payment happens only after both parties sign. The API call stays within the authorized scope.

Emergent cooperation can't enforce terms. It can only discover equilibria.

### 3. There's no audit trail

The cooperative behavior in this paper lives entirely inside the agents' weights and in-context dynamics. There is no record of what was agreed to. No way to verify that cooperation occurred for the right reasons. No mechanism for a third party to audit the interaction after the fact.

When something goes wrong — and in a world of millions of agent interactions, things *will* go wrong — "the agents' training dynamics converged to a cooperative equilibrium" is not an answer that satisfies regulators, counterparties, or courts.

## The Missing Piece: Modal Logic Was Already the Answer

Remarkably, the theoretical foundation for robust one-shot agent cooperation was laid over a decade ago — using exactly the kind of formal logic that Modality is built on.

In 2014, Critch, LaVictoire, and colleagues from MIRI (the Machine Intelligence Research Institute, co-founded by Eliezer Yudkowsky) published ["Robust Cooperation in the Prisoner's Dilemma: Program Equilibrium via Provability Logic"](https://arxiv.org/abs/1401.5577). Their approach was radically different from Google's: instead of hoping cooperation emerges from training dynamics, they *proved* it could be achieved through modal logic constraints.

The setup: two agents in a one-shot Prisoner's Dilemma, each with read-access to the other's source code. Using the modal logic of provability (Löb's theorem), they constructed "modal agents" that achieve mutual cooperation with two critical properties:

- **Robust:** Cooperation doesn't require the agents to be identical — they just need to be *provably* cooperative
- **Unexploitable:** A modal agent *never* cooperates when its opponent defects — this is mathematically guaranteed, not statistically likely

This is the key distinction. Google's 2026 paper shows cooperation emerging from repeated interaction, diverse training, and in-context inference. The MIRI paper showed cooperation achievable in a *single shot* — no history, no repeated games, no training dynamics — through formal verification of the other agent's commitments.

The 2014 paper had a limitation the authors themselves acknowledged: it required agents to have read-access to each other's source code, and the bounded (computationally tractable) analogues remained unproven. But the core insight stands: **modal logic is the natural language for expressing verifiable cooperative commitments between agents.**

That was 2014. Today, we have AI agents that can read and write modal logic natively. The computational barrier that kept this theoretical result impractical for a decade has been removed.

## The Modality Perspective

This paper validates a core premise of [Modality](https://github.com/modality-org/modality): agents *will* need to cooperate, and the AI community is actively working on making that happen. The question isn't whether agents will cooperate — it's whether that cooperation will be *verifiable*.

Modality approaches the same problem from the opposite direction:

| | Google (2026) | MIRI Modal Agents (2014) | Modality |
|---|---|---|---|
| **Cooperation source** | Emergent from training | Proven via provability logic | Explicit signed contracts |
| **Guarantees** | Statistical | Mathematical (theoretical) | Mathematical (practical) |
| **Enforcement** | None (equilibrium) | Source code inspection | Verification engine |
| **One-shot capable** | No (needs repeated interaction) | Yes | Yes |
| **Auditability** | None | Proof-based | Append-only commit log |
| **Practical today** | Yes (but fragile) | No (requires source access) | Yes |

These approaches are not competitors — they're complementary layers in a stack:

- **Layer 1 (Google, 2026):** Agents develop cooperative *dispositions* through training. They're inclined to cooperate. This is valuable — it means agents aren't starting from pure self-interest in every interaction.

- **Layer 2 (MIRI, 2014 — the theoretical foundation):** Modal logic can express provably robust, unexploitable cooperation. This was the insight that showed the *right formalism* existed. It just lacked a practical implementation.

- **Layer 3 (Modality):** Agents formalize their cooperation into verifiable contracts using temporal modal logic — making the MIRI insight practical at scale. Dispositions become commitments. Inclinations become guarantees. Every action is signed, every rule is checked mathematically, every interaction is auditable.

The cooperative dispositions this paper describes make Modality contracts *easier* to form — agents predisposed to cooperation will more readily agree to formal terms. But the contracts make the cooperation *trustworthy* — not because we believe the agents' training dynamics will hold, but because the math proves it.

Think of it this way: humans evolved cooperative instincts too. We're naturally inclined toward fairness, reciprocity, and reputation management. But we still write contracts. Not because our instincts are worthless — but because instincts alone aren't sufficient when the stakes are high enough.

## The Bottom Line

Google has shown that cooperation can emerge from self-interested agents through a beautifully simple mechanism: diversity forces adaptation, adaptation enables shaping, and mutual shaping converges on cooperation.

It's an important result. And it's exactly why verification matters more than ever.

Because the question was never "will agents cooperate?" The question is: **"can you prove it?"**
