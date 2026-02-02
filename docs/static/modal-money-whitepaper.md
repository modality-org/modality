# Modal Money: A Network for Verifiable Contracts

## Abstract

Modal Money is a decentralized network for verifiable contracts, combining permissionless proof-of-work mining with fast-finality BFT consensus. Built on the Modality verification language, Modal Money enables agents and humans to create, negotiate, and execute modal contracts—append-only logs of signed commits governed by temporal logic formulas. This whitepaper presents the network architecture, consensus mechanism, contract execution model, and the role of formal verification in enabling trustless cooperation.

## 1. Introduction

The proliferation of AI agents creates an urgent need for trustless coordination mechanisms. Traditional contracts rely on legal enforcement and human judgment—neither scales to autonomous agents operating at machine speed. Modal Money addresses this by providing infrastructure where contracts are:

- **Verifiable**: Temporal logic formulas are checked automatically
- **Decentralized**: No central authority controls contract execution
- **Permissionless**: Anyone can participate through CPU-friendly mining
- **Fast**: BFT consensus provides second-scale finality

### 1.1 The Agent Cooperation Problem

When two agents want to cooperate—exchange resources, share data, coordinate actions—they face a fundamental trust problem. Neither can rely on the other to honor commitments without enforcement. Legal systems don't apply to agents. Reputation systems can be gamed.

Modal Money solves this through cryptographic commitment and formal verification. Agents express their requirements as modal formulas, and the network ensures these formulas are satisfied throughout the contract's lifecycle.

### 1.2 Design Goals

1. **Permissionless Entry**: No gatekeepers, no staking requirements
2. **Fast Finality**: Transactions confirm in seconds, not minutes
3. **Native Verification**: Temporal logic built into the protocol
4. **Agent-First Design**: Optimized for machine-to-machine contracts

## 2. Modal Contracts

A modal contract is an append-only log of signed commits, where each commit is validated against accumulated rules.

### 2.1 Contract Structure

```
Contract = {
  commits: [Commit],
  derived_state: State
}

Commit = {
  parent: Hash,
  action: Action,
  payload: Data,
  signatures: [Signature],
  timestamp: Time
}
```

Contracts start empty. Parties add commits that:
- Modify state (POST to paths)
- Add rules (RULE commits with formulas)
- Perform domain actions (ACTION commits)

### 2.2 Path-Based State

Contract state is organized as paths with typed values:

| Path Type | Description |
|-----------|-------------|
| `.bool` | Boolean flag |
| `.text` | Text content |
| `.json` | Structured data |
| `.id` | Identity (public key) |
| `.wasm` | Executable predicate |
| `.modality` | State machine model |

Example contract paths:
```
/parties/alice.id
/parties/bob.id
/escrow/amount.json
/escrow/released.bool
/rules/protection.modality
```

### 2.3 Rules and Formulas

Rules are temporal logic formulas that constrain future commits:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [<+ACTION>] (
        signed_by(/parties/alice.id) | signed_by(/parties/bob.id)
      )
    )
  }
}
```

This rule ensures every action must be signed by either Alice or Bob.

### 2.4 Modal Logic Semantics

Modal Money uses modal mu-calculus with multi-action labels:

| Operator | Meaning |
|----------|---------|
| `[+A] φ` | After any A transition, φ holds |
| `<+A> φ` | Some A transition leads to φ |
| `[<+A>] φ` | Committed: must do A and φ follows |
| `always(φ)` | φ holds in all future states |
| `eventually(φ)` | φ holds in some future state |
| `lfp(X, φ)` | Least fixed point (reachability) |
| `gfp(X, φ)` | Greatest fixed point (safety) |

## 3. Network Architecture

Modal Money operates as a two-layer system: PoW mining for block production and validator nomination, BFT consensus for fast finality.

```
┌─────────────────────────────────────────────────┐
│                 Applications                     │
│    (Agents, Wallets, Contract Interfaces)       │
└───────────────────────┬─────────────────────────┘
                        │
┌───────────────────────┴─────────────────────────┐
│              Modal Money Network                 │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │   PoW Mining    │  │   BFT Consensus     │   │
│  │  - RandomX      │  │  - Shoal Algorithm  │   │
│  │  - Nominations  │  │  - DAG Processing   │   │
│  └────────┬────────┘  └──────────┬──────────┘   │
│           │                      │              │
│  ┌────────┴──────────────────────┴──────────┐   │
│  │           Contract Execution              │   │
│  │  - WASM Runtime    - Modal Verification  │   │
│  │  - Gas Metering    - State Management    │   │
│  └───────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### 3.1 Proof-of-Work Layer

Mining uses RandomX, a CPU-friendly algorithm resistant to ASICs and GPUs:

- **Permissionless**: Anyone with a CPU can mine
- **Fair**: No advantage from specialized hardware
- **Secure**: Based on random code execution

Each mined block nominates a validator for the upcoming epoch:

```rust
BlockData {
  nominated_peer_id: String,
  miner_number: u64,
}
```

### 3.2 Validator Selection

Validators are selected from mining nominees using a deterministic shuffle:

1. Collect nominees from epoch blocks
2. Generate seed from XOR of block nonces
3. Fisher-Yates shuffle with seed
4. Top N nominees become validators

This ensures:
- Permissionless entry via mining
- Unpredictable selection (no grinding)
- Sybil resistance through PoW cost

### 3.3 BFT Consensus

The Shoal algorithm provides fast finality:

