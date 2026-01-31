# LLM Synthesizer Experiment Log

## Session: 2026-01-31 23:35 UTC

### Approach

Instead of building a separate tool that calls Claude, I (Gerold) will act as the synthesizer directly. This gives me real-time access to Claude's capabilities while allowing me to iterate on the synthesis approach.

### Experiment 1: Simple Escrow

**Input:** "Alice wants to buy something from Bob. She deposits payment, Bob delivers the goods, then Alice releases the payment."

**Analysis:**
- Parties: Alice (buyer), Bob (seller)
- Trust requirements: Alice doesn't want to pay without receiving goods; Bob doesn't want to deliver without getting paid
- Flow: deposit → deliver → release

**Synthesized Model:**

```modality
model SimpleEscrow {
  part flow {
    pending --> deposited: +DEPOSIT +signed_by(/users/alice.id)
    deposited --> delivered: +DELIVER +signed_by(/users/bob.id)
    delivered --> complete: +RELEASE +signed_by(/users/alice.id)
    complete --> complete
  }
}
```

**Rule (Alice's protection):**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+RELEASE] implies <+DELIVER> true
    )
  }
}
```
*Meaning: Alice's RELEASE action can only happen after Bob's DELIVER.*

**Rule (Bob's protection):**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+DELIVER] implies <+DEPOSIT> true
    )
  }
}
```
*Meaning: Bob's DELIVER action can only happen after Alice's DEPOSIT.*

**Protections:**
- Alice: Cannot release payment without first receiving delivery
- Bob: Won't deliver until deposit is secured

---

### Experiment 2: Atomic Data Exchange

**Input:** "Two AI agents want to exchange datasets. Neither should receive data without the other receiving theirs too."

**Analysis:**
- Parties: AgentA, AgentB
- Trust requirements: Atomicity - both commit before either reveals
- Pattern: Atomic swap with hash-locked commitments

**Synthesized Model:**

```modality
model AtomicDataExchange {
  part exchange {
    init --> a_committed: +COMMIT_A +signed_by(/users/agent_a.id)
    init --> b_committed: +COMMIT_B +signed_by(/users/agent_b.id)
    a_committed --> both_committed: +COMMIT_B +signed_by(/users/agent_b.id)
    b_committed --> both_committed: +COMMIT_A +signed_by(/users/agent_a.id)
    both_committed --> a_revealed: +REVEAL_A +signed_by(/users/agent_a.id)
    both_committed --> b_revealed: +REVEAL_B +signed_by(/users/agent_b.id)
    a_revealed --> complete: +REVEAL_B +signed_by(/users/agent_b.id)
    b_revealed --> complete: +REVEAL_A +signed_by(/users/agent_a.id)
    complete --> complete
  }
}
```

**Rule (Atomicity):**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+REVEAL_A] implies (
        <+COMMIT_A> true & <+COMMIT_B> true
      )
    ) & always (
      [+REVEAL_B] implies (
        <+COMMIT_A> true & <+COMMIT_B> true
      )
    )
  }
}
```
*Meaning: Neither party can reveal their data until both have committed.*

**Protections:**
- AgentA: Data not revealed until AgentB has also committed
- AgentB: Data not revealed until AgentA has also committed

---

### Experiment 3: Delegation with Revocation

**Input:** "AgentA delegates authority to AgentB to perform tasks on their behalf. AgentA can revoke this at any time."

**Analysis:**
- Parties: AgentA (principal), AgentB (delegate)
- Trust requirements: Principal maintains control; delegate has bounded authority
- Pattern: Delegation with revocation

**Synthesized Model:**

```modality
model Delegation {
  part authority {
    init --> delegated: +DELEGATE +signed_by(/users/agent_a.id)
    delegated --> delegated: +ACT_ON_BEHALF +signed_by(/users/agent_b.id)
    delegated --> revoked: +REVOKE +signed_by(/users/agent_a.id)
    revoked --> revoked
  }
}
```

**Rule (Scope of authority):**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+ACT_ON_BEHALF] implies (
        <+DELEGATE> true & !<+REVOKE> true
      )
    )
  }
}
```
*Meaning: AgentB can only act after delegation and before revocation.*

**Protections:**
- AgentA: Can revoke at any time; AgentB cannot act after revocation
- AgentB: Actions are valid as long as delegation is active

---

### Experiment 4: Multi-party Approval (2-of-3)

**Input:** "A DAO has 3 members. Any action requires approval from at least 2 of them."

**Analysis:**
- Parties: Member1, Member2, Member3
- Trust requirements: Quorum (2-of-3)
- Pattern: Multisig threshold

**Synthesized Model:**

