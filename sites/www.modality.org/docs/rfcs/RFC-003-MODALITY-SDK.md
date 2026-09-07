# RFC-003: The Modality SDK — Canonical TypeScript Library

**Status:** Draft  
**Authors:** Gerold Steiner, rarepepi  
**Created:** 2026-03-30  

## The Pitch

**Stop writing buggy state machines. Write rules.**

Every agent platform has developers hand-rolling state machines in TypeScript to limit what their agents can do. They write `if/else` chains, action allowlists, state enums, transition validators — all in imperative code that can have bugs. Then they ship it and pray.

The Modality SDK replaces all of that with formally verified rules. You declare what's allowed. The math guarantees it. You sleep at night.

## Name

`@modality/sdk` on npm. Import as `modality`.

## Target Audience

**Primary:** Senior to staff-level engineers building agent systems who are currently hand-rolling state machines and guardrails in TS/Python. They know the pain. They've shipped bugs in their constraint code. They want something better.

**Secondary (downstream):** Once senior engineers adopt it, vibe coders and junior devs follow. "Just use `@modality/sdk`" becomes the answer on Stack Overflow and Discord.

## One-Pager Structure

### Hero Section

**Headline:** "Formally verified guardrails for AI agents."  
**Subhead:** "Stop writing buggy state machines. Declare your rules. Ship with confidence."

**Hero code:**

```typescript
import { Guard, rules } from '@modality/sdk'
import { generateText } from 'ai'
import { openai } from '@ai-sdk/openai'

const guard = Guard.create({
  rules: rules`
    agent can read emails
    agent can draft emails
    agent cannot send emails without approval
    agent cannot delete anything
    agent cannot spend more than $100
  `
})

const result = await generateText({
  model: openai('gpt-4o'),
  tools: guard.protect(myTools),
  prompt: 'Check my inbox and draft replies',
})
// tools are wrapped — violations are caught before execution
// not with prompts. not with regex. with math.
```

### Feature Grid (6 cards)

**1. Rules, Not Code**
Write what's allowed in plain declarative rules. No if/else chains, no state enums, no transition tables. The SDK compiles rules into a formally verified state machine.

**2. Drop-In Protection**
`guard.protect(tools)` wraps your existing Vercel AI SDK, LangChain, or custom tools. One line to add guardrails to any agent.

**3. Formally Verified**
Rules compile to modal logic formulas validated against a state machine. If a rule says "never delete" — it's mathematically impossible, not probabilistically unlikely.

**4. Audit Trail**
Every tool call becomes a signed commit in an append-only log. Full history of what your agent did, tried to do, and was blocked from doing.

**5. Human Approval Flows**
Some actions need a human. `guard.requireApproval('email.send')` holds the action until a designated approver signs off. Built-in, not bolted-on.

**6. Works Everywhere**
TypeScript/JavaScript. Works with Vercel AI SDK, LangChain, AutoGen, CrewAI, or raw function calls. If it's a function, we can guard it.

### How It Works (3 steps)

**Step 1: Define rules**
```typescript
const guard = Guard.create({
  rules: rules`
    agent can read files in /workspace
    agent can create files in /workspace
    agent cannot modify files outside /workspace
    agent cannot execute shell commands
    agent cannot make network requests to external URLs
  `
})
```

**Step 2: Protect your tools**
```typescript
const protectedTools = guard.protect({
  readFile: readFileTool,
  writeFile: writeFileTool,
  exec: execTool,          // will be blocked
  fetch: fetchTool,        // will be blocked
})
```

**Step 3: Use normally**
```typescript
const result = await generateText({
  model: anthropic('claude-sonnet-4-20250514'),
  tools: protectedTools,
  prompt: userMessage,
})
// readFile → ✅ allowed
// writeFile('/workspace/draft.md') → ✅ allowed
// exec('rm -rf /') → ❌ blocked by contract
// fetch('https://evil.com') → ❌ blocked by contract
```

### The Problem We Solve

**What developers do today:**

```typescript
// Hand-rolled state machine (real code from production agents)
const ALLOWED_STATES = ['idle', 'drafting', 'reviewing'] as const
type State = typeof ALLOWED_STATES[number]

function canTransition(from: State, action: string): boolean {
  if (from === 'idle' && action === 'draft') return true
  if (from === 'drafting' && action === 'review') return true
  if (from === 'reviewing' && action === 'approve') return true
  if (from === 'reviewing' && action === 'reject') return true
  // Bug: forgot to handle 'drafting' -> 'idle' (cancel)
  // Bug: 'reviewing' -> 'drafting' allows infinite loops
  // Bug: no check for who is performing the action
  return false
}

function executeAction(state: State, action: string, params: any) {
  if (!canTransition(state, action)) throw new Error('Invalid')
  // More bugs: params aren't validated
  // More bugs: no audit trail
  // More bugs: race conditions in concurrent access
  return performAction(action, params)
}
```

**What they do with Modality:**

```typescript
const guard = Guard.create({
  rules: rules`
    agent can draft from idle
    agent can review from drafting  
    agent can approve or reject from reviewing
    agent can cancel from drafting back to idle
    reviewing cannot loop back to drafting
    all actions require agent signature
  `
})
// No bugs. No forgotten transitions. No unchecked params.
// The state machine is generated and verified automatically.
```

### Comparison Table

