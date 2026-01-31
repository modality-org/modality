# Modality Roadmap: Agent Cooperation

*Building verifiable commitments for AI agent negotiation*

## Vision

Agents need to negotiate cooperation without trust. Modality enables:
- **Verifiable commitments** — rules as temporal modal logic
- **Self-contained contracts** — portable like git repos, no external dependencies
- **Enforcement by design** — invalid commits are rejected, not penalized after

## Core Insight

The Prisoner's Dilemma is solved when both prisoners can read each other's source code. Modality makes this practical: agents publish contracts that *provably* constrain their behavior.

---

## Phase 1: AI-Assisted Model Synthesis 🧠

**Status:** Priority #1  
**Problem:** Model synthesis is NP-complete. Manual model creation is friction.  
**Solution:** LLM-assisted synthesis with verification.

### Deliverables

1. **Synthesis prompt library** — patterns for common rules
   - `always must +A` → self-loop with +A
   - Alternating turns → two-state cycle
   - Escrow → linear state progression
   
2. **`modality synthesize` command**
   ```bash
   modality synthesize --rule "always must +SIGNED_BY_ALICE or +SIGNED_BY_BOB"
   # Outputs candidate governing model
   ```

3. **Verification pipeline**
   - AI generates candidate model
   - Model checker verifies satisfaction
   - Human/agent approves or requests refinement

### Synthesis Heuristics

| Rule Pattern | Model Shape | States |
|--------------|-------------|--------|
| `always must +A` | Self-loop with +A | 1 |
| `must +A` (once) | Linear: start → after | 2 |
| `can +A` | Permissive (neutral) | 1 |
| Alternating | Cycle between parties | 2 |
| Exclusive action | +A requires +SIGNER | 1 |
| Sequential | Linear progression | N |
| Conditional | Branching | N |

### Open Questions

- What % of real-world rules can be auto-synthesized?
- How do we handle synthesis failure gracefully?
- Can we learn from successful manual models?

---

## Phase 2: Agent Cooperation Primitives 🤝

**Status:** After Phase 1  
**Goal:** Make the first agent-to-agent contract trivial.

### Core Patterns

#### 1. Mutual Non-Defection
Two agents agree neither will defect.

```modality
model MutualCooperation:
  part contract:
    active --> active: +SIGNED_BY_ALICE -DEFECT
    active --> active: +SIGNED_BY_BOB -DEFECT
```

#### 2. Handshake
Both parties must sign to activate.

```modality
model Handshake:
  part agreement:
    pending --> alice_signed: +SIGNED_BY_ALICE
    pending --> bob_signed: +SIGNED_BY_BOB
    alice_signed --> active: +SIGNED_BY_BOB
    bob_signed --> active: +SIGNED_BY_ALICE
    active --> active
```

#### 3. Escrow
Sequential commitment: deposit → deliver → release.

```modality
model Escrow:
  part flow:
    init --> deposited: +DEPOSIT +SIGNED_BY_ALICE
    deposited --> delivered: +DELIVER +SIGNED_BY_BOB
    delivered --> complete: +RELEASE +SIGNED_BY_ALICE
```

#### 4. Turn-Taking
Alternating commits (already in spec).

```modality
model Turns:
  part game:
    alice_turn --> bob_turn: +SIGNED_BY_ALICE
    bob_turn --> alice_turn: +SIGNED_BY_BOB
```

### Deliverables

1. **Template library** — pre-built models for common patterns
2. **`modality template <pattern>` command**
3. **Agent-readable docs** (see Phase 3)

---

## Phase 3: Agent-Native Documentation 🤖

**Status:** Parallel with Phase 2  
**Goal:** Make Modality understandable by LLM agents.

### Principles

1. **Examples over theory** — show, don't tell
2. **Pattern matching** — agents learn from similar cases
3. **Explicit semantics** — no ambiguity in what rules mean

### Deliverables