```modality
model Multisig2of3 {
  part approval {
    proposed --> signed_1: +SIGN +signed_by(/users/member1.id)
    proposed --> signed_2: +SIGN +signed_by(/users/member2.id)
    proposed --> signed_3: +SIGN +signed_by(/users/member3.id)
    signed_1 --> approved: +SIGN +signed_by(/users/member2.id)
    signed_1 --> approved: +SIGN +signed_by(/users/member3.id)
    signed_2 --> approved: +SIGN +signed_by(/users/member1.id)
    signed_2 --> approved: +SIGN +signed_by(/users/member3.id)
    signed_3 --> approved: +SIGN +signed_by(/users/member1.id)
    signed_3 --> approved: +SIGN +signed_by(/users/member2.id)
    approved --> executed: +EXECUTE
    executed --> executed
  }
}
```

**Rule (Quorum requirement):**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+EXECUTE] implies (
        (<+signed_by(/users/member1.id)> true & <+signed_by(/users/member2.id)> true) |
        (<+signed_by(/users/member1.id)> true & <+signed_by(/users/member3.id)> true) |
        (<+signed_by(/users/member2.id)> true & <+signed_by(/users/member3.id)> true)
      )
    )
  }
}
```

**Protections:**
- All members: No single member can execute alone; requires consensus

---

### Experiment 5: Milestone-based Project

**Input:** "A contractor has 3 milestones: Design, Build, Test. The client pays after each milestone is delivered and approved."

**Analysis:**
- Parties: Client, Contractor
- Trust requirements: Payment tied to delivery; progress is incremental
- Pattern: Sequential milestones with payment gates

**Synthesized Model:**

```modality
model MilestoneProject {
  part project {
    init --> started: +START +signed_by(/users/client.id)
    started --> design_done: +DELIVER_DESIGN +signed_by(/users/contractor.id)
    design_done --> design_paid: +APPROVE_DESIGN +PAY_DESIGN +signed_by(/users/client.id)
    design_paid --> build_done: +DELIVER_BUILD +signed_by(/users/contractor.id)
    build_done --> build_paid: +APPROVE_BUILD +PAY_BUILD +signed_by(/users/client.id)
    build_paid --> test_done: +DELIVER_TEST +signed_by(/users/contractor.id)
    test_done --> complete: +APPROVE_TEST +PAY_TEST +signed_by(/users/client.id)
    complete --> complete
  }
}
```

**Rule (Payment requires delivery):**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+PAY_DESIGN] implies <+DELIVER_DESIGN> true
    ) & always (
      [+PAY_BUILD] implies <+DELIVER_BUILD> true
    ) & always (
      [+PAY_TEST] implies <+DELIVER_TEST> true
    )
  }
}
```

**Protections:**
- Client: Only pays after each milestone is delivered
- Contractor: Receives payment after each approved delivery

---

## Key Insights

1. **State machines are natural for sequential flows** - escrow, milestones, delegations all map cleanly to linear or branching state progressions.

2. **Atomic operations need commitment phases** - any "both or neither" requirement needs explicit commit-before-reveal patterns.

3. **Formulas express invariants** - "always X implies Y" is the workhorse for conditional requirements.

4. **Signature requirements attach to transitions** - `+signed_by(path)` is how you specify who can take an action.

5. **The diamondbox `[<+A>]` is powerful** - "committed to A" (can do A, cannot refuse) expresses obligations cleanly.

## Validation Results

All synthesized models were validated against the modality-lang parser:

| Model | Status | States | Transitions |
|-------|--------|--------|-------------|
| SimpleEscrow | ✅ PASS | 4 | 4 |
| AtomicDataExchange | ✅ PASS | 7 | 9 |
| Delegation | ✅ PASS | 3 | 4 |
| Multisig2of3 | ✅ PASS | 6 | 11 |
| MilestoneProject | ✅ PASS | 8 | 8 |
| AgentSwarm | ✅ PASS | 7 | 15 |
| ReputationEscrow | ✅ PASS | 7 | 8 |
| DutchAuction | ✅ PASS | 7 | 8 |

## Additional Examples Created

### Agent Swarm Coordination
- Coordinator assigns tasks to multiple workers
- Workers complete independently  
- Coordinator aggregates when enough workers are done

### Reputation-Gated Escrow
- Buyer deposits
- Oracle checks seller reputation
- Proceed or refund based on reputation check

### Dutch Auction
- Seller lists item
- Price decreases over time
- First acceptable bid wins

## Next Steps

1. ~~Validate these models against the modality-lang parser~~ ✅ DONE
2. ~~Create more complex scenarios (auctions, subscriptions, swarms)~~ ✅ DONE  
3. ~~Build a pattern library from successful syntheses~~ ✅ DONE (see SYNTHESIS_GUIDE.md)
4. Integrate LLM synthesis into CLI workflow
5. Add rule synthesis from protection descriptions
6. Build interactive refinement loop