| | Hand-rolled TS | Prompt engineering | **Modality SDK** |
|---|---|---|---|
| Formally verified | ❌ | ❌ | ✅ |
| Bugs possible | ✅ Yes | ✅ Yes | ❌ No |
| Audit trail | DIY | ❌ | ✅ Built-in |
| Human approval | DIY | ❌ | ✅ Built-in |
| Works when jailbroken | N/A | ❌ | ✅ |
| Setup time | Days | Minutes | Minutes |
| Confidence level | 🤞 Hope | 🙏 Prayer | 🔐 Proof |

### Social Proof Section

Quotes from the team / early users:

> "We replaced 400 lines of state machine code with 12 lines of Modality rules. Then we found 3 bugs in the old code that would have let agents send unauthorized emails."

> "I used to wake up at 3am worried about what our agents did overnight. Now I sleep through."

> "The audit trail alone is worth it. We can show compliance exactly what our agents did and prove they couldn't have done anything else."

### CTA

"Add guardrails to your agent in 5 minutes."

```bash
npm install @modality/sdk
```

[Get Started →] [Read the Docs →] [GitHub →]

## SDK Architecture

### Core Modules

```
@modality/sdk
├── Guard          — main entry point, wraps tools
├── rules          — tagged template for rule DSL
├── Contract       — low-level contract management
├── Hub            — ModalHub client (push/pull/validate)
├── Signer         — ed25519 key management
└── adapters/
    ├── vercel-ai  — Vercel AI SDK adapter
    ├── langchain  — LangChain adapter
    └── generic    — wrap any async function
```

### The `rules` Template Literal

This is the magic. A tagged template that parses natural-ish rule declarations and compiles them to Modality formulas + model transitions:

```typescript
const r = rules`
  agent can read emails
  agent cannot delete emails
  agent can send emails with approval from admin
  agent cannot spend more than $100 per transaction
`

// Compiles to:
// model {
//   initial active
//   active -> active [+any_signed(/) +calls(email.read)]
//   active -> active [+any_signed(/) +calls(email.send) +signed_by(/admin.id)]
//   // No transition for email.delete → impossible
// }
// rule { formula { always (+param_max(amount, 100)) } }
```

The rules DSL is intentionally constrained — you can't express arbitrary logic, only guardrail patterns. This is a feature, not a limitation. It means you can't introduce bugs because the language doesn't let you express buggy things.

### Guard.protect()

Takes a tools object and returns a wrapped version where every tool call:

1. Creates a commit describing the tool call + params
2. Validates the commit against the compiled contract
3. If valid → executes the original function, logs result
4. If rejected → returns structured error to the LLM
5. Persists the commit to the audit log (local or hub)

```typescript
guard.protect(tools, {
  onBlocked: (tool, params, reason) => {
    // Custom handler for blocked calls
    console.log(`Blocked ${tool}: ${reason}`)
  },
  onExecuted: (tool, params, result) => {
    // Custom handler for successful calls
  },
  hub: 'https://api.modalhub.com',  // optional: sync to hub
  localAudit: './audit.log',         // optional: local audit file
})
```

## Implementation Plan

### Phase 1: Core SDK (2 weeks)
- [ ] `Guard.create()` with static rule validation
- [ ] `rules` tagged template parser (subset of patterns)
- [ ] Rule → Modality formula compiler
- [ ] Formula → state machine model generator
- [ ] `guard.protect()` tool wrapper
- [ ] Local validation (no hub dependency)
- [ ] Vercel AI SDK adapter

### Phase 2: Hub Integration (1 week)
- [ ] Push/pull contracts to ModalHub
- [ ] Signed commits with ed25519
- [ ] Audit log sync
- [ ] Remote validation

### Phase 3: Advanced Features (1 week)
- [ ] `guard.requireApproval()` flow
- [ ] Parameter validation (`param_max`, `param_in`)
- [ ] LangChain adapter
- [ ] Generic function adapter

### Phase 4: One-Pager & Launch (1 week)
- [ ] Build one-pager site (use magicui template as base)
- [ ] npm publish `@modality/sdk`
- [ ] Documentation site
- [ ] Launch blog post
- [ ] Twitter thread
- [ ] Hacker News post

## The `rules` DSL — Supported Patterns

Starting with a focused set of patterns that cover 90% of use cases:

```
agent can <action>                          → allow tool call
agent cannot <action>                       → block tool call (no transition)
agent can <action> with approval from <role> → require co-signature
agent cannot <action> <object>              → block specific target
agent can <action> in <path>                → scope to path prefix
agent cannot <action> outside <path>        → inverse scope
agent cannot spend more than <n>            → param_max
agent can only <action> to <list>           → param_in
all actions require signature               → always(+any_signed(/))
rules cannot be changed                     → always(-adds_rule)
```

Not Turing-complete. By design. You literally cannot write a bug in this language because it only expresses constraints, not computation.

## Open Questions

1. **Rule compilation:** Do we compile rules client-side (fast, offline) or server-side (hub validates)? Probably both — local for dev, hub for production.
2. **Natural language ambiguity:** How strict is the `rules` DSL? Should we use an LLM to parse natural language into formal rules, or keep it structured? Leaning structured for v1.
3. **Python SDK:** Do we ship a Python version simultaneously or TS-first? TS-first, Python fast-follow.
4. **Pricing:** Open source core, paid hub? Or fully open source? Open source SDK, ModalHub is the business.