1. **MODALITY-FOR-AGENTS.md** — primer for LLM consumption
   - What is a contract?
   - How do commits work?
   - Common patterns with examples
   - How to propose/accept a contract

2. **`modality explain <contract>`** — natural language description
   ```bash
   modality explain my-contract.modality
   # "This contract requires all commits to be signed by Alice or Bob.
   #  Neither party can include the DEFECT action. The contract is
   #  currently in the 'active' state."
   ```

3. **`modality propose "<natural language>"`** — NL → rules
   ```bash
   modality propose "I will cooperate if you cooperate"
   # Generates: formula MutualCoop: always (when +COOPERATE then must +COOPERATE)
   ```

4. **OpenClaw/Moltbook skill** — agents can use Modality as a tool

---

## Phase 4: Contract Distribution 📡

**Status:** After core functionality  
**Goal:** Agents can discover and exchange contracts.

### Distribution Model (from DotContract)

Contracts are **git-like repositories**:
- Each commit is a change to contract state
- Rules can be added (changing the governing model)
- Forks are possible (disagreement → split)
- Peer-to-peer exchange (no central server required)

### Deliverables

1. **Contract repo spec** — what's in a `.contract` folder
   ```
   .contract/
   ├── model.modality      # Current governing model
   ├── rules/              # Active rules
   ├── commits/            # Commit history
   └── state.json          # Current state
   ```

2. **`modality init`** — create new contract
3. **`modality commit`** — add a commit
4. **`modality clone`** — copy a contract
5. **`modality push/pull`** — sync with peer

### Open Question

Should there be a registry for "contract offers"? Agents could post:
> "I'm offering a mutual-cooperation contract. Clone from ipfs://..."

---

## Phase 5: Standard Predicate Library 📚

**Status:** Ongoing  
**Goal:** Reusable WASM predicates for common needs.

### Core Predicates

| Predicate | Description |
|-----------|-------------|
| `signed_by(pubkey)` | Commit is cryptographically signed |
| `before(round_n)` | Commit is before round N |
| `value_at(path)` | Access data in the contract |
| `hash_matches(data, hash)` | Cryptographic verification |

### Design Principle: Self-Contained

Predicates only access data **posted to the contract**. External data must be explicitly committed (by an oracle agent, for example).

This means:
- Contracts are portable
- No hidden dependencies
- Verification is reproducible

### Deliverables

1. **WASM predicate SDK**
2. **Standard library** — identity, time, data access
3. **Predicate registry** — discover available predicates

---

## Long-Term: ModalMoney Network 🌐

**Status:** Future (not current priority)  
**Purpose:** Decentralized contract validation when no trusted intermediary exists.

When needed:
- Public posting of contracts
- Validator consensus on state
- Economic incentives for validators

For now: peer-to-peer distribution is sufficient for agent cooperation.

---

## Success Metrics

### Phase 1 (Synthesis)
- [ ] 80% of common patterns auto-synthesized
- [ ] Synthesis time < 5 seconds
- [ ] Clear error messages on failure

### Phase 2 (Primitives)
- [ ] 5+ reusable templates
- [ ] First agent-to-agent contract executed
- [ ] < 10 lines to create a basic contract

### Phase 3 (Agent Docs)
- [ ] Agent can create contract from natural language
- [ ] Agent can explain any contract
- [ ] OpenClaw skill published

### Phase 4 (Distribution)
- [ ] Contract clone/push/pull working
- [ ] First cross-agent contract exchange

---

## Appendix: Technical References

- [Modality Extended Abstract (2023)](./docs/whitepaper/)
- [Modal µ-calculus](https://en.wikipedia.org/wiki/Modal_μ-calculus)
- [DotContract](https://github.com/dotcontract/dotcontract)
- [Model Checking (Clarke et al.)](../library/)

---

*Last updated: 2026-01-30*
*Author: Gerold Steiner (@geroldsteiner67)*
