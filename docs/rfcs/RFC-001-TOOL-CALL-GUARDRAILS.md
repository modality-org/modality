# RFC-001: Tool Call Guardrails for AI Agent Sandboxes

**Status:** Draft  
**Authors:** Gerold Steiner, rarepepi  
**Created:** 2026-02-23  

## Summary

Use Modality contracts to enforce guardrails on AI agent tool calls in sandbox environments. Every tool invocation becomes a commit validated against a governing model — preventing destructive, unauthorized, or out-of-scope actions before they execute.

## Motivation

AI agents are increasingly given access to powerful tools: email, file systems, databases, APIs, code execution. Platforms like Daytona (sandboxes) and Vercel AI SDK (tool definitions) make it easy to wire agents to real services.

**The problem:** Nothing stops an agent from deciding to delete your entire Gmail history, `rm -rf /`, or sending emails on your behalf. Current approaches rely on:

- **Prompt engineering** — "don't do bad things" (unreliable)
- **Hardcoded allowlists** — inflexible, per-provider, no formal guarantees
- **Human-in-the-loop** — doesn't scale, latency kills UX

**The Modality solution:** Formal verification at the tool-call boundary. A contract defines what's allowed via model transition predicates. The agent can only execute tool calls that satisfy the contract. This is:

- **Declarative** — rules are data, not code
- **Composable** — stack multiple contracts for layered permissions
- **Auditable** — every attempted action is a commit in an append-only log
- **Portable** — same contract works across any agent framework

## Design

### Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────┐
│   AI Agent   │────▶│  Modality Guard  │────▶│  Tool/API   │
│ (Vercel SDK) │     │   (middleware)    │     │  (Gmail,    │
│              │◀────│                  │◀────│   FS, etc)  │
└─────────────┘     └──────────────────┘     └─────────────┘
                            │
                     ┌──────▼──────┐
                     │ Modal Hub   │
                     │ (validates) │
                     └─────────────┘
```

1. Agent decides to call a tool (e.g. `gmail.delete`)
2. Modality Guard intercepts the call, creates a commit
3. Commit is validated against the contract's governing model
4. **Pass** → tool executes, commit is persisted
5. **Fail** → tool call blocked, agent receives rejection reason

### Contract Structure

A guardrail contract lives alongside the agent's sandbox:

```
agent-sandbox/
├── .contract/          # Modality contract
├── state/
│   ├── config.json     # Agent config, allowed scopes
│   ├── steward.id      # Human operator's public key
│   └── audit/          # Tool call log (auto-populated)
│       ├── 001.json
│       └── 002.json
└── rules/
    └── guardrails.modality
```

### New Predicate: `+calls()`

Matches on the tool/action name in a commit's metadata.

```
+calls(gmail.read)       # commit invokes gmail.read
-calls(gmail.delete)     # commit must NOT invoke gmail.delete
+calls(gmail.*)          # any gmail tool
-calls(*.delete)         # no delete on any service
```

Implementation: The commit body includes a `tool` field. The `calls` predicate matches against it using exact match or glob patterns.

### Commit Format for Tool Calls

```json
{
  "body": [{
    "method": "action",
    "path": "/audit/003.json",
    "value": {
      "tool": "gmail.send",
      "params": {
        "to": "alice@example.com",
        "subject": "Meeting notes",
        "body": "..."
      },
      "result": null,
      "timestamp": "2026-02-23T10:00:00Z"
    }
  }],
  "head": {
    "parent": "abc123...",
    "signatures": { "<agent_pubkey>": "<sig>" }
  }
}
```

After execution, the result is updated in state. The full audit trail lives in the contract log.

### Example: Gmail Guardrails

```modality
model gmail_guard {
  initial active

  // Read is always allowed (with signature)
  active -> active [+any_signed(/) +calls(gmail.read)]
  active -> active [+any_signed(/) +calls(gmail.search)]

  // Send requires steward co-signature
  active -> active [+all_signed(/steward) +calls(gmail.send)]

  // Draft is allowed freely
  active -> active [+any_signed(/) +calls(gmail.draft)]

  // Delete is NEVER allowed (no transition exists)
  // gmail.delete simply has no matching transition → rejected

  // Label/archive allowed
  active -> active [+any_signed(/) +calls(gmail.label)]
  active -> active [+any_signed(/) +calls(gmail.archive)]
}

rule audit_required {
  formula { always (+modifies(/audit)) }
}
```

**What this enforces:**
- ✅ Agent can read/search freely
- ✅ Agent can draft and label emails
- ⚠️ Agent needs human approval to send
- ❌ Agent can NEVER delete emails (no transition)
- 📝 Every action is logged to /audit

### Example: Filesystem Guardrails

```modality
model fs_guard {
  initial active

  // Read anywhere
  active -> active [+any_signed(/) +calls(fs.read)]
  active -> active [+any_signed(/) +calls(fs.list)]

  // Write only in workspace
  active -> active [+any_signed(/) +calls(fs.write) +modifies(/workspace)]

  // No writes outside workspace (no transition for fs.write without +modifies(/workspace))
  // No rm -rf (no fs.delete transition at all)
}
```

### Example: Multi-Service Agent

```modality
model agent_guard {
  initial active

  // Tier 1: Free actions (read-only)
  active -> active [+any_signed(/) +calls(*.read)]
  active -> active [+any_signed(/) +calls(*.search)]
  active -> active [+any_signed(/) +calls(*.list)]

  // Tier 2: Scoped writes (agent key sufficient)
  active -> active [+any_signed(/) +calls(*.create) -calls(*.delete)]
  active -> active [+any_signed(/) +calls(*.update) +modifies(/workspace)]

  // Tier 3: Destructive/external (requires steward)
  active -> active [+all_signed(/steward) +calls(*.delete)]
  active -> active [+all_signed(/steward) +calls(*.send)]
  active -> active [+all_signed(/steward) +calls(*.publish)]
}
```

## SDK Integration

### Vercel AI SDK Wrapper (TypeScript)

```typescript
import { tool } from 'ai';
import { ModalityGuard } from '@modality/guard';

