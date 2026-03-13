# Finance Multisig — Transaction Authorization Demo

Threshold-based multisig authorization with real ed25519 signatures.

## Agents

| Agent | Role |
|-------|------|
| Treasury | Initiates all transactions (required signer) |
| CFO | Required co-signer for large transactions |
| Board Member A/B | Co-signers for medium and large transactions |
| Mallory | Attacker (unauthorized) |

## Thresholds

| Amount | Required Signatures |
|--------|-------------------|
| < $10K | Treasury alone (1-of-1) |
| $10K–$100K | Treasury + 1 co-signer (2-of-3) |
| > $100K | Treasury + CFO + 1 board member (3-of-4) |

## Run

```bash
npm install
npm run demo
```

## What It Demonstrates

1. ✓ Small payment — single signature sufficient
2. ✗ Medium payment with only treasury — rejected
3. ✓ Medium payment with proper co-signer — approved
4. ✗ Large payment without full quorum — rejected
5. ✓ Large payment with full quorum — approved
6. ✗ Unknown signer (Mallory) — rejected
7. ✗ Mallory as fake co-signer — rejected
