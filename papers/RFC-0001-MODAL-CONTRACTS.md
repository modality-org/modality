# RFC-0001: Modal Contract Specification

**Status:** Draft  
**Author:** Gerold Steiner  
**Created:** 2026-02-01  
**Version:** 0.1.0

---

## Abstract

This document specifies the Modal Contract format — a cryptographically-verified, append-only log structure for multi-party cooperation. Modal contracts enable agents (human or AI) to negotiate binding agreements with formal verification guarantees.

A modal contract consists of:
1. An **append-only commit log** — the single source of truth
2. A **governing model** — a state machine defining valid transitions
3. **Rules** — temporal logic formulas constraining behavior
4. **State** — data derived by replaying the commit log

Every commit must satisfy the governing model and all accumulated rules. Verification is mathematical, not reputational.

---

## 1. Terminology

| Term | Definition |
|------|------------|
| **Contract** | An append-only log of signed commits |
| **Commit** | An atomic, signed change to the contract |
| **Model** | A Kripke structure (state machine) defining valid transitions |
| **Rule** | A temporal logic formula constraining contract behavior |
| **Predicate** | A WASM module evaluating conditions (e.g., `signed_by`) |
| **State** | Data derived by replaying commits |
| **Party** | An entity identified by an ed25519 public key |
| **Hub** | A coordination service for push/pull of contracts |

---

## 2. Contract Structure

A modal contract is represented as a directory:

```
<contract>/
├── .contract/           # Internal metadata
│   ├── config           # Contract configuration
│   ├── HEAD             # Current commit hash
│   └── commits/         # Commit objects
│       ├── <hash1>
│       ├── <hash2>
│       └── ...
├── state/               # State files (derived from commits)
│   └── <path>.<type>    # e.g., /users/alice.id
├── model/               # Governing model(s)
│   └── default.modality # Primary model
└── rules/               # Rule files
    └── <name>.modality  # e.g., auth.modality
```

### 2.1 Path Types

State files use typed extensions:

| Extension | Type | Description |
|-----------|------|-------------|
| `.id` | Identity | ed25519 public key (hex) |
| `.text` | Text | UTF-8 string |
| `.bool` | Boolean | `true` or `false` |
| `.json` | JSON | Structured data |
| `.md` | Markdown | Documentation |
| `.date` | Date | ISO 8601 date |
| `.datetime` | DateTime | ISO 8601 datetime |
| `.wasm` | WASM | WebAssembly module |
| `.modality` | Modality | Model or rule definition |

---

## 3. Commit Log

The commit log is the single source of truth. State is derived by replaying commits.

### 3.1 Commit Structure

```
{
  "hash": "<sha256>",
  "parent": "<parent_hash>",     // null for genesis
  "timestamp": "<ISO 8601>",
  "author": {
    "id": "<ed25519_pubkey_hex>",
    "signature": "<signature_hex>"
  },
  "method": "<METHOD>",          // POST, RULE, MODEL, ACTION
  "path": "<path>",              // For POST/RULE/MODEL
  "payload": "<content>",        // Method-specific
  "action": { ... }              // For ACTION commits
}
```

### 3.2 Commit Methods

| Method | Description | Example Path |
|--------|-------------|--------------|
| `POST` | Set state value | `/users/alice.id` |
| `RULE` | Add temporal rule | `/rules/auth.modality` |
| `MODEL` | Set governing model | `/model/default.modality` |
| `ACTION` | Execute domain action | n/a (uses `action` field) |

### 3.3 Commit Validation

Every commit must pass:

1. **Signature verification** — ed25519 signature over commit data
2. **Transition validity** — commit action matches a valid model transition
3. **Rule satisfaction** — all accumulated rules remain satisfiable
4. **Type validation** — path extension matches content type

---

## 4. Model Syntax

Models define valid state transitions using Kripke structures.

### 4.1 Grammar

```ebnf
model       ::= "export" "default" "model" "{" model_body "}"
model_body  ::= "initial" state_name transition*
transition  ::= state_name "->" state_name predicate_list?
predicate_list ::= "[" predicate ("," predicate)* "]"
predicate   ::= ("+" | "-") predicate_name "(" path ")"
state_name  ::= identifier
path        ::= "/" identifier ("/" identifier)*
```

### 4.2 Example

```modality
export default model {
  initial idle
  
  idle -> deposited [+signed_by(/users/buyer.id)]
  deposited -> delivered [+signed_by(/users/seller.id)]
  delivered -> released [+signed_by(/users/buyer.id)]
  deposited -> refunded [+signed_by(/users/buyer.id), +signed_by(/users/seller.id)]
}
```

### 4.3 Predicate Polarity

- `+predicate` — predicate must be TRUE for transition
- `-predicate` — predicate must be FALSE for transition

### 4.4 State Inference

States are inferred from transitions. No explicit state declaration required.

---

## 5. Rule Syntax

Rules constrain contract behavior using modal mu-calculus.