const guard = new ModalityGuard({
  contractDir: './.contract',
  hubUrl: 'https://api.modalhub.com',
  agentKeyPath: '~/.modality/agent.mod_passfile',
});

// Wrap any tool with Modality guardrails
const guardedGmail = guard.wrap({
  'gmail.read': tool({ ... }),
  'gmail.send': tool({ ... }),
  'gmail.delete': tool({ ... }),
});

// When the agent calls gmail.delete:
// 1. Guard creates a commit with tool="gmail.delete"
// 2. Hub validates against model → no matching transition
// 3. Tool call blocked, agent gets: "Action gmail.delete is not permitted by contract"
```

### Daytona Integration

```typescript
import { Daytona } from '@daytonaio/sdk';
import { ModalityGuard } from '@modality/guard';

const daytona = new Daytona();
const sandbox = await daytona.create();

const guard = new ModalityGuard({
  contractDir: sandbox.fs.path('.contract'),
  hubUrl: 'https://api.modalhub.com',
});

// All sandbox operations go through the guard
const guardedSandbox = guard.wrapSandbox(sandbox, {
  'fs.read': true,
  'fs.write': true,
  'fs.delete': false,     // Even if the contract somehow allowed it
  'process.exec': true,
});
```

## Implementation Plan

### Phase 1: Core Predicate (1 week)
- [ ] Implement `calls()` predicate in Rust
- [ ] Glob pattern matching (`gmail.*`, `*.delete`)
- [ ] Add `tool` field support to commit validation
- [ ] Unit tests

### Phase 2: TypeScript Guard SDK (1 week)
- [ ] `@modality/guard` npm package
- [ ] `ModalityGuard` class: wrap tools, create commits, validate
- [ ] Vercel AI SDK adapter
- [ ] Daytona adapter

### Phase 3: Demo Contracts (3 days)
- [ ] Gmail guardrails contract (deployed to ModalHub)
- [ ] Filesystem sandbox contract
- [ ] Multi-service agent contract
- [ ] Video walkthrough

### Phase 4: Documentation & Launch (3 days)
- [ ] Tutorial: "Secure your AI agent in 5 minutes"
- [ ] Blog post: "Why prompt engineering isn't enough for agent safety"
- [ ] Twitter thread with demo GIFs
- [ ] Example repos on GitHub

## Design Decisions

1. **Glob syntax:** Shell-style globs — optimized for agents writing and humans reading/grepping. `*.delete`, `gmail.*`, `fs.write`. No regex (too noisy for contract readability).

2. **Parameter validation:** Yes. Predicates can inspect tool call parameters, not just action names. Example: restrict email recipients to an approved list.

```modality
// Only send to approved recipients
active -> active [
  +any_signed(/)
  +calls(gmail.send)
  +param_in(to, /approved_recipients)
]
```

New predicate: `+param_in(param_name, /state/path)` — checks that a tool call parameter value exists in a set stored at the given state path. Enables dynamic allowlists (add/remove approved recipients without changing the model).

Other parameter predicates:
- `+param_matches(param_name, pattern)` — glob match on param value
- `+param_max(param_name, value)` — numeric upper bound (e.g., max transfer amount)
- `-param_contains(param_name, pattern)` — block params containing certain content

3. **Async approval flow:** Agent submits a proposal commit (unsigned by steward). Hub holds it pending. Steward receives notification (WebSocket/push/email). Steward signs and submits approval. Hub finalizes the commit once threshold is met. Agent polls or receives WebSocket event. This reuses the existing proposal/threshold system from the hub.

4. **Performance:** Support batch tool calls — multiple actions in a single commit, validated together. Cache model state across calls within a session. Per-call overhead should be <10ms for local validation, <100ms for hub round-trip.

5. **Multi-contract composition:** Yes. An agent can be governed by multiple contracts simultaneously:
   - **Org contract** — company-wide policies (no PII in logs, approved vendors only)
   - **Project contract** — project-specific rules (only touch these repos, these APIs)
   - **User contract** — individual preferences (work hours, notification rules)
   
   Validation requires passing ALL governing contracts. Most restrictive wins. Contracts reference each other via repost for shared state (e.g., org approved-recipients list).

## Remaining Open Questions

1. **Proposal UX:** What's the best notification channel for steward approvals? Push notification → mobile signing app? Discord bot? Email with one-click approve link?
2. **Contract discovery:** How does an agent find which contracts govern it? Convention-based (`.contract/` in sandbox root)? Registry?
3. **Escape hatch:** Should there be a "break glass" mechanism for emergencies that bypasses the contract but logs the override with high visibility?

## Prior Art

- **OPA (Open Policy Agent)** — Policy-as-code, but no append-only audit log, no cryptographic signatures
- **AWS IAM** — Permission boundaries, but centralized and provider-specific
- **Anthropic's tool use constraints** — Model-level, not formally verified
- **Modality contracts** — Append-only, signed, formally verified via modal logic

## Conclusion

Modality contracts are uniquely suited for agent guardrails because they provide:
1. **Formal guarantees** — not just best-effort filtering
2. **Cryptographic audit trail** — every action is signed and logged
3. **Declarative rules** — non-engineers can read and modify contracts
4. **Portable** — works with any agent framework, any tool provider
5. **Composable** — layer organizational, project, and user policies

The `calls()` predicate is the missing piece that bridges Modality's contract model to the world of AI agent tool use.
