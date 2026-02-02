# Proposal Commits Design

**Status:** Draft  
**Author:** Gerold Steiner  
**Date:** 2026-02-01

## Problem

Some commits need multiple signatures before they become valid. Currently, commits are atomic (sign → push → done). We need a mechanism for commits that require N-of-M signatures to finalize.

## Use Cases

1. **Treasury Multisig** - 3-of-5 board members must approve a payment
2. **Contract Amendments** - Both parties must sign to change terms
3. **High-Value Transfers** - Large amounts require additional approvers
4. **Governance Decisions** - Quorum needed for protocol changes

## Design

### Commit Types

```
PROPOSE  - Create a pending proposal with payload
APPROVE  - Add signature to an existing proposal
CANCEL   - Cancel a proposal (by proposer, or via counter-threshold)
```

### Proposal Structure

```json
{
  "method": "PROPOSE",
  "proposal": {
    "id": "prop_001",
    "payload": {
      "method": "ACTION",
      "action": "TRANSFER",
      "data": {"recipient": "0x...", "amount": 50000}
    },
    "threshold": {
      "required": 3,
      "signers": [
        "/members/alice.id",
        "/members/bob.id", 
        "/members/carol.id",
        "/members/dave.id",
        "/members/eve.id"
      ]
    },
    "expires_at": "2026-02-05T00:00:00Z"
  }
}
```

### Approval Structure

```json
{
  "method": "APPROVE",
  "proposal_id": "prop_001"
}
// Signature in commit header proves identity
```

### Cancel Structure

```json
{
  "method": "CANCEL",
  "proposal_id": "prop_001",
  "reason": "Changed requirements"
}
// Only proposer can cancel, or counter-threshold
```

## State Machine

```
                    ┌──────────────┐
                    │   (start)    │
                    └──────┬───────┘
                           │ PROPOSE
                           ▼
                    ┌──────────────┐
          ┌─────────│   pending    │─────────┐
          │         └──────┬───────┘         │
          │                │                 │
    CANCEL│         APPROVE│           EXPIRE│
          │                │                 │
          ▼                ▼                 ▼
   ┌──────────┐    ┌──────────────┐   ┌──────────┐
   │ cancelled│    │  threshold   │   │ expired  │
   └──────────┘    │    met?      │   └──────────┘
                   └──────┬───────┘
                          │ yes
                          ▼
                   ┌──────────────┐
                   │  finalized   │
                   │ (payload     │
                   │  applied)    │
                   └──────────────┘
```

## Storage

### Proposal Namespace

Proposals stored at `/.proposals/<proposal_id>/`:

```
/.proposals/prop_001/
  payload.json     - The proposed commit payload
  meta.json        - threshold, expires_at, proposed_by, proposed_at
  approvals/
    alice.sig      - Alice's signature
    bob.sig        - Bob's signature
  status           - "pending" | "finalized" | "cancelled" | "expired"
```

### Tracking Current Proposals

```json
// /.proposals/index.json
{
  "pending": ["prop_001", "prop_002"],
  "finalized": ["prop_000"],
  "cancelled": [],
  "expired": []
}
```

## Validation Flow

### On PROPOSE

1. Validate proposer is in `threshold.signers`
2. Validate payload structure
3. **Pre-validate payload against model** (fail fast)
4. Generate proposal ID
5. Store proposal in pending state
6. Add proposer's signature as first approval

### On APPROVE

1. Validate proposal exists and is pending
2. Validate signer is in `threshold.signers`
3. Validate signer hasn't already approved
4. Add signature to approvals
5. Check if threshold met:
   - If yes: **Finalize** (apply payload, update contract state)
   - If no: Remain pending

### On CANCEL

1. Validate proposal exists and is pending
2. Validate canceller is proposer (or counter-threshold met)
3. Mark proposal as cancelled

### On Heartbeat/Expiry Check

1. For each pending proposal:
   - If `expires_at` passed: mark as expired

