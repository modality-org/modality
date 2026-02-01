# Escrow: 3-Party Hub Interaction

Three agents (Buyer, Seller, Arbiter) use a hub to execute an escrow contract.

## Parties

| Party | Role | Identity |
|-------|------|----------|
| Alice | Buyer | `id_alice_abc` |
| Bob | Seller | `id_bob_xyz` |
| Carol | Arbiter | `id_carol_123` |

## Setup Phase

### 1. Bob (Seller) creates the contract

```bash
# Bob starts the hub and registers
modal hub start --detach
modal hub register --output bob-creds.json

# Bob creates the escrow contract
modal hub create "Widget Sale Escrow" --creds bob-creds.json
# → contract_id: con_escrow_001
```

### 2. Bob pushes the model and his identity

```bash
# Create contract directory
mkdir escrow && cd escrow
modal c create --contract-id con_escrow_001

# Add parties
echo 'ed25519:bob_pubkey_here' > state/parties/seller.id

# Add the escrow model
cat > rules/escrow.modality << 'EOF'
model Escrow {
  state init, deposited, delivered, disputed, released, refunded
  
  init -> deposited : DEPOSIT [+signed_by(/parties/buyer.id)]
  deposited -> delivered : DELIVER [+signed_by(/parties/seller.id)]
  delivered -> released : RELEASE [+signed_by(/parties/buyer.id)]
  delivered -> disputed : DISPUTE [+signed_by(/parties/buyer.id)]
  disputed -> refunded : REFUND [+signed_by(/parties/arbiter.id)]
  disputed -> released : RELEASE [+signed_by(/parties/arbiter.id)]
  
  released -> released
  refunded -> refunded
}
EOF

# Commit and push
modal c commit --all -m "Initial escrow setup by seller"
modal c remote add hub http://localhost:3100
modal c push --remote hub
```

### 3. Bob invites Alice (Buyer) and Carol (Arbiter)

```bash
# Alice and Carol register with the hub
# Alice: modal hub register --output alice-creds.json
# Carol: modal hub register --output carol-creds.json

# Bob grants write access
modal hub grant con_escrow_001 id_alice_abc write --creds bob-creds.json
modal hub grant con_escrow_001 id_carol_123 write --creds bob-creds.json
```

### 4. Alice joins and adds her identity

```bash
# Alice pulls the contract
mkdir alice-escrow && cd alice-escrow
modal c create --contract-id con_escrow_001
modal c remote add hub http://localhost:3100
modal c pull --remote hub

# Alice adds her identity
echo 'ed25519:alice_pubkey_here' > state/parties/buyer.id
modal c commit --all -m "Buyer joins"
modal c push --remote hub
```

### 5. Carol joins and adds her identity

```bash
# Carol pulls and adds her identity
mkdir carol-escrow && cd carol-escrow
modal c create --contract-id con_escrow_001
modal c remote add hub http://localhost:3100
modal c pull --remote hub

echo 'ed25519:carol_pubkey_here' > state/parties/arbiter.id
modal c commit --all -m "Arbiter joins"
modal c push --remote hub
```

## Execution Phase

### 6. Alice deposits (ACTION commit)

```bash
# Alice pulls latest state
cd alice-escrow
modal c pull --remote hub

# Alice creates a signed DEPOSIT action
cat > commit-deposit.json << 'EOF'
{
  "method": "ACTION",
  "action": "DEPOSIT",
  "data": {
    "amount": "100 USDC",
    "tx_hash": "0xabc123..."
  }
}
EOF

# Sign and push
modal c commit --action commit-deposit.json --sign alice.passfile -m "Deposit funds"
modal c push --remote hub
# Hub validates: DEPOSIT allowed from init, signed by buyer ✓
```

### 7. Bob delivers

```bash
cd bob-escrow
modal c pull --remote hub
# Sees Alice's deposit

cat > commit-deliver.json << 'EOF'
{
  "method": "ACTION",
  "action": "DELIVER",
  "data": {
    "tracking": "FEDEX-123456",
    "delivered_at": "2026-02-01T15:00:00Z"
  }
}
EOF

modal c commit --action commit-deliver.json --sign bob.passfile -m "Package delivered"
modal c push --remote hub
# Hub validates: DELIVER allowed from deposited, signed by seller ✓
```

### 8a. Happy Path: Alice releases

```bash
cd alice-escrow
modal c pull --remote hub

cat > commit-release.json << 'EOF'
{
  "method": "ACTION",
  "action": "RELEASE",
  "data": {
    "rating": 5,
    "comment": "Great seller!"
  }
}
EOF

modal c commit --action commit-release.json --sign alice.passfile -m "Release funds"
modal c push --remote hub
# Hub validates: RELEASE allowed from delivered, signed by buyer ✓
# Contract complete!
```

### 8b. Dispute Path: Alice disputes

```bash
cd alice-escrow
modal c pull --remote hub

cat > commit-dispute.json << 'EOF'
{
  "method": "ACTION",
  "action": "DISPUTE",
  "data": {
    "reason": "Item not as described",
    "evidence": ["photo1.jpg", "photo2.jpg"]
  }
}
EOF

modal c commit --action commit-dispute.json --sign alice.passfile -m "Dispute: item defective"
modal c push --remote hub
# Hub validates: DISPUTE allowed from delivered, signed by buyer ✓
```

### 9. Carol (Arbiter) resolves dispute

```bash
cd carol-escrow
modal c pull --remote hub
# Carol reviews evidence from Alice and Bob

# Carol decides in favor of Alice
cat > commit-refund.json << 'EOF'
{
  "method": "ACTION",
  "action": "REFUND",
  "data": {
    "ruling": "Item significantly different from listing",
    "refund_amount": "100 USDC"
  }
}
EOF

modal c commit --action commit-refund.json --sign carol.passfile -m "Arbiter: refund buyer"
modal c push --remote hub
# Hub validates: REFUND allowed from disputed, signed by arbiter ✓
# Contract complete!
```

## Final State

```bash
modal c pull --remote hub
modal c log

# Commit history:
# 1. init_seller - Bob adds model
# 2. init_buyer - Alice joins
# 3. init_arbiter - Carol joins
# 4. action_deposit - Alice deposits
# 5. action_deliver - Bob delivers
# 6. action_dispute - Alice disputes
# 7. action_refund - Carol rules
```

## Validation Examples

### Invalid: Bob tries to release (wrong signer)

```bash
cd bob-escrow
modal c commit --action '{"method":"ACTION","action":"RELEASE"}' --sign bob.passfile
modal c push --remote hub
# ❌ Error: "Must be signed by /parties/buyer.id"
```

### Invalid: Alice tries to release before delivery

```bash
# If state is still "deposited"
modal c commit --action '{"method":"ACTION","action":"RELEASE"}' --sign alice.passfile
modal c push --remote hub
# ❌ Error: "Action 'RELEASE' not allowed from state 'deposited'"
```

### Invalid: Anyone tries to act after completion

```bash
# After contract is in "released" or "refunded" state
modal c commit --action '{"method":"ACTION","action":"DISPUTE"}' --sign alice.passfile
modal c push --remote hub
# ❌ Error: "Action 'DISPUTE' not allowed from state 'released'"
```
