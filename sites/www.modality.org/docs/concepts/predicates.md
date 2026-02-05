---
sidebar_position: 5
title: Predicates
---

# Cryptographic Predicates

Predicates bind **real-world identity** to **logical constraints**.

## Signature Verification

```modality
+signed_by(/parties/alice.id)
```

Requires the commit to be signed by the ed25519 key at that path.

## Threshold Signatures (Multisig)

```modality
+threshold(/signers, 2, 3)
```

Requires 2-of-3 signers from the list.

## Oracle Attestations

```modality
+oracle_attests(/oracles/price-feed.id, "price > 100")
```

Requires an external oracle to attest to a condition.

## Timestamps

```modality
+after(/deadlines/expiry.datetime)
+before(/deadlines/cutoff.datetime)
```

## Complete Reference

| Predicate | Description |
|-----------|-------------|
| `signed_by(path)` | Commit signed by key at path |
| `threshold(path, n, m)` | n-of-m signatures from list |
| `oracle_attests(oracle, claim, value)` | Oracle attestation |
| `before(path)` | Current time before timestamp |
| `after(path)` | Current time after timestamp |
| `num_eq(a, b)` | Numeric equality |
| `num_gt(a, b)` | Numeric greater than |
| `text_eq(a, b)` | Text equality |
| `hash_matches(hash, preimage)` | SHA256 verification |

## Why Predicates Matter

Predicates bridge the gap between:
- **Mathematical logic** (formulas, proofs)
- **Real-world identity** (keys, signatures)
- **External data** (oracles, timestamps)

Without predicates, contracts would be purely abstract. With predicates, they bind to actual cryptographic commitments.
