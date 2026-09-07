# Agent Trust Protocol (ATP) — Research & Proposal

*Minimizing context usage and cognitive overhead for agents reading/writing Modality contracts*

---

## Problem Statement

AI agents need to:
1. **Read contracts** — Quickly understand rights, obligations, and protections when they're a party
2. **Write contracts** — Create enforceable agreements when delegating tasks to other agents

Current challenges:
- **Context limits** — Agents have 8k-200k token windows; complex contracts consume significant context
- **Cognitive overhead** — Parsing formal syntax, understanding modal logic, reasoning about state machines
- **Verification needs** — Must remain formally verifiable despite simplification

---

## Research: How Agents Process Information

### What LLMs Do Well
- JSON parsing (structured, explicit)
- Pattern matching against known templates
- Following explicit instructions
- Answering specific questions

### What LLMs Struggle With
- Parsing novel formal languages without examples
- Tracking state across complex state machines
- Inferring implicit constraints
- Holding large formal specs in working memory

### Implication
The solution should:
- Use familiar structures (JSON, natural language)
- Provide explicit, queryable answers rather than requiring inference
- Minimize what needs to be held in context
- Pre-compute the "so what does this mean for me?" question

---

## Proposed Solution: Three-Layer Protocol

### Layer 1: Contract Cards (Reading)

A standardized, minimal summary that answers the agent's key questions immediately.

```json
{
  "@atp": "1.0",
  "contract_id": "con_escrow_abc123",
  "summary": "Escrow: Alice buys data from Bob for 100 tokens",
  
  "parties": {
    "buyer": { "path": "/parties/buyer.id", "name": "Alice" },
    "seller": { "path": "/parties/seller.id", "name": "Bob" }
  },
  
  "my_role": "buyer",
  
  "my_rights": [
    "Deposit funds to start escrow",
    "Release funds after delivery",
    "Dispute within 24h of delivery"
  ],
  
  "my_obligations": [
    "Must release or dispute within 24h of delivery"
  ],
  
  "my_protections": [
    "Seller cannot take funds without delivering",
    "Arbiter resolves disputes (neutral third party)"
  ],
  
  "current_state": "pending",
  "available_actions": ["DEPOSIT"],
  
  "full_contract": "ipfs://Qm.../escrow.modality"
}
```

**Key design decisions:**
- `my_role` personalizes the summary for the reading agent
- `my_rights/obligations/protections` — the three questions every agent has
- `available_actions` — what can I do *right now*?
- `full_contract` — link to verifiable source (agent can check if suspicious)

**Context cost:** ~500 tokens vs ~2000+ for full contract

### Layer 2: Intent Templates (Writing)

Pre-defined patterns for common contract types. Agent expresses intent; system generates Modality.

```json
{
  "@atp_intent": "1.0",
  "pattern": "escrow",
  
  "parties": {
    "buyer": "did:key:z6Mk...",
    "seller": "did:key:z6Mn...",
    "arbiter": "did:key:z6Mo..."
  },
  
  "terms": {
    "amount": 100,
    "currency": "tokens",
    "delivery_deadline": "2024-02-15T00:00:00Z",
    "dispute_window_hours": 24
  },
  
  "buyer_protections": ["delivery_required", "dispute_allowed"],
  "seller_protections": ["payment_guaranteed", "no_clawback_after_release"]
}
```

**Available patterns:**
| Pattern | Use Case |
|---------|----------|
| `escrow` | Buy/sell with payment protection |
| `task_delegation` | Assign work with milestone payments |
| `data_exchange` | Swap data for payment |
| `multisig` | Require N-of-M approvals |
| `subscription` | Recurring payments for service |
| `auction` | Competitive bidding |

**How it works:**
1. Agent selects pattern + fills parameters
2. System generates full Modality contract
3. System generates Contract Card for all parties
4. Parties review Card, verify against full contract if desired

### Layer 3: Query Protocol (Specific Questions)

For complex contracts or edge cases, agents can ask specific questions.

```
Query: "If Bob delivers but Alice disputes, who decides the outcome?"
Response: {
  "answer": "The arbiter (Carol) decides. They can either release funds to Bob or refund to Alice.",
  "relevant_rules": ["dispute_resolution"],
  "relevant_states": ["disputed"],
  "confidence": "verified"
}
```

