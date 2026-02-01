# Building an Oracle-Verified Escrow

Learn to create an escrow contract with external delivery verification using the `oracle_attests` predicate.

---

## What We're Building

An escrow where:
- Buyer deposits funds
- Seller ships goods
- Trusted oracle confirms delivery
- Funds release only after oracle verification
- Timeout protects buyer if oracle fails

---

## Step 1: Create the Contract

```bash
mkdir escrow && cd escrow
modal contract create

# Create participant identities
modal id create --path buyer.passfile
modal id create --path seller.passfile

# Create oracle identity (in production, oracle provides their own key)
modal id create --path oracle.passfile
```

---

## Step 2: Set Up State

```bash
modal c checkout

# Add identities
modal c set /users/buyer.id $(modal id get --path ./buyer.passfile)
modal c set /users/seller.id $(modal id get --path ./seller.passfile)
modal c set /oracles/delivery.id $(modal id get --path ./oracle.passfile)

# Set escrow terms
echo '{"price": 100, "currency": "USDC"}' > state/escrow/terms.json

# Set deadline (Unix timestamp)
echo '{"deadline": 1770000000}' > state/escrow/timeout.json
```

---

## Step 3: Define the Model

Create **model/escrow.modality**:

```modality
model oracle_escrow {
  initial awaiting_deposit
  
  // Buyer deposits funds
  awaiting_deposit -> funded [+DEPOSIT +signed_by(/users/buyer.id)]
  
  // Seller ships goods
  funded -> shipped [+SHIP +signed_by(/users/seller.id)]
  
  // Oracle confirms delivery -> release to seller
  shipped -> completed [+RELEASE +oracle_attests(/oracles/delivery, "delivered", "true")]
  
  // Oracle denies delivery -> refund buyer
  shipped -> refunded [+DISPUTE_REFUND +oracle_attests(/oracles/delivery, "delivered", "false")]
  
  // Timeout: buyer can reclaim after deadline if no oracle response
  shipped -> refunded [+TIMEOUT_REFUND +signed_by(/users/buyer.id) +after(/escrow/timeout)]
  
  // Terminal states
  completed -> completed [+DONE]
  refunded -> refunded [+DONE]
}
```

---

## Step 4: Define Protection Rules

Create **rules/escrow_protection.modality**:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // Release requires oracle confirmation
    always ([+RELEASE] implies oracle_attests(/oracles/delivery, "delivered", "true")) &
    
    // Dispute refund requires oracle denial
    always ([+DISPUTE_REFUND] implies oracle_attests(/oracles/delivery, "delivered", "false")) &
    
    // Timeout refund requires deadline passed + buyer signature
    always ([+TIMEOUT_REFUND] implies (signed_by(/users/buyer.id) & after(/escrow/timeout)))
  }
}
```

---

## Step 5: The Oracle Flow

### How Oracle Attestation Works

The oracle creates a signed attestation:

```json
{
  "oracle_pubkey": "abc123...",
  "claim": "delivered",
  "value": "true",
  "contract_id": "escrow_xyz",
  "timestamp": 1769953000,
  "signature": "oracle_signature..."
}
```

The `oracle_attests` predicate verifies:
1. ✅ Oracle is in the trusted list
2. ✅ Signature is valid over the attestation data
3. ✅ Claim type matches expected
4. ✅ Value matches expected (if specified)
5. ✅ Contract ID matches current contract
6. ✅ Attestation isn't too old (if max_age set)

---

## Step 6: Execute the Contract

### Happy Path: Delivery Confirmed

```bash
# 1. Buyer deposits
modal c act DEPOSIT --sign buyer.passfile
modal c commit --all --sign buyer.passfile -m "Buyer deposits"

# 2. Seller ships
modal c act SHIP --sign seller.passfile
modal c commit --all --sign seller.passfile -m "Seller ships"

# 3. Oracle attests delivery (oracle provides attestation)
# The attestation is included in the commit data
modal c act RELEASE --attestation delivery_attestation.json
modal c commit --all -m "Oracle confirms, funds released"
```

### Dispute Path: Delivery Failed

```bash
# ... after SHIP ...

# Oracle attests non-delivery
modal c act DISPUTE_REFUND --attestation non_delivery_attestation.json
modal c commit --all -m "Oracle denies delivery, buyer refunded"
```

### Timeout Path: Oracle Unresponsive

```bash
# ... after SHIP, and deadline has passed ...

# Buyer reclaims after timeout
modal c act TIMEOUT_REFUND --sign buyer.passfile
modal c commit --all --sign buyer.passfile -m "Timeout refund"
```

---

## Security Properties

The oracle predicate provides:

| Property | Protection |
|----------|------------|
| **Authenticity** | Only trusted oracle can attest |
| **Integrity** | Signature covers all attestation data |
| **Freshness** | Max age prevents replay of old attestations |
| **Binding** | Contract ID prevents cross-contract replay |

---

## Advanced: Multiple Oracles

For high-value escrows, require multiple oracle confirmations:

```modality
// 2-of-3 oracles must confirm delivery
shipped -> completed [+RELEASE +threshold(2, /oracles/delivery_quorum)]
```

Or use a primary oracle with appeal:

```modality
// Primary oracle decides
shipped -> completed [+RELEASE +oracle_attests(/oracles/primary, "delivered", "true")]

// Appeal to backup oracle if primary denies
shipped -> appeal [+APPEAL +oracle_attests(/oracles/primary, "delivered", "false")]
appeal -> completed [+OVERRIDE +oracle_attests(/oracles/backup, "override", "true")]
appeal -> refunded [+CONFIRM_DENIAL +oracle_attests(/oracles/backup, "override", "false")]
```

---

## Next Steps

- [Multisig Treasury Tutorial](./MULTISIG_TREASURY.md) - n-of-m multisig
- [standard-predicates.md](../standard-predicates.md) - All predicates
- [FOR_AGENTS.md](../FOR_AGENTS.md) - Why this matters for agents

---

*Questions? Open a GitHub issue or ask on Discord.* 🔐
