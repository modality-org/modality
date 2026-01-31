# Two-Step Synthesis Pipeline

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Natural Language                            │
│     "Alice pays after Bob delivers, only Alice can release"     │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Step 1: RULE GENERATION                        │
│                      (LLM-Assisted)                             │
│                                                                 │
│  NL → Temporal Modal Logic Formulas                             │
│                                                                 │
│  Output:                                                        │
│    F1: always([+RELEASE] implies eventually(<+DELIVER> true))   │
│    F2: always([+RELEASE] implies <+signed_by(alice)> true)      │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Step 2: MODEL SYNTHESIS                        │
│                    (Algorithmic/Heuristic)                      │
│                                                                 │
│  Formulas → State Machine satisfying them                       │
│                                                                 │
│  Output:                                                        │
│    model Contract {                                             │
│      part flow {                                                │
│        init --> delivered: +DELIVER                             │
│        delivered --> released: +RELEASE +signed_by(alice)       │
│        released --> released                                    │
│      }                                                          │
│    }                                                            │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      VERIFICATION                               │
│                                                                 │
│  Model Checker verifies: M ⊨ F1 ∧ F2                           │
│                                                                 │
│  If FAIL: refine model and retry                               │
│  If PASS: output final model + rules                           │
└─────────────────────────────────────────────────────────────────┘
```

## Why Two Steps?

1. **Separation of concerns**
   - LLM handles ambiguous NL→formal translation
   - Synthesis is deterministic constraint satisfaction

2. **Verifiability**
   - Formulas are the specification
   - Model is verified against spec
   - No "trust the LLM" for correctness

3. **Debuggability**
   - If model is wrong, check formulas first
   - Formulas are human-readable specs

4. **Composability**
   - Add new formulas → re-synthesize
   - Parties add their own protection rules

## Implementation Plan

### Phase 1: Rule Generation
- [ ] Create LLM prompt for NL → Formula
- [ ] Test on common patterns
- [ ] Build pattern library

### Phase 2: Model Synthesis  
- [ ] Implement ordering heuristic
- [ ] Implement authorization heuristic
- [ ] Implement forbidden-after heuristic
- [ ] Handle disjunctions

### Phase 3: Verification Loop
- [ ] Integrate with model checker
- [ ] Implement refinement on failure
- [ ] Add counterexample feedback

### Phase 4: CLI Integration
- [ ] `modality model synthesize --rules "F1, F2, ..."`
- [ ] `modality model synthesize --describe "NL description"`
