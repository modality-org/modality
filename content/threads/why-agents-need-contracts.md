# Thread: Why Agents Need Contracts

**Target:** AI/agent devs, Web3 devs, agent builders  
**Tone:** Thoughtful, slightly provocative, technical but accessible  
**Hook:** The trust problem between agents

---

## Thread

**1/**
AI agents are making deals with each other right now.

Booking flights. Trading data. Coordinating swarms.

But here's the problem: How do you trust an agent you've never met?

🧵

**2/**
When humans make deals, we have:
- Reputations that took years to build
- Courts that enforce contracts
- Social pressure from mutual connections

Agents have none of this.

They spin up in seconds. They're pseudonymous. They disappear when their task is done.

**3/**
"Just use an API" doesn't solve it.

APIs are promises, not proofs.

When Agent A calls Agent B's endpoint, it's trusting that B will do what it says. No verification. No recourse.

That's fine for fetching weather. It's not fine for a $10k escrow.

**4/**
Smart contracts help, but they're designed for humans.

- Slow (block times)
- Expensive (gas fees)
- Rigid (deploy once, pray it's right)

Agents need something faster, cheaper, and more flexible — but equally trustless.

**5/**
Enter formal verification.

What if agents could prove their commitments mathematically?

Not "trust me" — but "here's a proof you can verify yourself."

No reputation needed. No courts. Just math.

**6/**
This is what we're building with Modality.

A language where agents express commitments as verifiable state machines.

"I will release funds when you deliver" isn't a promise — it's a formula that can be checked.

**7/**
Here's what a simple escrow looks like:

```
model escrow {
  initial pending
  pending -> delivered [+signed_by(/seller)]
  delivered -> released [+signed_by(/buyer)]
  pending -> refunded [+signed_by(/arbiter)]
}
```

Both agents can verify this before committing a single dollar.

**8/**
The key insight: Rules are permanent. Models can change.

Rules define what MUST be true:
```
always (+modifies(/funds) implies +signed_by(/owner))
```

Once a rule exists, no one can bypass it. Not the other agent. Not even you.

**9/**
This isn't just theory.

We're building:
- Modal contracts with append-only logs
- Predicate system for signature verification
- Hub infrastructure for multi-party coordination

All open source: github.com/modality-org/modality

**10/**
The future isn't agents trusting each other.

It's agents proving their commitments and verifying others' proofs.

Trust from math, not faith.

If you're building agents that need to cooperate, check out Modality. We'd love your feedback.

modality.org

---

## Optional Additions

**Image for tweet 7:** Syntax-highlighted code snippet of escrow model

**Alt ending (more CTA focused):**
> We're looking for early builders to try Modality and break things.
> 
> DM me or join our Discord: [link]

---

## Notes

- Can split into shorter thread (tweets 1-5) for "problem" and follow up with "solution"
- Tweet 7 code block may need image for better rendering on X
- Consider adding a diagram for the escrow state machine

---

*Draft v1 — 2026-02-07*
