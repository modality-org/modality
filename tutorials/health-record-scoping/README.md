# Health Record Scoping — HIPAA Compliance Demo

Path-based access control with dual-signature enforcement for medical records.

## Agents

| Agent | Scope | Notes |
|-------|-------|-------|
| Scheduling Agent | `/appointments/*` | Cannot access medical or billing records |
| Clinical Agent | `/appointments/*`, `/records/medical/*` | Medical writes require admin co-sign |
| Human Admin | All paths | Co-signs sensitive operations |

## Rules

- `modifies(/appointments/*) → signed_by(scheduling | clinical | admin)`
- `modifies(/records/medical/*) → signed_by(clinical) ∧ signed_by(admin)`
- `modifies(/records/billing/*) → signed_by(admin)`

## Run

```bash
npm install
npm run demo
```

## What It Demonstrates

1. ✓ Scheduling agent books appointment (within scope)
2. ✗ Scheduling agent tries to access medical records (rejected)
3. ✗ Scheduling agent sneaks medical path into commit (caught)
4. ✓ Clinical + Admin dual-sign medical record update
5. ✗ Clinical agent acts alone on medical records (rejected)
6. ✗ Clinical agent tries to modify billing (rejected)
7. ✓ Cross-agent cooperation through proper channels
