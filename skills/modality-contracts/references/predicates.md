# Predicate Implementation Reference

## Predicate Evaluation

Each predicate is evaluated against a commit + current data state.

### `modifies(path)`
Returns `true` if commit method is POST or DELETE and the commit's path equals or is under the predicate's path.

```
target = normalize(pred.arg)    // strip leading/trailing slashes
commitPath = normalize(data.path)
holds = (commitPath == target) || commitPath.startsWith(target + "/")
```

### `adds_rule`
Returns `true` if commit method is RULE.

```
holds = (data.method == "RULE")
```

No arguments. Can be written with or without parens in model syntax.

### `signed_by(path)`
Returns `true` if the commit is signed by the specific key stored at the given state path.

```
expectedKey = state[pred.arg]   // e.g. state["/users/alice.id"]
holds = commit.signatures.includes(expectedKey)
```

### `any_signed(path)`
Returns `true` if the commit is signed by ANY `.id` member under the path prefix.

```
memberKeys = all state entries where:
  key.startsWith(pred.arg) && key.endsWith(".id")
holds = commit.signatures.some(s => memberKeys.includes(s))
```

Special case: `any_signed(/)` with arg `/` or empty = any signature at all.

### `all_signed(path)`
Returns `true` if the commit is signed by ALL `.id` members under the path prefix.

```
memberKeys = all state entries where:
  key.startsWith(pred.arg) && key.endsWith(".id")
holds = memberKeys.length > 0 && memberKeys.every(k => commit.signatures.includes(k))
```

### `threshold(n, path)`
Returns `true` if at least `n` members under the path signed the commit.

```
args = pred.arg.split(",")
n = parseInt(args[0])
memberPath = args[1].trim()
memberKeys = getMemberKeys(memberPath)
sigCount = commit.signatures.filter(s => memberKeys.includes(s)).length
holds = sigCount >= n
```

## Extracting Signatures from Commits

Signatures can appear in multiple places. Check all of them:

1. `commit.signature` — string format `"sig_hex:pubkey_hex"` → extract pubkey
2. `commit.signature` — object `{signature, signer_key}` → use signer_key
3. `commit.signatures` — array of pubkey strings
4. `commit.head.signatures` — object where keys are pubkeys

Normalize all to lowercase hex for comparison.

## Member Key Resolution

Members are stored as state paths ending in `.id`:
- `/members/alice.id` → `"a09aa5f4..."`
- `/members/bob.id` → `"b12cc6e8..."`

When resolving `any_signed(/members)`, collect all values at paths matching `/members/*.id`.

## Negative Predicates

`-predicate(arg)` means the predicate must NOT hold for this transition.

Critical pattern — without negative guards, a general transition can be exploited:

```modality
// WRONG — first transition allows modifying /members with one sig
active -> active [+any_signed(/members)]
active -> active [+modifies(/members) +all_signed(/members)]

// RIGHT — first transition explicitly blocks /members modification
active -> active [+any_signed(/members) -modifies(/members)]
active -> active [+modifies(/members) +all_signed(/members)]
```