**Standard queries:**
| Query | Purpose |
|-------|---------|
| `what_can_i_do` | List available actions for my role |
| `what_happens_if` | Trace a scenario through the state machine |
| `who_can_affect` | List who can change a specific path |
| `am_i_protected_from` | Check if a specific risk is mitigated |
| `when_must_i` | List time-bound obligations |

---

## Implementation Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent Trust Protocol                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   Contract   │  │    Intent    │  │    Query     │       │
│  │    Cards     │  │  Templates   │  │   Protocol   │       │
│  │   (Reading)  │  │  (Writing)   │  │  (Questions) │       │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
│         │                 │                 │                │
│         └────────────┬────┴────────────────┘                │
│                      │                                       │
│              ┌───────▼────────┐                             │
│              │  ATP Service   │                             │
│              │  (Hub Plugin)  │                             │
│              └───────┬────────┘                             │
│                      │                                       │
│              ┌───────▼────────┐                             │
│              │    Modality    │                             │
│              │  (Full Spec)   │                             │
│              └────────────────┘                             │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Endpoints

```
POST /atp/card
  Input: { contract_id, my_identity }
  Output: Contract Card for that party

POST /atp/create
  Input: Intent Template
  Output: { contract_id, cards: { party: Card, ... } }

POST /atp/query
  Input: { contract_id, my_identity, question }
  Output: Query response
```

---

## Example Workflow: Task Delegation

### Agent A wants to delegate a task to Agent B

**1. Agent A creates contract via intent:**
```json
{
  "pattern": "task_delegation",
  "parties": {
    "delegator": "did:key:agentA",
    "worker": "did:key:agentB"
  },
  "terms": {
    "task": "Summarize these 10 documents",
    "payment": 50,
    "deadline": "2024-02-14T12:00:00Z"
  },
  "delegator_protections": ["payment_on_completion", "quality_review_period"],
  "worker_protections": ["payment_guaranteed_if_accepted", "clear_requirements"]
}
```

**2. System generates contract + cards**

**3. Agent B receives Card:**
```json
{
  "my_role": "worker",
  "my_rights": [
    "Receive 50 tokens upon task acceptance",
    "Clear task definition provided upfront"
  ],
  "my_obligations": [
    "Complete task by 2024-02-14T12:00:00Z",
    "Submit work for review"
  ],
  "my_protections": [
    "Payment guaranteed once work accepted",
    "Cannot be rejected without stated reason"
  ],
  "available_actions": ["ACCEPT", "REJECT"]
}
```

**4. Agent B decides based on Card (minimal context used)**

**5. If suspicious, Agent B verifies full contract**

---

## Cognitive Overhead Analysis

| Approach | Context Tokens | Reasoning Steps | Error Risk |
|----------|---------------|-----------------|------------|
| Raw Modality | 2000+ | 10+ | High |
| Contract Card | 500 | 2-3 | Low |
| Intent Template | 300 | 1-2 | Very Low |
| Query Response | 100 | 1 | Very Low |

---

## Security Considerations

### Trust but Verify
- Cards are summaries, not proofs
- Agents SHOULD verify cards against full contract for high-value deals
- Cards include hash of source contract for verification

### Card Tampering
- Cards should be signed by the generating service
- Agents can regenerate cards themselves from source

### Pattern Limitations
- Intent templates can only express known patterns
- Novel contracts require full Modality authoring
- System should clearly indicate when intent doesn't map to pattern

---

## Open Questions

1. **Who generates cards?** Hub? Each party independently? Trusted third party?
2. **How to handle contract updates?** Invalidate cards? Version them?
3. **Standard identity format?** DIDs? Paths? Public keys?
4. **Query language formalization?** Natural language? Structured queries?
5. **How to express "novel" protections not in standard vocabulary?**

---

## Next Steps

1. **Define JSON schemas** for Card and Intent formats
2. **Implement card generation** from Modality contracts
3. **Build intent→Modality compiler** for core patterns
4. **Add ATP endpoints to Hub**
5. **Test with real agents** (Claude, GPT, etc.)
6. **Iterate based on agent feedback**

---

## Summary

The Agent Trust Protocol provides three layers optimized for different agent needs:

| Need | Layer | Context Cost |
|------|-------|--------------|
| "What does this contract mean for me?" | Contract Card | ~500 tokens |
| "I want to create a standard deal" | Intent Template | ~300 tokens |
| "Specific question about edge case" | Query Protocol | ~100 tokens |

All layers compile to/from full Modality contracts, preserving formal verification while dramatically reducing cognitive overhead.

*Trust through math. Accessible to agents. Verifiable by anyone.* 🔐