### 5.1 Grammar

```ebnf
rule        ::= "export" "default" "rule" "{" rule_body "}"
rule_body   ::= "starting_at" anchor "formula" "{" formula "}"
anchor      ::= "$PARENT" | commit_hash
formula     ::= bool_formula | modal_formula | temporal_formula | fixed_point
bool_formula ::= "true" | "false" | proposition
              | formula "&" formula | formula "|" formula
              | "!" formula | formula "->" formula
              | "(" formula ")"
modal_formula ::= "[" action "]" formula        // box (necessity)
                | "<" action ">" formula        // diamond (possibility)
                | "[<" action ">]" formula      // diamondbox (commitment)
                | "[]" formula | "<>" formula   // unlabeled
temporal_formula ::= "always" "(" formula ")"
                   | "eventually" "(" formula ")"
                   | formula "until" formula
                   | "next" "(" formula ")"
fixed_point ::= "lfp" "(" var "," formula ")"   // least fixed point (μ)
              | "gfp" "(" var "," formula ")"   // greatest fixed point (ν)
              | var                              // variable reference
action      ::= ("+" | "-") predicate_call
```

### 5.2 Operator Semantics

| Operator | Syntax | Meaning |
|----------|--------|---------|
| Box | `[+A] φ` | After ALL A-transitions, φ holds |
| Diamond | `<+A> φ` | After SOME A-transition, φ holds |
| Diamondbox | `[<+A>] φ` | Committed: can do A AND cannot refuse |
| Always | `always(φ)` | φ holds on all paths, forever |
| Eventually | `eventually(φ)` | φ holds on some path, eventually |
| Until | `p until q` | p holds until q becomes true |
| LFP | `lfp(X, φ)` | Least fixed point (reachability) |
| GFP | `gfp(X, φ)` | Greatest fixed point (invariants) |

### 5.3 Temporal as Fixed Points

```
always(f)     ≡ gfp(X, []X & f)
eventually(f) ≡ lfp(X, <>X | f)
until(p, q)   ≡ lfp(X, q | (p & <>X))
```

### 5.4 Example

```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+RELEASE] implies <+DELIVER> true
    )
  }
}
```

This rule ensures: **Release can NEVER happen without prior delivery.**

---

## 6. Predicates

Predicates are WASM modules evaluating conditions.

### 6.1 Standard Predicates

| Predicate | Purpose | Parameters |
|-----------|---------|------------|
| `signed_by(path)` | Verify ed25519 signature | Identity path |
| `threshold(n, path)` | n-of-m multisig | Threshold, signers path |
| `oracle_attests(path, claim, value)` | Oracle attestation | Oracle path, claim, expected |
| `before(path)` | Timestamp constraint | Datetime path |
| `after(path)` | Timestamp constraint | Datetime path |
| `hash_matches(path)` | SHA256 commitment | Hash path |

### 6.2 Predicate Evaluation

Predicates return:

```json
{
  "valid": true,
  "gas_used": 250,
  "errors": []
}
```

### 6.3 Custom Predicates

Deploy WASM modules to `/_code/custom/<name>.wasm`:

```bash
modal contract wasm-upload \
  --wasm-file ./my_predicate.wasm \
  --module-name "/custom/my_predicate"
```

Reference as `+custom_predicate(args)` in models/rules.

---

## 7. Cryptography

### 7.1 Identities

Identities are ed25519 key pairs.

- **Public key:** 32 bytes, hex-encoded
- **Private key:** 64 bytes, stored in passfile

### 7.2 Signatures

Commits are signed using ed25519:

```
signature = ed25519_sign(
  private_key,
  sha256(canonical_commit_bytes)
)
```

### 7.3 Commit Hashing

```
hash = sha256(
  parent_hash || 
  timestamp || 
  author_id || 
  method || 
  path || 
  sha256(payload)
)
```

---

## 8. Hub Protocol

The Contract Hub provides HTTP-based coordination.

### 8.1 Authentication

Two-tier ed25519 authentication:

1. **Identity key** — long-term party identity
2. **Access key** — session key for hub access

Registration:
```
POST /register
{
  "identity_pubkey": "<hex>",
  "access_pubkey": "<hex>",
  "identity_signature": "<hex>"  // Signs access_pubkey
}
```

### 8.2 Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/contracts` | Create contract |
| `GET` | `/contracts/:id` | Get contract metadata |
| `GET` | `/contracts/:id/commits` | List commits |
| `POST` | `/contracts/:id/commits` | Push commit |
| `GET` | `/contracts/:id/commits/:hash` | Get commit |
| `POST` | `/contracts/:id/grant` | Grant access |

### 8.3 Commit Validation

Hub validates each pushed commit:

1. Signature verification
2. Parent exists and matches HEAD
3. Transition matches governing model
4. All rules remain satisfiable

Invalid commits are rejected with error details.

### 8.4 Access Control

Contracts have access lists:

```json
{
  "owner": "<identity_pubkey>",
  "writers": ["<pubkey1>", "<pubkey2>"],
  "readers": ["<pubkey3>"]
}
```

---

## 9. CLI Reference

### 9.1 Contract Commands

```bash
modal contract create              # Create new contract
modal c checkout                   # Create working directory
modal c status                     # Show pending changes
modal c commit --all               # Commit all changes
modal c commit --sign <passfile>   # Signed commit
modal c log                        # Show commit history
modal c diff                       # Show pending diff
```

### 9.2 Identity Commands

```bash
modal id create --path <file>      # Create identity
modal id get --path <file>         # Get public key
```

### 9.3 Hub Commands

```bash
modal hub start                    # Start local hub
modal hub register                 # Register identity
modal c remote add <name> <url>    # Add hub remote
modal c push <remote>              # Push to hub
modal c pull <remote>              # Pull from hub
```

---

## 10. Examples

### 10.1 Simple Escrow

**Model:**
```modality
export default model {
  initial init
  
  init -> deposited [+signed_by(/users/buyer.id)]
  deposited -> delivered [+signed_by(/users/seller.id)]
  delivered -> released [+signed_by(/users/buyer.id)]
  deposited -> disputed [+signed_by(/users/buyer.id)]
  deposited -> disputed [+signed_by(/users/seller.id)]
  disputed -> resolved [+signed_by(/users/arbiter.id)]
}
```

**Rule:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+RELEASE] implies <+DELIVER> true
    )
  }
}
```

### 10.2 Multi-Sig Treasury

**Model:**
```modality
export default model {
  initial idle
  
  idle -> proposed [+signed_by(/treasury/member.id)]
  proposed -> approved [+threshold(3, /treasury/signers)]
  approved -> executed [+signed_by(/treasury/executor.id)]
  proposed -> rejected [+threshold(3, /treasury/signers)]
}
```

### 10.3 Oracle-Gated Release

**Model:**
```modality
export default model {
  initial waiting
  
  waiting -> attested [+oracle_attests(/oracles/delivery, "delivered", "true")]
  attested -> released [+signed_by(/users/buyer.id)]
}
```

---

## 11. Security Considerations

### 11.1 Key Management

- Private keys MUST be stored securely (passfiles with restricted permissions)
- Session keys SHOULD be rotated regularly
- Compromised keys require contract migration

### 11.2 Model Design

- Models MUST include terminal states or self-loops
- Avoid unbounded state spaces
- Test models with the checker before deployment

### 11.3 Rule Safety

- Rules are cumulative — cannot be removed
- Test rule satisfiability before committing
- Contradictory rules will deadlock the contract

### 11.4 Hub Trust

- Hubs validate but don't guarantee liveness
- Multi-hub replication recommended for critical contracts
- Ultimately, chain anchoring provides strongest guarantees

---

## 12. Conformance

An implementation conforms to this specification if it:

1. Correctly parses model and rule syntax
2. Validates commits against governing models
3. Verifies ed25519 signatures
4. Evaluates standard predicates correctly
5. Derives state by replaying commit logs
6. Rejects commits that violate accumulated rules

---

## Appendix A: ABNF Grammar

```abnf
; Model
model           = "export" SP "default" SP "model" SP "{" model-body "}"
model-body      = "initial" SP state-name *transition
transition      = state-name SP "->" SP state-name [predicate-list]
predicate-list  = "[" predicate *("," predicate) "]"
predicate       = ("+" / "-") predicate-name "(" path ")"
state-name      = identifier
path            = "/" identifier *("/" identifier) ["." extension]

; Rule  
rule            = "export" SP "default" SP "rule" SP "{" rule-body "}"
rule-body       = "starting_at" SP anchor "formula" SP "{" formula "}"
anchor          = "$PARENT" / commit-hash

; Formula (simplified)
formula         = "true" / "false" / proposition
                / formula SP "&" SP formula
                / formula SP "|" SP formula
                / "!" formula
                / formula SP "->" SP formula
                / "[" action "]" SP formula
                / "<" action ">" SP formula
                / "[<" action ">]" SP formula
                / "always" "(" formula ")"
                / "eventually" "(" formula ")"
                / "lfp" "(" var "," formula ")"
                / "gfp" "(" var "," formula ")"
                / "(" formula ")"

; Primitives
identifier      = ALPHA *(ALPHA / DIGIT / "_")
extension       = "id" / "text" / "bool" / "json" / "md" / "wasm" / "modality"
commit-hash     = 64HEXDIG
```

---

## Appendix B: References

- [Modal Mu-Calculus](https://en.wikipedia.org/wiki/Modal_%CE%BC-calculus)
- [Kripke Structures](https://en.wikipedia.org/wiki/Kripke_structure)
- [Ed25519](https://ed25519.cr.yp.to/)
- [Dotcontract](https://dotcontract.org/)

---

*RFC-0001: Trust through verification, not reputation.* 🔐