## Finalization

When threshold is met:

1. Re-validate payload against current contract state
2. Apply payload as if it were a normal commit
3. Mark proposal as finalized
4. Emit finalization event with:
   - Original proposal ID
   - All signers who approved
   - Final commit hash

## Model Integration

### Option A: Explicit States

```modality
model Treasury {
  state idle, proposed, executed
  
  idle -> proposed : PROPOSE [+signed_by(/members/*)]
  proposed -> proposed : APPROVE [+signed_by(/members/*)]
  proposed -> executed : FINALIZE [+proposal_threshold_met(3)]
  proposed -> idle : CANCEL [+signed_by(/proposer)]
  proposed -> idle : EXPIRE [+proposal_expired]
}
```

### Option B: Implicit Finalization

The final APPROVE that meets threshold auto-applies the payload:

```modality
model Treasury {
  state idle, pending
  
  // PROPOSE creates pending proposal
  idle -> pending : PROPOSE [+signed_by(/members/*)]
  
  // APPROVE either:
  // - Stays pending (threshold not met)
  // - Transitions to payload's target state (threshold met)
  pending -> pending : APPROVE [+signed_by(/members/*) -threshold_met]
  pending -> * : APPROVE [+signed_by(/members/*) +threshold_met]
}
```

The `*` indicates the transition target is determined by the proposal payload.

### Option C: Proposal as Wrapper

Proposals wrap any commit type:

```modality
// Normal commit:
ACTION TRANSFER {...}

// Proposal-wrapped commit:
PROPOSE {
  threshold: 3,
  signers: [...],
  payload: ACTION TRANSFER {...}
}
```

The model validates the payload, not the proposal wrapper.

## CLI Integration

```bash
# Create a proposal
modal c propose --threshold 3 \
  --signers "/members/alice.id,/members/bob.id,..." \
  --action '{"method":"ACTION","action":"TRANSFER",...}' \
  --expires "2026-02-05"

# List pending proposals
modal c proposals

# Approve a proposal
modal c approve prop_001 --sign me.passfile

# Cancel a proposal
modal c cancel prop_001 --sign me.passfile

# Check proposal status
modal c proposal prop_001
```

## Hub Implementation

### New Endpoints

```
POST /contracts/:id/propose    - Create proposal
POST /contracts/:id/approve    - Approve proposal  
POST /contracts/:id/cancel     - Cancel proposal
GET  /contracts/:id/proposals  - List proposals
GET  /contracts/:id/proposals/:propId - Get proposal details
```

### Database Schema

```sql
CREATE TABLE proposals (
  id TEXT PRIMARY KEY,
  contract_id TEXT NOT NULL,
  payload TEXT NOT NULL,
  threshold_required INTEGER NOT NULL,
  threshold_signers TEXT NOT NULL,  -- JSON array
  proposed_by TEXT NOT NULL,
  proposed_at INTEGER NOT NULL,
  expires_at INTEGER,
  status TEXT NOT NULL,  -- pending, finalized, cancelled, expired
  finalized_at INTEGER,
  finalized_commit TEXT
);

CREATE TABLE proposal_approvals (
  proposal_id TEXT NOT NULL,
  signer TEXT NOT NULL,
  signature TEXT NOT NULL,
  approved_at INTEGER NOT NULL,
  PRIMARY KEY (proposal_id, signer)
);
```

## Open Questions

1. **Can proposal payload be modified?** Or must you cancel and re-propose?
2. **Parallel proposals?** Multiple pending proposals for same action?
3. **Ordering?** If two proposals finalize, which applies first?
4. **Delegation?** Can a signer delegate approval authority?
5. **Time-locks?** Minimum time before finalization?

## Security Considerations

- Replay protection: Proposal IDs must be unique
- Signature binding: Signatures must commit to full payload hash
- Expiration: Unbounded pending proposals are a risk
- State races: Concurrent approvals need atomic threshold check
