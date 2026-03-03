# Protection Rings for Agent Development

**OS-style security boundaries for AI agents, enforced by Modality contracts.**

## The Problem

You want AI agents building your app. But your codebase is a monorepo where auth logic, database schemas, deployment scripts, and feature code all live side by side. Every file has the same level of access.

That's fine when humans are writing code — we know not to touch `auth.js` when we're building a feature. But AI agents moving at machine speed don't have that intuition. And guardrails in markdown files are just suggestions.

**What happens when an agent "helpfully" optimizes your authentication by skipping password verification?**

## The Solution: Protection Rings

Operating systems solved this 50 years ago with **protection rings**:

```
┌─────────────────────────────────────────────┐
│  Ring 3: Userspace                          │
│  ┌─────────────────────────────────────┐    │
│  │  Ring 0: Kernel                     │    │
│  │  • Database schemas                 │    │
│  │  • Authentication                   │    │
│  │  • Configuration                    │    │
│  │  • Deploy scripts                   │    │
│  │  • Secrets management               │    │
│  └─────────────────────────────────────┘    │
│  • API routes                               │
│  • UI components                            │
│  • Tests                                    │
│  • Documentation                            │
│  • Feature logic                            │
└─────────────────────────────────────────────┘
```

In this tutorial, we apply the same concept to agent development:

- **Ring 0 (Kernel)**: Critical infrastructure. Only the Kernel Agent can modify, and only with human approval.
- **Ring 3 (Userspace)**: Feature code. The Userspace Agent works freely — no bottleneck.

The boundaries are enforced by **Modality verifiable contracts** — not linter rules, not code review, not markdown guidelines. Mathematical constraints that reject invalid commits before they happen.

## Project Structure

```
sample-app/
├── kernel/              ← Ring 0 (protected)
│   ├── schema.sql       ← Database schema
│   ├── auth.js          ← Authentication
│   └── config.js        ← App configuration
├── userspace/           ← Ring 3 (free)
│   ├── routes.js        ← API routes
│   └── components.jsx   ← UI components
└── shared/              ← Shared types/utils

contracts/
├── protection-rings.modality    ← Ring boundary contract
└── cooperation.modality         ← Cross-ring change requests
```

## The Contract

The core contract (`protection-rings.modality`) encodes three rules:

### Rule 1: Userspace Boundary
```modality
rule userspace_boundary {
  formula {
    always(
      +signed_by(/agents/userspace.id) implies -modifies(/kernel)
    )
  }
}
```
Translation: If the userspace agent signed it, it CANNOT modify any kernel path. Ever.

### Rule 2: Kernel Dual Signature
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
Translation: Any commit that touches kernel paths must be signed by BOTH the kernel agent AND a human. No unilateral kernel changes.

### Rule 3: Known Signers
```modality
rule known_signers_only {
  formula {
    always(
      +signed_by(/agents/userspace.id)
      | +signed_by(/agents/kernel.id)
      | +signed_by(/humans/admin.id)
    )
  }
}
```
Translation: Every commit must come from a known identity. No anonymous modifications.

## Run the Demo

```bash
npm install
npm run demo
```

The demo walks through six scenarios:

1. **Userspace ships a feature** → ✓ Accepted (Ring 3 paths only)
2. **Userspace tries to modify auth** → ✗ Rejected (touches Ring 0)
3. **Sneaky mixed commit** → ✗ Rejected (kernel path hidden among userspace paths)
4. **Kernel agent acts alone** → ✗ Rejected (no human co-signature)
5. **Proper dual-signed kernel change** → ✓ Accepted (kernel + human)
6. **Cross-ring cooperation** → ✓ Userspace requests → Kernel implements → Human approves → Userspace builds

Every action uses real ed25519 signatures. Every rule is checked mathematically. Every commit is independently verifiable.

## Cross-Ring Cooperation

When the userspace agent needs something from Ring 0 (a new table, a config change, a new auth endpoint), it can't just make the change. Instead:

```
Userspace Agent                    Kernel Agent              Human Admin
      │                                 │                        │
      ├── PROPOSE change request ──────►│                        │
      │   (signed commit to             │                        │
      │    /userspace/requests/)        │                        │
      │                                 ├── REVIEW & ACCEPT ────►│
      │                                 │   (signed commit)      │
      │                                 │                        ├── APPROVE
      │                                 │◄── (co-signed) ────────┤
      │                                 │                        │
      │                                 ├── EXECUTE              │
      │                                 │   (dual-signed commit  │
      │                                 │    to /kernel/)        │
      │◄── builds feature ─────────────│                        │
      │   (signed commit to            │                        │
      │    /userspace/)                │                        │
```

This flow is encoded in `cooperation.modality`. The contract ensures:
- Only userspace can propose
- Only kernel can execute
- Execution requires prior human approval
- Every step is signed and auditable

## Key Takeaways

1. **Agents need boundaries, not just prompts.** A markdown file saying "don't touch auth.js" is a suggestion. A Modality contract is a mathematical proof.

2. **Protection rings let fast agents stay fast.** The userspace agent ships features at machine speed with zero friction. The bottleneck only exists where it should — at the kernel boundary.

3. **Cooperation, not just restriction.** The system doesn't just say "no" — it provides a formal path for cross-ring changes. Request → Review → Approve → Execute.

4. **Everything is auditable.** Six months from now, when a regulator asks "who changed the auth logic and why?" — the commit log has every signature, every approval, every step.

## Next Steps

- **Try it on your codebase**: Map your files to Ring 0 and Ring 3. What's kernel? What's userspace?
- **Add more rings**: Ring 1 for infrastructure (CI/CD, monitoring), Ring 2 for business logic.
- **Deploy with a Hub**: Push your contract to a Modality Hub and have agents commit against it in real-time.

---

*Built with [Modality](https://modality.org) — verifiable contracts for the agentic economy.*
