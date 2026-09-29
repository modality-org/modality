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
- Every commit after the first is signed by the buyer, the seller or the oracle

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
modal c set-named-id /users/buyer.id buyer
modal c set-named-id /users/seller.id seller
modal c set-named-id /oracles/delivery.id delivery_oracle

# Set escrow terms
mkdir -p state/escrow
echo '{"price": 100, "currency": "USDC"}' > state/escrow/terms.json
```

The release is a write to `/escrow/release.json`. The rules below gate that
path.

## Step 4: Define the Rules

Create `rules/escrow-auth.modality`. Every commit after the one that adds it
must be signed by a known party or the oracle:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([-signed_by(/users/buyer.id) -signed_by(/users/seller.id) -signed_by(/oracles/delivery.id)] false)
  }
}
```

Create `rules/escrow-flow.modality`. No commit writes the release without the
oracle's delivery attestation:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([+modifies(/escrow/release.json) -oracle_attests(/oracles/delivery.id, "delivered", "true")] false)
  }
}
```

## Step 5: Write the Witness Model

You can start from a synthesized candidate:

```bash
modality model synthesize --describe "escrow where buyer deposits, seller ships, oracle confirms delivery before release"
```

Review it against both rules. A witness that meets them:

```modality
model Escrow {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/users/buyer.id) -modifies(/escrow/release.json)
    q2 --> q3: +signed_by(/users/seller.id) -modifies(/escrow/release.json)
    q3 --> q4: +signed_by(/oracles/delivery.id) +oracle_attests(/oracles/delivery.id, "delivered", "true")
    q3 --> q5: +signed_by(/oracles/delivery.id) +oracle_attests(/oracles/delivery.id, "delivered", "false") -modifies(/escrow/release.json)
  }
}
```

The first edge is the unlabeled bootstrap. Then the buyer deposits, the seller
ships, and the oracle either confirms delivery (`q4`, release) or denies it
(`q5`, refund). The deposit, shipment and refund edges carry
`-modifies(/escrow/release.json)`: without it, the flow rule is refused for
this model, because those steps could also write the release.

## Step 6: Commit the Setup

```bash
modal c commit --all --sign buyer -m "Initialize escrow"
```

## Step 7: Execute the Contract

### Happy Path: Delivery Confirmed

```bash
# 1. Buyer deposits
echo '{"amount": 100}' > state/escrow/deposit.json
modal c commit --all --sign buyer -m "Buyer deposits"

# 2. Seller ships
echo '{"carrier": "post", "tracking": "123"}' > state/escrow/shipment.json
modal c commit --all --sign seller -m "Seller ships"

# 3. Oracle attests delivery
echo '{"to": "seller"}' > state/escrow/release.json
modal c commit --all --sign delivery_oracle -m "Oracle confirms, funds released"
```

### Dispute Path: Delivery Failed

```bash
echo '{"to": "buyer"}' > state/escrow/refund.json
modal c commit --all --sign delivery_oracle -m "Oracle denies delivery, buyer refunded"
```

`oracle_attests` holds only when the commit carries a valid signed replay
bundle for the claim (see
[standard predicates](../reference/standard-predicates.md#oracle_attests)).
`modal c commit` does not attach one yet, so the two oracle commits above are
refused until it does. The deposit and shipment steps run today.

There is no timeout path. Time predicates such as `after` are not evaluated
yet, so an edge that needs one never fires.

## Security Properties

| Property | Protection |
|----------|------------|
| **Authenticity** | Only trusted oracle can attest |
| **Integrity** | Signature covers all attestation data |
| **Freshness** | Max age prevents replay |
| **Binding** | Contract ID prevents cross-contract replay |