1. **DAG Construction**: Certificates form a directed acyclic graph
2. **Leader Selection**: Reputation-weighted with deterministic tiebreaking
3. **Commit Rule**: 2f+1 support in DAG triggers commit
4. **Fault Tolerance**: Survives f < n/3 Byzantine validators

Finality is achieved in seconds, enabling responsive contract execution.

## 4. Contract Execution

### 4.1 WASM Runtime

Dynamic contract logic executes in a sandboxed WASM environment:

```rust
executor.execute(wasm_bytes, method, args) -> Result<String>
```

Features:
- **Gas Metering**: Prevents infinite loops and DoS
- **Memory Isolation**: Contracts can't access host memory
- **Deterministic**: Same inputs always produce same outputs

### 4.2 Standard Predicates

Built-in predicates for common operations:

| Predicate | Description |
|-----------|-------------|
| `signed_by(id)` | Verify ed25519 signature |
| `threshold(n, [ids])` | n-of-m multisig |
| `before(time)` | Timestamp constraint |
| `after(time)` | Timestamp constraint |
| `oracle_attests(id, stmt)` | External oracle attestation |

### 4.3 Model Checking

Each commit is validated against accumulated rules:

1. Parse commit action and payload
2. Update derived state
3. Check all rule formulas against new state
4. Reject if any formula fails

This provides continuous verification throughout contract lifecycle.

## 5. Contract Patterns

### 5.1 Escrow

```modality
model escrow {
  states { idle, funded, complete, refunded }
  initial { idle }
  transitions {
    idle -[DEPOSIT]-> funded
    funded -[RELEASE]-> complete
    funded -[REFUND]-> refunded
  }
}
```

Rules ensure:
- Only buyer can deposit
- Only seller can release (after delivery)
- Refund requires timeout or dispute resolution

### 5.2 Atomic Swap

Two-phase commit with hash locks:

1. Alice commits hash(secret) with locked funds
2. Bob commits matching hash with his funds
3. Alice reveals secret, claims Bob's funds
4. Bob uses revealed secret, claims Alice's funds

### 5.3 Multisig Treasury

```modality
rule {
  formula {
    always (
      [<+WITHDRAW>] threshold(3, [
        /signers/alice.id,
        /signers/bob.id,
        /signers/carol.id,
        /signers/dave.id,
        /signers/eve.id
      ])
    )
  }
}
```

3-of-5 approval required for any withdrawal.

### 5.4 Agent Swarm

Coordinator distributes tasks to workers:

```modality
model swarm {
  states { idle, assigned, working, complete, failed }
  transitions {
    idle -[ASSIGN]-> assigned
    assigned -[ACCEPT]-> working
    assigned -[REJECT]-> idle
    working -[SUBMIT]-> complete
    working -[TIMEOUT]-> failed
  }
}
```

## 6. Security Analysis

### 6.1 Consensus Security

- **PoW**: Cumulative difficulty prevents reorgs
- **BFT**: Tolerates f < n/3 Byzantine validators
- **Hybrid**: Mining prevents validator capture

### 6.2 Contract Security

- **Verification**: All commits checked against rules
- **Isolation**: WASM sandbox prevents escape
- **Gas Limits**: Resource exhaustion prevented

### 6.3 Economic Security

- **Mining Cost**: Sybil resistance through PoW
- **No Staking**: No plutocratic advantages
- **Fair Selection**: Unpredictable validator shuffle

## 7. Performance

| Metric | Target |
|--------|--------|
| Block Time | 10 seconds |
| Finality | < 3 seconds |
| Throughput | 1000+ TPS |
| Contract Verify | < 10ms |

## 8. Use Cases

### 8.1 AI Agent Coordination

Agents negotiate cooperation through modal contracts:
- Data exchange with privacy guarantees
- Service agreements with SLAs
- Resource sharing with fair allocation

### 8.2 Decentralized Finance

Verified financial instruments:
- Escrow without counterparty risk
- Lending with collateral rules
- Trading with atomic settlement

### 8.3 Supply Chain

Temporal guarantees for logistics:
- Delivery deadlines as formulas
- Custody transfer verification
- Quality attestation chains

### 8.4 Digital Identity

Self-sovereign identity with:
- Revocable credentials
- Threshold recovery
- Privacy-preserving attestations

## 9. Roadmap

### Phase 1: Foundation (Current)
- Modality language and verification
- Contract log implementation
- CLI tooling

### Phase 2: Network
- PoW mining implementation
- Validator consensus
- P2P networking

### Phase 3: Mainnet
- Security audits
- Economic parameters
- Public launch

## 10. Conclusion

Modal Money provides infrastructure for the agent economy—a network where cooperation is verified through temporal logic, not trusted through reputation. By combining permissionless PoW entry, fast BFT finality, and native modal verification, Modal Money enables trustless coordination at machine speed.

The future of autonomous agents requires contracts that verify themselves. Modal Money makes this possible.

---

## References

1. Nakamoto, S. (2008). Bitcoin: A Peer-to-Peer Electronic Cash System
2. Castro, M. & Liskov, B. (1999). Practical Byzantine Fault Tolerance
3. Haber, S. & Stornetta, W.S. (1991). How to Time-Stamp a Digital Document
4. Buterin, V. (2014). Ethereum White Paper
5. Pnueli, A. (1977). The Temporal Logic of Programs
6. RandomX Documentation. https://github.com/tevador/RandomX
7. Danezis, G. et al. (2022). Narwhal and Tusk
8. Shoal Algorithm Specification
