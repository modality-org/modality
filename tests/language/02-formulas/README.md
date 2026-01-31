# Formula Syntax Tests

This directory demonstrates the Modality formula syntax including modal operators,
temporal operators, and fixed points (modal mu-calculus).

## Files

- `modal-operators.modality` - Box, diamond, and diamondbox operators
- `temporal-operators.modality` - Always, eventually, until, next
- `fixed-points.modality` - lfp and gfp (modal mu-calculus)
- `run-tests.sh` - Script to run all formula tests

## Formula Syntax

### Modal Operators
| Syntax | Meaning |
|--------|---------|
| `<+A> φ` | Diamond: some +A transition leads to φ |
| `[+A] φ` | Box: all +A transitions lead to φ |
| `[<+A>] φ` | Diamondbox: committed (can do +A, cannot refuse) |
| `<> φ` | Unlabeled diamond: some transition leads to φ |
| `[] φ` | Unlabeled box: all transitions lead to φ |

### Temporal Operators
| Syntax | Meaning |
|--------|---------|
| `always(φ)` | φ holds on all paths forever |
| `eventually(φ)` | φ holds at some future state |
| `φ until ψ` | φ holds until ψ becomes true |
| `next(φ)` | φ holds in the next state |

### Fixed Points (Modal Mu-Calculus)
| Syntax | Meaning |
|--------|---------|
| `lfp(X, φ)` | Least fixed point (reachability) |
| `gfp(X, φ)` | Greatest fixed point (invariants) |
| `μX.φ` | Alternate notation for lfp |
| `νX.φ` | Alternate notation for gfp |

## Running Tests

```bash
./run-tests.sh
```

Or run individual checks:

```bash
# Modal operators
modality model check modal-operators.modality --model Escrow --formula CanPay
modality model check modal-operators.modality --model Escrow --formula CommittedToSign

# Temporal operators
modality model check temporal-operators.modality --model SimpleLoop --formula AlwaysSafe
modality model check temporal-operators.modality --model Reachability --formula EventuallyGoal

# Fixed points
modality model check fixed-points.modality --model Invariant --formula SafetyInvariant
```
