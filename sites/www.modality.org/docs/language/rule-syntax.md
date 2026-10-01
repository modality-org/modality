---
sidebar_position: 3
title: Rule Syntax
---

# Rule Syntax

Rules express **temporal constraints** using modal mu-calculus.

## Submitting Rules

A rule commit must leave the contract with a **witness model** that meets
every rule, the new one included. Write the rule, write or synthesize the
model, and commit both:

```bash
modal add-rule --name alice_signs \
  'always([-signed_by(/members/alice.id)] false)'
modality model synthesize --rule rules/alice_signs.modality \
  --verify -o model/default.modality
modal commit --all --sign alice.modal_passfile
```

The validator checks the model against every rule and refuses the commit if
one fails. So a rule that no model can meet cannot be added.

That does not prove the contract can always move. A rule such as
`always([] false)` is met by a model with no moves, and then no commit is
ever accepted again. The public testnet runs predicate theory v3 and refuses
an edge whose labels cannot hold together, such as
`+num_gt(/x.num, "5") +num_lt(/x.num, "3")`. A network that sets no version,
such as the bundled devnets, runs v0 and reads each predicate label as an
opaque name. On v0 a model can meet a rule with that edge, and then no commit
takes it, so a rule that needs it can leave the contract stuck. `modal c theory`
lists those dead edges. See [Predicate theory](../reference/predicate-theory).

## Basic Structure

```modality
rule <name> {
  starting_at $PARENT
  formula {
    <modal_formula>
  }
}

// Or as default export
export default rule {
  starting_at $PARENT
  formula {
    <modal_formula>
  }
}
```

A rule file holds one or more rules. Each rule has one or more
`formula { ... }` blocks, optionally named (`formula no_early_release { ... }`),
and every one is checked and enforced. A file of top-level
`formula <name> { ... }` blocks is also a rule file; each formula is a rule.
`//` comments may appear between items, not inside a formula.

The validator reads the whole file or refuses the commit. A misspelled
keyword, an unclosed brace, or trailing text is an error, never skipped.

## Anchoring (`starting_at`)

```modality
starting_at $PARENT           // The commit that adds the rule
```

A rule is anchored at the commit that adds it, from the states that commit
reaches. `starting_at` is optional, and `$PARENT` is its only value. Any
other anchor is refused.

## Modal Operators

| Operator | Meaning |
|----------|---------|
| `[ACTION] φ` | After ALL ACTION transitions, φ holds |
| `<ACTION> φ` | After SOME ACTION transition, φ holds |
| `[-ACTION] φ` | If ACTION is refused/impossible, φ holds |
| `[<+ACTION>] φ` | Committed: CAN do ACTION and CANNOT refuse |

### Commitment Versus Enabledness

Use the operator that matches the claim you want the contract to make:

```modality
<+PAY> true
```

`PAY` is enabled from the current witness state.

```modality
[<+PAY>] true
```

`PAY` is committed: it is enabled and its refusal edge is unavailable.

With several labels, `[<+PAY +signed_by(/parties/alice.id)>] true` refuses
each one: no commit may move without `PAY`, and none without Alice's
signature.

```modality
[+PAY] +signed_by(/parties/alice.id)
```

Every matching `PAY` transition must carry Alice's signature predicate.

Avoid `[+PAY] true` as a guard. A box formula with inner `true` is satisfied
even when there is no matching `PAY` transition, so it does not prove `PAY`
happened or that `PAY` is committed. Run `modality model lint <file>` before
signing rules; it reports this as `modality/vacuous-box-guard`.

## Temporal Operators (Syntactic Sugar)

```modality
always(φ)           // φ holds forever (invariant)
                    // = gfp(X, φ & []X)

eventually(φ)       // φ holds now, or some path of moves reaches φ
                    // = lfp(X, φ | <>X)

until(p, q)         // some path keeps p true until it reaches q
                    // = lfp(X, q | (p & <>X))
```

`eventually` and `until` promise that φ can be reached, not that it will
be. No rule can make a commit happen: runs may stop, or loop, before
reaching φ. To promise that φ stays reachable wherever the contract
goes, write `always(eventually(φ))`.

## Fixed Points

```modality
// Greatest fixed point (νX) - invariants, safety
gfp(X, property & []X)

// Least fixed point (μX) - reachability, liveness
lfp(X, target | <>X)

// Unicode alternatives
νX. (property & []X)
μX. (target | <>X)
```

A name in a posted rule must be a variable bound by `lfp` or `gfp`. Model
node names are the model author's choice, so a rule such as `always(safe)`
would hold on any model with a node named `safe` and constrain no commit.
The validator refuses a rule that names a node. Rules already in a
contract's log keep replaying.

## Boolean Connectives

```modality
φ & ψ           // Conjunction (and)
φ | ψ           // Disjunction (or)
!φ              // Negation (not)
true            // Always true
false           // Always false
```

Prefer explicit boolean form for conditional rules:

```modality
!+modifies(/members) | +all_signed(/members)
```

The parser still accepts implication syntax in some contexts, but docs and
onboarding examples avoid it so temporal steps and proof implication are not
conflated. `modality model lint <file>` reports signed-rule uses as
`modality/implication-sugar`; rewrite them to explicit Boolean form before
signing.

## Lint Findings

`modality model lint <file>` reads every rule and formula in a file and
reports what parses but does not say what it seems to. `modal add-rule` prints
the same findings for the rule it writes, and checks the new rule against the
rules already in `rules/`.

| Code | Finding |
|------|---------|
| `modality/vacuous-box-guard` | `[+ACTION] true` holds everywhere |
| `modality/implication-sugar` | `A -> B`; write `!A \| B` |
| `modality/bare-witness-prop` | A bare name, which reads a model node, not contract state |
| `modality/witness-node-leak` | A bare name that matches a node of the witness model |
| `modality/backward-eventually-ordering` | `eventually` under a guard, read as "already happened" |
| `modality/unsatisfiable-label-set` | Box or diamond labels no commit can carry together |
| `modality/guarded-diamond` | `always(!<+X> true \| <+X +E> true)`, which forbids nothing; write `always([+X -E] false)` |
| `modality/leading-next-box` | A rule that starts with `[]`, which also leaves the next commit free |
| `modality/redundant-label` | A label the other labels of its box or diamond already imply |
| `modality/redundant-conjunct` | A `[L] false` box another box of the same rule already covers |
| `modality/subsumed-rule` | A rule whose boxes other rules already cover |

The last three use the standard predicate declarations and no contract state,
so what they report holds in every contract: `+num_gt(/x.num, "7")` implies
`+num_gt(/x.num, "5")`, and `always([-signed_by(/a.id)] false)` covers
`always([+modifies(/x) -signed_by(/a.id)] false)`.
