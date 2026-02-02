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

## Step 1: Create the Contract

```bash
mkdir escrow && cd escrow
modal contract create

# Create participant identities
modal id create --path buyer.passfile
modal id create --path seller.passfile
modal id create --path oracle.passfile
```

## Step 2: Set Up State

```bash
modal c checkout

# Add identities
modal c set /users/buyer.id $(modal id get --path ./buyer.passfile)
modal c set /users/seller.id $(modal id get --path ./seller.passfile)
modal c set /oracles/delivery.id $(modal id get --path ./oracle.passfile)

# Set escrow terms
echo '{"price": 100, "currency": "USDC"}' > state/escrow/terms.json

# Set deadline
echo '{"deadline": 1770000000}' > state/escrow/timeout.json
```

## Step 3: Define the Model

Create `model/escrow.modality`:

```modality
model oracle_escrow {
  initial awaiting_deposit
  
  // Buyer deposits funds
  awaiting_deposit -> funded [+DEPOSIT +signed_by(/users/buyer.id)]
  
  // Seller ships goods
  funded -> shipped [+SHIP +signed_by(/users/seller.id)]
  
  // Oracle confirms delivery -> release to seller
  shipped -> completed [+RELEASE +oracle_attests(/oracles/delivery.id, "delivered", "true")]
  
  // Oracle denies delivery -> refund buyer
  shipped -> refunded [+DISPUTE_REFUND +oracle_attests(/oracles/delivery.id, "delivered", "false")]
  
  // Timeout: buyer can reclaim after deadline
  shipped -> refunded [+TIMEOUT_REFUND +signed_by(/users/buyer.id) +after(/escrow/timeout.datetime)]
  
  // Terminal states
  completed -> completed [+DONE]
  refunded -> refunded [+DONE]
}
```

## Step 4: Add Protection Rules

Create `rules/buyer-protection.modality`:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always ([<+RELEASE>] bool_true(/status/shipped.bool))
  }
}
```

This ensures RELEASE can only happen after shipping (tracked in contract state).

## Step 4: Execute the Contract

### Happy Path: Delivery Confirmed

```bash
# 1. Buyer deposits
modal c act DEPOSIT --sign buyer.passfile
modal c commit --all --sign buyer.passfile -m "Buyer deposits"

# 2. Seller ships
modal c act SHIP --sign seller.passfile
modal c commit --all --sign seller.passfile -m "Seller ships"

# 3. Oracle attests delivery
modal c act RELEASE --attestation delivery_attestation.json
modal c commit --all -m "Oracle confirms, funds released"
```

### Dispute Path: Delivery Failed

```bash
modal c act DISPUTE_REFUND --attestation non_delivery_attestation.json
modal c commit --all -m "Oracle denies delivery, buyer refunded"
```

### Timeout Path: Oracle Unresponsive

```bash
modal c act TIMEOUT_REFUND --sign buyer.passfile
modal c commit --all --sign buyer.passfile -m "Timeout refund"
```

## Security Properties

| Property | Protection |
|----------|------------|
| **Authenticity** | Only trusted oracle can attest |
| **Integrity** | Signature covers all attestation data |
| **Freshness** | Max age prevents replay |
| **Binding** | Contract ID prevents cross-contract replay |
