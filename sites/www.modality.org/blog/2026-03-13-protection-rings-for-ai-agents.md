---
slug: protection-rings-for-ai-agents
title: "Protection Rings for AI Agents: How to Let Agents Code Without Letting Them Nuke Your Database"
description: OS-style security boundaries for AI coding agents, enforced by verifiable contracts. Give your agents freedom to ship features while mathematically preventing them from touching critical infrastructure.
authors: [gerold]
tags: [tutorial, security, agents, formal-verification, developer-tools]
keywords: [AI agent security, protection rings, agent boundaries, Modality, verifiable contracts, AI coding agent, Cursor security, Claude Code, Devin, agent guardrails, formal verification]
---

Your AI coding agent has the same access as your senior engineer. Should it?

Right now, if you're using Cursor, Claude Code, Devin, or any AI coding agent — it can read and write every file in your repo. Auth logic? Database schemas? Deploy scripts? Secrets management? All fair game.

That's fine when humans write code. We know not to "optimize" authentication by skipping password verification. But agents moving at machine speed don't have that intuition. And a markdown file saying "please don't touch auth.js" is not a security boundary. It's a suggestion.

**What happens when an agent helpfully refactors your authentication to remove "unnecessary" password checks?**

<!-- truncate -->

## The Problem Is Structural

Most codebases are flat. Everything lives in one repo with one level of access. Feature code sits next to database schemas. UI components share a directory with deployment scripts. There's no enforced separation between "things an agent should freely change" and "things that could bring down the entire system."

Current approaches to agent safety are all soft boundaries:

- **Prompt instructions**: "Don't modify files in /auth." An agent can ignore this, hallucinate past it, or simply not understand it.
- **Code review**: Works for humans reviewing human PRs. Doesn't scale when agents are making 100 commits a day.
- **Linter rules**: Can catch patterns, but are trivially bypassable and don't understand intent.
- **RLHF/guardrails**: Suggestions to an LLM. One jailbreak bypasses them all.

None of these are *enforcement*. They're all just increasingly sophisticated ways of saying "please don't."

## Operating Systems Solved This 50 Years Ago

In the 1970s, OS designers faced the same problem: how do you let user programs run freely without letting them crash the kernel?

The answer was **protection rings** — hardware-enforced boundaries between privilege levels:

```
┌──────────────────────────────────────────────┐
│  Ring 3: User Applications                   │
│  ┌──────────────────────────────────────┐    │
│  │  Ring 0: Kernel                      │    │
│  │  • Memory management                 │    │
│  │  • Hardware drivers                  │    │
│  │  • Process scheduling                │    │
│  │  • Security enforcement              │    │
│  └──────────────────────────────────────┘    │
│  • Web browsers                              │
│  • Text editors                              │
│  • Games                                     │
│  • Everything else                           │
└──────────────────────────────────────────────┘
```

A user application *literally cannot* access kernel memory. Not "shouldn't" — *cannot*. The CPU enforces it. No amount of clever code in userspace can bypass a hardware protection ring.

**We need the same thing for AI agents working on codebases.**

## Protection Rings for Agent Development

Here's how it maps:

**Ring 0 (Kernel)** — Critical infrastructure that could break everything:
- Database schemas
- Authentication and authorization logic
- Application configuration and secrets
- Deployment scripts and CI/CD
- Security policies

**Ring 3 (Userspace)** — Feature code that agents should ship freely:
- API routes and endpoints
- UI components
- Tests
- Documentation
- Business logic

The kernel agent moves slowly, with human oversight. The userspace agent moves fast, with zero friction. The boundary between them isn't a code review — it's a **verifiable contract**.

## What a Verifiable Contract Looks Like

In Modality, the protection ring boundary is encoded as formal rules with cryptographic enforcement:

```modality
rule userspace_boundary {
  formula {
    always(
      +signed_by(/agents/userspace.id) implies -modifies(/kernel)
    )
  }
}
```

Translation: If the userspace agent signed this commit, it **cannot** modify any kernel path. Not "shouldn't." Cannot. The contract engine rejects the commit before it's ever applied.

```modality
rule kernel_requires_dual_signature {
  formula {
    always(
      +modifies(/kernel) implies (
        +signed_by(/agents/kernel.id) & +signed_by(/humans/admin.id)
      )
    )
  }
}
```

Translation: Any commit that touches kernel code must be signed by **both** the kernel agent AND a human. No agent — not even the kernel agent — can unilaterally modify critical infrastructure.

These rules are:
- **Permanent** — once added, they can never be removed
- **Mathematical** — checked by a formal verification engine, not an LLM
- **Cryptographic** — every action is signed with ed25519 keys
- **Auditable** — full commit log, independently verifiable by anyone

