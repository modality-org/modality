---
sidebar_position: 3
title: Oracle Escrow
---

# Building an Oracle-Verified Escrow

Learn to create an escrow contract with external delivery verification using the `oracle_attests` predicate.

## What We're Building

An escrow where:
- Buyer deposits funds
- Seller ships goods
- Trusted oracle confirms delivery
- Funds release only after oracle verification
- Timeout protects buyer if oracle fails

## Step 1: Create Identities

```bash
# Create participant identities
modal id create --name buyer
modal id create --name seller
modal id create --name delivery_oracle
```

## Step 2: Create the Contract

```bash
mkdir escrow && cd escrow
modal contract create
modal c checkout
```

## Step 3: Set Up State

```bash
# Add identities
modal c set-named-id /users/buyer.id --named buyer
modal c set-named-id /users/seller.id --named seller
modal c set-named-id /oracles/delivery.id --named delivery_oracle

# Set escrow terms
mkdir -p state/escrow
echo '{"price": 100, "currency": "USDC"}' > state/escrow/terms.json

# Set timeout deadline
echo "2026-03-01T00:00:00Z" > state/escrow/timeout.datetime
```

## Step 4: Define the Rules

Create `rules/escrow-auth.modality`:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // All commits must be from a known party or oracle
    signed_by(/users/buyer.id) | signed_by(/users/seller.id) | signed_by(/oracles/delivery.id)
  }
}
```

Create `rules/escrow-flow.modality` — ordering constraints:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // Release requires prior delivery attestation
    always([+RELEASE] implies <+oracle_attests(/oracles/delivery.id, "delivered", "true")> true)
  }
}
```

## Step 5: Synthesize the Model

Use the escrow template as a starting point:

```bash
modality model synthesize --template escrow --party-a buyer --party-b seller -o model/escrow.modality
```

Or describe your requirements in natural language:

```bash
modality model synthesize --describe "escrow where buyer deposits, seller ships, oracle confirms delivery before release"
```

Review and customize the generated model for oracle integration:

```modality
export default model {
  initial awaiting_deposit
  
  // Buyer deposits funds
  awaiting_deposit -> funded [+signed_by(/users/buyer.id)]
  
  // Seller ships goods
  funded -> shipped [+signed_by(/users/seller.id)]
  
  // Oracle confirms delivery -> release to seller
  shipped -> completed [+oracle_attests(/oracles/delivery.id, "delivered", "true")]
  
  // Oracle denies delivery -> refund buyer
  shipped -> refunded [+oracle_attests(/oracles/delivery.id, "delivered", "false")]
  
  // Timeout: buyer can reclaim after deadline
  shipped -> refunded [+signed_by(/users/buyer.id), +after(/escrow/timeout.datetime)]
  
  // Terminal states (self-loop)
  completed -> completed [+signed_by(/users/buyer.id)]
  completed -> completed [+signed_by(/users/seller.id)]
  refunded -> refunded [+signed_by(/users/buyer.id)]
  refunded -> refunded [+signed_by(/users/seller.id)]
}
```

## Step 6: Commit the Setup

```bash
modal c commit --all --sign buyer -m "Initialize escrow"
```

## Step 7: Execute the Contract

### Happy Path: Delivery Confirmed

```bash
# 1. Buyer deposits
modal c commit --all --sign buyer -m "Buyer deposits"

# 2. Seller ships
modal c commit --all --sign seller -m "Seller ships"

# 3. Oracle attests delivery
modal c commit --all --sign delivery_oracle -m "Oracle confirms, funds released"
```

### Dispute Path: Delivery Failed

```bash
modal c commit --all --sign delivery_oracle -m "Oracle denies delivery, buyer refunded"
```

### Timeout Path: Oracle Unresponsive

After the timeout deadline:

```bash
modal c commit --all --sign buyer -m "Timeout refund"
```

## Security Properties

| Property | Protection |
|----------|------------|
| **Authenticity** | Only trusted oracle can attest |
| **Integrity** | Signature covers all attestation data |
| **Freshness** | Max age prevents replay |
| **Binding** | Contract ID prevents cross-contract replay |
| **Timeout** | Buyer protected if oracle fails |
