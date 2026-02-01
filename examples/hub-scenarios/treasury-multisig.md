# Treasury: 5-Party Multisig Hub Interaction

Five board members manage a DAO treasury requiring 3-of-5 approval.

## Parties

| Party | Role | Identity |
|-------|------|----------|
| Alice | Board Member | `id_alice` |
| Bob | Board Member | `id_bob` |
| Carol | Board Member | `id_carol` |
| Dave | Board Member | `id_dave` |
| Eve | Board Member | `id_eve` |

## Setup Phase

### 1. Alice creates the treasury contract

```bash
modal hub register --output alice-creds.json
modal hub create "DAO Treasury" --creds alice-creds.json
# → contract_id: con_treasury_001

mkdir treasury && cd treasury
modal c create --contract-id con_treasury_001

# Add the multisig model
cat > rules/treasury.modality << 'EOF'
model Treasury {
  state idle, pending, executed, cancelled
  
  // Anyone can propose
  idle -> pending : PROPOSE [+signed_by(/members/*)]
  
  // Need 3-of-5 to approve
  pending -> pending : APPROVE [+signed_by(/members/*)]
  pending -> executed : EXECUTE [+threshold(3, /approvals, /members)]
  pending -> cancelled : CANCEL [+threshold(3, /rejections, /members)]
  
  // Reset for next proposal
  executed -> idle : RESET
  cancelled -> idle : RESET
  
  idle -> idle
}
EOF

# Add all board members
mkdir -p state/members
echo 'ed25519:alice_key' > state/members/alice.id
echo 'ed25519:bob_key' > state/members/bob.id
echo 'ed25519:carol_key' > state/members/carol.id
echo 'ed25519:dave_key' > state/members/dave.id
echo 'ed25519:eve_key' > state/members/eve.id

modal c commit --all -m "Initialize treasury with 5 board members"
modal c remote add hub http://localhost:3100
modal c push --remote hub

# Grant access to all members
modal hub grant con_treasury_001 id_bob write
modal hub grant con_treasury_001 id_carol write
modal hub grant con_treasury_001 id_dave write
modal hub grant con_treasury_001 id_eve write
```

### 2. All members clone the contract

```bash
# Bob
mkdir bob-treasury && cd bob-treasury
modal c create --contract-id con_treasury_001
modal c remote add hub http://localhost:3100
modal c pull --remote hub

# Carol, Dave, Eve do the same...
```

## Proposal Flow

### 3. Bob proposes a payment

```bash
cd bob-treasury
modal c pull --remote hub

cat > state/proposals/prop_001.json << 'EOF'
{
  "id": "prop_001",
  "type": "payment",
  "recipient": "0x1234...",
  "amount": "50000 USDC",
  "description": "Q1 developer grants",
  "proposed_by": "bob",
  "proposed_at": "2026-02-01T16:00:00Z"
}
EOF

# Initialize approval tracking
mkdir -p state/approvals state/rejections
echo '[]' > state/approvals/prop_001.json
echo '[]' > state/rejections/prop_001.json

modal c commit --all -m "Propose: Q1 developer grants (50k USDC)"

# Push the ACTION
cat > action.json << 'EOF'
{
  "method": "ACTION",
  "action": "PROPOSE",
  "data": { "proposal_id": "prop_001" }
}
EOF
modal c commit --action action.json --sign bob.passfile -m "ACTION: PROPOSE prop_001"
modal c push --remote hub
# State: idle -> pending
```

### 4. Alice approves

```bash
cd alice-treasury
modal c pull --remote hub

# Add approval
cat state/approvals/prop_001.json | jq '. + ["alice"]' > tmp && mv tmp state/approvals/prop_001.json

cat > action.json << 'EOF'
{
  "method": "ACTION",
  "action": "APPROVE",
  "data": { "proposal_id": "prop_001", "member": "alice" }
}
EOF
modal c commit --action action.json --sign alice.passfile -m "Alice approves prop_001"
modal c push --remote hub
# State: pending (1/3 approvals)
```

### 5. Carol approves

```bash
cd carol-treasury
modal c pull --remote hub

cat state/approvals/prop_001.json | jq '. + ["carol"]' > tmp && mv tmp state/approvals/prop_001.json

modal c commit --action '{"method":"ACTION","action":"APPROVE","data":{"proposal_id":"prop_001","member":"carol"}}' \
  --sign carol.passfile -m "Carol approves prop_001"
modal c push --remote hub
# State: pending (2/3 approvals)
```

### 6. Dave approves - threshold reached!

```bash
cd dave-treasury
modal c pull --remote hub

cat state/approvals/prop_001.json | jq '. + ["dave"]' > tmp && mv tmp state/approvals/prop_001.json

modal c commit --action '{"method":"ACTION","action":"APPROVE","data":{"proposal_id":"prop_001","member":"dave"}}' \
  --sign dave.passfile -m "Dave approves prop_001"
modal c push --remote hub
# State: pending (3/3 approvals - threshold reached!)
```

### 7. Anyone can now execute

```bash
cd alice-treasury
modal c pull --remote hub

# Execute the payment
cat > state/executions/prop_001.json << 'EOF'
{
  "proposal_id": "prop_001",
  "executed_at": "2026-02-01T18:00:00Z",
  "tx_hash": "0xdef456..."
}
EOF

modal c commit --action '{"method":"ACTION","action":"EXECUTE","data":{"proposal_id":"prop_001"}}' \
  --sign alice.passfile -m "Execute prop_001"
modal c push --remote hub
# Hub validates: threshold(3, /approvals, /members) ✓
# State: pending -> executed
```

## Rejection Flow (Alternative)

### If Carol, Dave, Eve reject instead:

```bash
# Carol rejects
cd carol-treasury
cat state/rejections/prop_001.json | jq '. + ["carol"]' > tmp && mv tmp state/rejections/prop_001.json
modal c commit --action '{"method":"ACTION","action":"REJECT","data":{"proposal_id":"prop_001"}}' \
  --sign carol.passfile -m "Carol rejects"
modal c push --remote hub
# 1/3 rejections

# Dave rejects
# 2/3 rejections

# Eve rejects
# 3/3 rejections - can now cancel

modal c commit --action '{"method":"ACTION","action":"CANCEL","data":{"proposal_id":"prop_001"}}' \
  --sign eve.passfile -m "Cancel rejected proposal"
modal c push --remote hub
# State: pending -> cancelled
```

## Validation Examples

### Invalid: Duplicate approval

```bash
# Alice already approved
cd alice-treasury
modal c commit --action '{"method":"ACTION","action":"APPROVE"}' --sign alice.passfile
modal c push --remote hub
# ❌ Error: "Duplicate signer in approvals"
```

### Invalid: Execute without threshold

```bash
# Only 2 approvals
modal c commit --action '{"method":"ACTION","action":"EXECUTE"}' --sign alice.passfile
modal c push --remote hub
# ❌ Error: "threshold(3, /approvals, /members) not satisfied"
```

### Invalid: Non-member tries to approve

```bash
# Frank is not a member
modal c commit --action '{"method":"ACTION","action":"APPROVE"}' --sign frank.passfile
modal c push --remote hub
# ❌ Error: "Must be signed by /members/*"
```

## Final Contract Log

```
Commits:
1. setup          - Initialize with 5 members
2. propose        - Bob proposes payment
3. approve_alice  - Alice approves (1/3)
4. approve_carol  - Carol approves (2/3)
5. approve_dave   - Dave approves (3/3) 
6. execute        - Alice executes

State transitions: idle -> pending -> executed
```