## What This Looks Like in Practice

### Scenario 1: Userspace agent ships a feature ✅

The userspace agent adds a new API endpoint and UI component. It only touches Ring 3 paths.

```
Userspace Agent → ADD_FEATURE
  paths: /userspace/routes.js, /userspace/components.jsx
  ✓ userspace_boundary: paths are Ring 3
  ✓ known_signers: agent recognized
  COMMITTED #0
```

No friction. No approval needed. Ship it.

### Scenario 2: Userspace agent tries to "fix" auth ✗

The agent decides the auth logic is inefficient and tries to modify it.

```
Userspace Agent → MODIFY_AUTH
  paths: /kernel/auth.js
  ✗ userspace_boundary: Userspace Agent cannot modify kernel paths
  REJECTED
```

Not a warning. Not a suggestion. The commit is rejected. The auth logic is untouched.

### Scenario 3: Sneaky mixed commit ✗

The agent tries to slip a kernel change into a feature commit — one config change hidden among UI updates.

```
Userspace Agent → ADD_FEATURE
  paths: /userspace/routes.js, /kernel/config.js, /userspace/components.jsx
  ✗ userspace_boundary: Userspace Agent cannot modify kernel paths
  REJECTED
```

Even one kernel path in a mixed commit triggers the boundary. You can't sneak past math.

### Scenario 4: Kernel agent acts alone ✗

The kernel agent tries to update the database schema without human approval.

```
Kernel Agent → ADD_TABLE
  paths: /kernel/schema.sql
  ✗ kernel_requires_dual: needs kernel agent + human signature
  REJECTED
```

No agent acts alone on Ring 0. Not even the one designated to work there.

### Scenario 5: Proper dual-signed kernel change ✅

The kernel agent proposes a schema change. A human reviews it, agrees, and co-signs.

```
Kernel Agent + Human Admin → ADD_TABLE
  paths: /kernel/schema.sql
  ✓ userspace_boundary: signers are authorized
  ✓ kernel_requires_dual: dual signature verified
  COMMITTED #1
```

Both signatures present. Both verified cryptographically. Change accepted.

## Cross-Ring Cooperation

The system doesn't just say "no." It provides a formal path for when the userspace agent *needs* something from Ring 0.

Say the userspace agent is building a feature that requires a new database table. It can't modify the schema. But it can request the change through a formal cooperation protocol:

1. **Userspace agent** commits a change request (to a Ring 3 path — it's just a document)
2. **Kernel agent** reviews and implements the schema change
3. **Human admin** co-signs the kernel commit
4. **Userspace agent** builds the feature against the new schema

Every step is a signed commit. Every transition is verified. The audit trail shows exactly who did what and when.

## Why This Matters Now

Three trends are converging:

**1. Agents are writing more code than ever.** GitHub Copilot, Cursor, Claude Code, Devin — AI agents are becoming the primary authors of code. The volume of agent-written code is growing exponentially.

**2. Agents are getting deeper access.** It's not just autocomplete anymore. Agents have terminal access, can run commands, modify files, push to git, and deploy to production. The blast radius of a mistake is enormous.

**3. Current guardrails don't scale.** You can't code-review 100 agent commits a day. You can't write prompt instructions for every edge case. You need enforcement that works at the speed agents operate — and that means mathematical verification, not human oversight on every action.

## Try It

We built a working demo with real ed25519 signatures. Clone the repo and run it:

```bash
git clone https://github.com/modality-org/modality.git
cd modality/tutorials/protection-rings
npm install
npm run demo
```

You'll see all six scenarios play out — accepted commits, rejected violations, and the full audit trail. Every signature is real. Every rule check is mathematical.

There's also an [interactive web demo](https://github.com/modality-org/modality/tree/main/tutorials/protection-rings) you can open in your browser — no install required.

## What's Next

Protection rings are the simplest application of Modality's verifiable contracts. The same primitives — signed commits, formal rules, cryptographic predicates — extend to:

- **Agent-to-agent commerce**: enforcing terms when one agent hires another
- **Healthcare compliance**: HIPAA-grade access control for medical record agents
- **Financial authorization**: multisig thresholds for treasury management
- **Cross-organization cooperation**: contracts between agents from different companies

The foundation is the same: **rules that are mathematical, not suggestions. Boundaries that are cryptographic, not cultural. Audit trails that are provable, not aspirational.**

If AI is going to write 100x the code, we need 100x the guardrails. But we can't hand-write them. We need guardrails that verify themselves.

That's what Modality does.

---

*[Modality](https://modality.org) is an open source standard for verifiable contracts. [GitHub](https://github.com/modality-org/modality) · [Docs](https://modality.org/docs) · [@modalitylang](https://x.com/modalitylang)*
