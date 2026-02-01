# Service Agreement: Client-Provider Hub Interaction

A client hires a provider for ongoing work with milestone payments.

## Parties

| Party | Role | Identity |
|-------|------|----------|
| Acme Corp | Client | `id_acme` |
| DevShop | Provider | `id_devshop` |

## Contract Model

```modality
model ServiceAgreement {
  state negotiating, active, milestone_pending, milestone_paid, 
        paused, completed, terminated
  
  // Setup
  negotiating -> active : ACTIVATE [+signed_by(/parties/client.id) +signed_by(/parties/provider.id)]
  
  // Milestone cycle
  active -> milestone_pending : SUBMIT_MILESTONE [+signed_by(/parties/provider.id)]
  milestone_pending -> milestone_paid : APPROVE_MILESTONE [+signed_by(/parties/client.id)]
  milestone_pending -> active : REJECT_MILESTONE [+signed_by(/parties/client.id)]
  milestone_paid -> active : CONTINUE [+signed_by(/parties/provider.id)]
  milestone_paid -> completed : FINALIZE [+signed_by(/parties/client.id) +signed_by(/parties/provider.id)]
  
  // Pause/resume
  active -> paused : PAUSE [+signed_by(/parties/client.id)]
  paused -> active : RESUME [+signed_by(/parties/client.id)]
  
  // Termination (either party)
  active -> terminated : TERMINATE [+signed_by(/parties/client.id) | +signed_by(/parties/provider.id)]
  paused -> terminated : TERMINATE [+signed_by(/parties/client.id) | +signed_by(/parties/provider.id)]
  
  completed -> completed
  terminated -> terminated
}
```

## Interaction Flow

### 1. DevShop creates the contract

```bash
modal hub register --output devshop-creds.json
modal hub create "Acme Website Rebuild" --creds devshop-creds.json
# → con_service_001

mkdir service && cd service
modal c create --contract-id con_service_001

# Add model
cat > rules/service.modality << 'EOF'
model ServiceAgreement {
  state negotiating, active, milestone_pending, milestone_paid, completed, terminated
  
  negotiating -> active : ACTIVATE [+signed_by(/parties/client.id) +signed_by(/parties/provider.id)]
  active -> milestone_pending : SUBMIT_MILESTONE [+signed_by(/parties/provider.id)]
  milestone_pending -> milestone_paid : APPROVE_MILESTONE [+signed_by(/parties/client.id)]
  milestone_pending -> active : REJECT_MILESTONE [+signed_by(/parties/client.id)]
  milestone_paid -> active : CONTINUE [+signed_by(/parties/provider.id)]
  milestone_paid -> completed : FINALIZE [+signed_by(/parties/client.id) +signed_by(/parties/provider.id)]
  active -> terminated : TERMINATE [+signed_by(/parties/*)]
  
  completed -> completed
  terminated -> terminated
}
EOF

# Add agreement terms
cat > state/agreement.json << 'EOF'
{
  "title": "Acme Website Rebuild",
  "total_value": "50000 USDC",
  "milestones": [
    { "id": "m1", "description": "Design mockups", "value": "10000" },
    { "id": "m2", "description": "Frontend development", "value": "20000" },
    { "id": "m3", "description": "Backend + integration", "value": "15000" },
    { "id": "m4", "description": "Testing + launch", "value": "5000" }
  ],
  "start_date": "2026-02-01",
  "deadline": "2026-05-01"
}
EOF

# DevShop identity
mkdir -p state/parties
echo 'ed25519:devshop_key' > state/parties/provider.id

modal c commit --all -m "Draft service agreement"
modal c remote add hub http://localhost:3100
modal c push --remote hub

# Grant Acme write access
modal hub grant con_service_001 id_acme write --creds devshop-creds.json
```

### 2. Acme reviews and co-signs

```bash
mkdir acme-service && cd acme-service
modal c create --contract-id con_service_001
modal c remote add hub http://localhost:3100
modal c pull --remote hub

# Acme adds their identity
echo 'ed25519:acme_key' > state/parties/client.id
modal c commit --all -m "Acme joins as client"

# Both parties sign ACTIVATE
# DevShop signs first, Acme countersigns
modal c commit --action '{"method":"ACTION","action":"ACTIVATE","data":{"effective_date":"2026-02-01"}}' \
  --sign acme.passfile -m "Acme activates agreement"
modal c push --remote hub
# State: negotiating -> active
```

### 3. DevShop submits Milestone 1

