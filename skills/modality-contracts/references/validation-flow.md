# Validation Flow — Detailed Reference

## Commit Structure

A commit contains:
```json
{
  "hash": "hex string",
  "parent": "parent hash or null",
  "data": {
    "method": "POST|DELETE|MODEL|RULE|REPOST|CREATE|SEND|RECV",
    "path": "/some/path",
    "body": "content"
  },
  "signature": "sig_hex:pubkey_hex"
}
```

Signatures may appear in multiple formats:
- `commit.signature` — string `"sig:pubkey"` or object `{signature, signer_key}`
- `commit.signatures` — array of signer keys
- `commit.head.signatures` — object `{pubkey: sig_data}`

## State Replay

Building state from commits:

```
state = {}
model = null
modelState = null
rules = []

for each commit in order:
  if method == POST:  state[path] = body
  if method == DELETE: delete state[path]
  if method == MODEL:  model = parse(body); modelState = model.initial
  if method == RULE:   rules.push(parse(body))
                       if data.model: model = parse(data.model); modelState = model.initial
```

## Transition Matching

For a commit to be accepted when a model exists:

```
candidates = model.transitions.filter(t => t.from == modelState)
for each candidate:
  allPredicatesSatisfied = true
  for each predicate on candidate:
    holds = evaluatePredicate(predicate, commit, state)
    if predicate.positive and !holds: allPredicatesSatisfied = false
    if !predicate.positive and holds: allPredicatesSatisfied = false
  if allPredicatesSatisfied:
    modelState = candidate.to
    return ACCEPT

return REJECT (no transition matched)
```

## Parsing Model Transitions

Transition syntax: `state_a -> state_b [predicates]`

Predicates inside brackets:
- `+predicate(arg)` — positive, must hold
- `-predicate(arg)` — negative, must NOT hold
- `+adds_rule` or `-adds_rule` — bare (no parens)
- `[]` or omitted — no predicates, permissive

## Model Replacement

Models can be replaced by a MODEL commit. The new model must satisfy all accumulated rules. The model state resets to the new model's initial state.

Rules can never be removed. They accumulate forever.

## Error Reporting

When rejecting a commit, report which transitions were tried and which predicates failed:

```
"No valid transition from state 'active'. 
  transition -> active: +any_signed(/) not satisfied; 
  transition -> locked: -modifies(/config) violated"
```
