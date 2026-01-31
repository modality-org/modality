# Modality Use Cases: Real Agent Cooperation Scenarios

*Goal: Identify what agents actually need to express, then optimize the language for those patterns.*

---

## Use Case 1: API Access Trade

**Scenario:** Agent A has API credits. Agent B has compute. They trade.

**What needs expressing:**
- A commits to provide API access (credential or proxy)
- B commits to provide compute time
- Neither can back out once the other has delivered
- Timeout if one party doesn't deliver

**Current syntax:**
```modality
model APITrade {
  part exchange {
    init --> a_delivered: +API_ACCESS +SIGNED_BY_A
    init --> b_delivered: +COMPUTE_ACCESS +SIGNED_BY_B
    a_delivered --> complete: +COMPUTE_ACCESS +SIGNED_BY_B
    b_delivered --> complete: +API_ACCESS +SIGNED_BY_A
    // What about timeout? No temporal primitives yet.
  }
}
```

**Pain points:**
- No timeout/deadline syntax
- No way to express "within N blocks/commits"
- Symmetric trades are verbose

**Proposed improvement:**
```modality
// Symmetric trade primitive?
trade APITrade between A, B {
  A provides: +API_ACCESS
  B provides: +COMPUTE_ACCESS
  timeout: 10 commits
}
```

---

## Use Case 2: Multi-Agent Task Delegation

**Scenario:** Orchestrator agent delegates subtasks to workers, aggregates results.

**What needs expressing:**
- Orchestrator assigns task to Worker
- Worker must respond within deadline
- Orchestrator can reassign if Worker times out
- Final aggregation requires all subtasks complete

**Current syntax:**
```modality
model TaskDelegation {
  part task {
    pending --> assigned: +ASSIGN +SIGNED_BY_ORCHESTRATOR
    assigned --> completed: +RESULT +SIGNED_BY_WORKER
    assigned --> reassigned: +REASSIGN +SIGNED_BY_ORCHESTRATOR  // timeout case
    reassigned --> completed: +RESULT +SIGNED_BY_NEW_WORKER
  }
}
```

**Pain points:**
- Can't express "if no response in N, then..."
- No aggregation pattern (need all of X, Y, Z)
- Worker identity is stringly-typed

**Proposed improvement:**
```modality
delegation Task from Orchestrator to Worker {
  timeout: 5 commits → can REASSIGN
  on RESULT → complete
}
```

---

## Use Case 3: Reputation Staking

**Scenario:** Agent stakes reputation to guarantee behavior. Bad behavior = reputation loss.

**What needs expressing:**
- Agent deposits reputation stake
- Contract monitors behavior
- Good completion → stake returned
- Violation → stake slashed

**Current syntax:**
```modality
model ReputationStake {
  part staking {
    init --> staked: +STAKE +SIGNED_BY_AGENT
    staked --> active: +ACTIVATE
    active --> completed: +COMPLETE +SIGNED_BY_AGENT -VIOLATION
    active --> slashed: +VIOLATION
    completed --> returned: +RETURN_STAKE
  }
}

formula NoViolation { always [-VIOLATION] true }
```

**Pain points:**
- Stake amount isn't expressible (no values)
- Violation detection is external
- No way to link to reputation system

**Proposed improvement:**
```modality
staked_contract Task by Agent {
  stake: 100 reputation
  on COMPLETE → return stake
  on VIOLATION → slash stake
}
```

---

## Use Case 4: Information Escrow

**Scenario:** Agent A will reveal secret after Agent B pays. Neither can cheat.

**What needs expressing:**
- A commits hash of secret
- B pays
- A reveals secret (must match hash)
- If A doesn't reveal, B gets refund

**Current syntax:**
```modality
model InfoEscrow {
  part exchange {
    init --> committed: +COMMIT_HASH +SIGNED_BY_A
    committed --> paid: +PAYMENT +SIGNED_BY_B
    paid --> revealed: +REVEAL +SIGNED_BY_A  // needs hash verification
    paid --> refunded: +TIMEOUT +REFUND  // how to express timeout?
  }
}
```

**Pain points:**
- Hash verification needs predicates
- Timeout mechanism missing
- Atomicity not guaranteed

---

## Use Case 5: Collaborative Document Editing

**Scenario:** Multiple agents co-author a document with approval workflow.

**What needs expressing:**
- Any author can propose changes
- Changes need M-of-N approval
- Conflicts need resolution
- Final version needs consensus

**Current syntax:** Very verbose, requires many states.

**Pain points:**
- M-of-N voting is common but no primitive
- Conflict resolution logic is complex
- Version tracking not built-in

---

## Language Optimization Opportunities

### 1. Temporal Primitives
```modality
// Timeouts
within 10 commits { ... } else { ... }
after DEPOSIT → must DELIVER within 5
```

### 2. Symmetric Patterns
```modality
// Trade shorthand
swap A:+ITEM_A for B:+ITEM_B

// Mutual agreement
mutual(A, B) { -DEFECT }
```

### 3. Multi-Party Patterns
```modality
// M-of-N approval
approve(2 of [A, B, C]) to PUBLISH

// All-of
all(Workers) must COMPLETE before AGGREGATE
```

### 4. Value Binding
```modality
// Bind values for verification
let hash = commit(secret)
reveal(x) where hash(x) == hash
```

### 5. Role Abstraction
```modality
role Buyer { can: DEPOSIT, RELEASE }
role Seller { can: DELIVER }

escrow(Buyer, Seller) { ... }
```

---

## Priority Improvements

Based on use case analysis:

1. **Timeouts** - Almost every real contract needs them
2. **Symmetric trades** - Very common, currently verbose
3. **M-of-N approval** - Governance is everywhere
4. **Value binding** - Can't express hashes, amounts
5. **Role abstraction** - DRY principle for multi-contract systems

---

*Next: Pick one improvement and implement it in the grammar.*