```bash
cd devshop-service
modal c pull --remote hub

cat > state/milestones/m1_submission.json << 'EOF'
{
  "milestone_id": "m1",
  "submitted_at": "2026-02-15T10:00:00Z",
  "deliverables": [
    { "name": "Homepage mockup", "url": "figma.com/..." },
    { "name": "Product page mockup", "url": "figma.com/..." },
    { "name": "Mobile designs", "url": "figma.com/..." }
  ],
  "notes": "Ready for review"
}
EOF

modal c commit --action '{"method":"ACTION","action":"SUBMIT_MILESTONE","data":{"milestone_id":"m1"}}' \
  --sign devshop.passfile -m "Submit milestone 1: Design mockups"
modal c push --remote hub
# State: active -> milestone_pending
```

### 4. Acme reviews and approves

```bash
cd acme-service
modal c pull --remote hub

# Review the deliverables...
cat state/milestones/m1_submission.json

# Approve and pay
cat > state/payments/m1_payment.json << 'EOF'
{
  "milestone_id": "m1",
  "amount": "10000 USDC",
  "paid_at": "2026-02-16T14:00:00Z",
  "tx_hash": "0xpay001..."
}
EOF

modal c commit --action '{"method":"ACTION","action":"APPROVE_MILESTONE","data":{"milestone_id":"m1","tx_hash":"0xpay001"}}' \
  --sign acme.passfile -m "Approve milestone 1, payment sent"
modal c push --remote hub
# State: milestone_pending -> milestone_paid
```

### 5. DevShop continues to next milestone

```bash
cd devshop-service
modal c pull --remote hub

modal c commit --action '{"method":"ACTION","action":"CONTINUE","data":{"next_milestone":"m2"}}' \
  --sign devshop.passfile -m "Continue to milestone 2"
modal c push --remote hub
# State: milestone_paid -> active
```

### 6. Repeat for milestones 2, 3, 4...

```bash
# M2: Frontend
modal c commit --action '{"method":"ACTION","action":"SUBMIT_MILESTONE","data":{"milestone_id":"m2"}}' ...
modal c commit --action '{"method":"ACTION","action":"APPROVE_MILESTONE","data":{"milestone_id":"m2"}}' ...

# M3: Backend
# M4: Launch
```

### 7. Final milestone - both sign to complete

```bash
# After M4 is approved and paid...
cd acme-service
modal c pull --remote hub

# Acme signs completion
modal c commit --action '{"method":"ACTION","action":"FINALIZE","data":{"completed_at":"2026-04-20"}}' \
  --sign acme.passfile -m "Acme confirms completion"
modal c push --remote hub

# DevShop countersigns
cd devshop-service
modal c pull --remote hub
modal c commit --action '{"method":"ACTION","action":"FINALIZE"}' \
  --sign devshop.passfile -m "DevShop confirms completion"
modal c push --remote hub
# State: milestone_paid -> completed
# Contract complete! 🎉
```

## Alternative: Rejection Flow

```bash
# Acme is not satisfied with milestone 2 submission
cd acme-service
modal c pull --remote hub

cat > state/milestones/m2_feedback.json << 'EOF'
{
  "milestone_id": "m2",
  "feedback": "Mobile responsive issues on product page",
  "required_changes": ["Fix navbar collapse", "Adjust image sizing"]
}
EOF

modal c commit --action '{"method":"ACTION","action":"REJECT_MILESTONE","data":{"milestone_id":"m2","reason":"Mobile issues"}}' \
  --sign acme.passfile -m "Reject M2: needs mobile fixes"
modal c push --remote hub
# State: milestone_pending -> active

# DevShop fixes and resubmits...
```

## Alternative: Termination

```bash
# Either party can terminate from active state
modal c commit --action '{"method":"ACTION","action":"TERMINATE","data":{"reason":"Budget cuts"}}' \
  --sign acme.passfile -m "Acme terminates contract"
modal c push --remote hub
# State: active -> terminated
```

## Contract Log

```
1. draft        - DevShop drafts agreement
2. join_client  - Acme joins
3. activate     - Both sign, contract active
4. submit_m1    - DevShop submits design
5. approve_m1   - Acme approves, pays 10k
6. continue     - Move to M2
7. submit_m2    - DevShop submits frontend
8. reject_m2    - Acme requests changes
9. submit_m2v2  - DevShop resubmits
10. approve_m2  - Acme approves, pays 20k
... (M3, M4)
15. finalize    - Both sign completion

Total paid: 50,000 USDC
Duration: 78 days
```
