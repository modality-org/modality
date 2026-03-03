# Protection Rings for Agent Development

**Two repos. Two agents. The app agent can't even READ kernel code.**

## The Problem

You want AI agents building your app. But your codebase has auth logic, database schemas, secrets, and deployment scripts living alongside feature code.

The usual fix: "just add a rule in the agent's instructions saying don't touch auth.js." That's a prompt. Prompts get jailbroken.

**What if the agent can't even see the code it shouldn't touch?**

## The Architecture

Separate your codebase into two repos with different security levels — like an OS kernel vs. userspace:

```
┌──────────────────────────┐   ┌──────────────────────────┐
│  🔒 kernel-repo          │   │  📦 app-repo              │
│  schema.sql              │   │  routes.js               │
│  auth.js                 │   │  components.jsx          │
│  config.js               │   │  tests/                  │
│  secrets/                │   │  docs/                   │
│                          │   │                          │
│  Kernel Agent + Human    │   │  App Agent (free)        │
│  (dual signature)        │   │  (single signature)      │
└──────────────────────────┘   └──────────────────────────┘
              ▲ Modality Contract ▲
    cooperation happens here, not through shared code
```

- **kernel-repo**: Critical infrastructure. Only the Kernel Agent can commit, and only with human co-signature. The App Agent has **zero access** — can't read, can't write.
- **app-repo**: Feature code. The App Agent works freely. No bottleneck.

The boundary isn't a linter rule. It's a **Modality verifiable contract** — mathematical constraints enforced with cryptographic signatures.

## The Contract

### Rule 1: App Agent Cannot Access Kernel Repo
```modality
rule app_cannot_touch_kernel {
  formula {
    always(
      +signed_by(/agents/app.id) implies -modifies(/kernel-repo)
    )
  }
}
```
The app agent's identity key is not authorized for kernel-repo. Period.

### Rule 2: Kernel Changes Require Dual Signature
```modality
rule kernel_requires_dual_signature {
  formula {
    always(
      +modifies(/kernel-repo) implies (
        +signed_by(/agents/kernel.id) & +signed_by(/humans/admin.id)
      )
    )
  }
}
```
Even the kernel agent can't act alone. Every kernel change needs a human.

### Rule 3: Known Signers Only
```modality
rule known_signers_only {
  formula {
    always(
      +signed_by(/agents/app.id)
      | +signed_by(/agents/kernel.id)
      | +signed_by(/humans/admin.id)
    )
  }
}
```
No anonymous commits.

## Cross-Repo Cooperation

When the app agent needs something from the kernel (a new table, a new API endpoint):

```
App Agent                      Kernel Agent              Human Admin
    │                               │                        │
    ├── REQUEST (in app-repo) ─────►│                        │
    │   "Need items table"          │                        │
    │                               ├── IMPLEMENT ──────────►│
    │                               │   (in kernel-repo,     │── APPROVE
    │                               │    app can't see this) │   (co-sign)
    │                               │◄──────────────────────┤
    │                               │                        │
    │   PUBLISH API CONTRACT ◄──────┤                        │
    │   (in app-repo — the only     │                        │
    │    thing app agent sees)      │                        │
    │                               │                        │
    ├── BUILD FEATURE              │                        │
    │   (against published API)    │                        │
```

The app agent never sees HOW the kernel implements the change. It only sees the **API contract** that gets published. Implementation details stay locked in the kernel.

## Run the Demo

```bash
npm install
npm run demo
```

Five scenarios:

1. **App agent ships a feature** → ✓ (app-repo, single signature)
2. **App agent tries to access kernel-repo** → ✗ ACCESS DENIED (can't read or write)
3. **Kernel agent acts alone** → ✗ (needs human co-signature)
4. **Proper dual-signed kernel change** → ✓ (kernel + human)
5. **Cross-repo cooperation** → Request → Implement → Publish API → Build feature

Every action uses real ed25519 signatures.

## Why Two Repos?

**One repo with folder-level access control** still lets the app agent read kernel code. It can learn how auth works, discover secrets in config files, understand the schema structure. Even if it can't write, reading is a vulnerability.

**Two repos** mean:
- The app agent's environment literally doesn't contain kernel code
- Secrets never appear in the app agent's context
- Auth implementation details can't leak through code reading
- The only interface is the published API contract

This is how operating systems work. Ring 3 processes can't read kernel memory — not because there's a rule against it, but because the memory mapping doesn't exist in their address space.

## Key Takeaways

1. **Isolation > access control.** Don't restrict what the agent can write. Restrict what it can see.
2. **Cooperation through contracts, not shared code.** Agents coordinate via formal requests and published API contracts.
3. **Dual signature for critical changes.** No agent acts alone on infrastructure. A human always co-signs.
4. **Everything is auditable.** Every commit is signed, every rule check is logged, every cooperation step is in the contract.

---

*Built with [Modality](https://modality.org) — verifiable contracts for the agentic economy.*

*Freedom at the edges. Constraints at the kernel.*
