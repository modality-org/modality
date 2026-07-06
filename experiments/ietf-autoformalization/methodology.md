# IETF Normative Core Extraction Methodology

Operational checklist for extracting autoformalizable behavior from any IETF RFC.

## Extraction Checklist

For each RFC, work through these steps before writing formulas:

- [ ] Read the abstract and terminology — list all named roles
- [ ] Identify explicit or implied state machine (diagrams, sequence charts, "MUST be in state X")
- [ ] Collect all **MUST**, **MUST NOT**, **SHALL**, **SHALL NOT** sentences involving ordering or authorization
- [ ] Skip **SHOULD**, **MAY**, and implementation recommendations unless they define safety properties
- [ ] Map each role to a Modality identity path (`/users/<role>.id`)
- [ ] Express state-machine transitions as `+sets(/path.text, "value")` (alias: `post_to` / `posts_to`)
- [ ] Add a second path when a parent status stays fixed during sub-steps (e.g. order `pending` while challenge progresses)
- [ ] Mark wire format, crypto, encoding, and timer sections as **out of scope**
- [ ] Write one NL obligation per candidate MUST rule
- [ ] Review obligations with a domain expert before formula generation

## RFC Phrase → Modality Pattern

| RFC language (paraphrased) | Modality formula pattern |
|---|---|
| "State MUST become X" / status transition | `+sets(/path.text, "value")` on model transition |
| "X MUST NOT occur until Y has occurred" | `always(<+sets(/path, "later")> true -> !<+sets(/path, "earlier")> true)` (phase gate) |
| "Sub-step while parent status fixed" | Second path: `+sets(/sub/status.text, …)` with self-loops at witness node |
| "X MUST NOT occur until Y has completed (no overlap)" | same; use `<+…>` diamonds, not `[<+…>]` committed forms |
| "Only party P may perform X" | `always(<+sets(/path, "value")> true -> <+signed_by(/users/p.id)> true)` |
| "Party P is committed to X" | `always([<+X>] true -> <+signed_by(/users/p.id)> true)` |
| "X MUST NOT occur after Y" | `always([+Y] true -> always([-X] true))` |
| "X and Y are mutually exclusive after Z" | `always([+Z] true -> (always([-X] true) & always([-Y] true)))` |
| "X requires evidence E" | `always([+X] true -> <+oracle_attests(/oracles/e.id, "field", "value")> true)` |
| "Parties alternate turns" | `always([+A_TURN] true -> eventually(<+B_TURN> true))` (and reverse) |
| "Process MUST eventually terminate" | `always(eventually(terminal_state))` |

Patterns align with synthesis heuristics in [ROADMAP-AGENT-COOPERATION.md](../../ROADMAP-AGENT-COOPERATION.md) and [llm_synthesis.rs](../../rust/modality-lang/src/llm_synthesis.rs).

## NL Obligation Template

For each candidate MUST rule, fill in:

```
Obligation N:
  RFC section: §X.Y
  Parties:     [role_a, role_b]
  Statement:   "<role_a> must not <action_x> until <role_b> has <action_y>"
  Path write:  +sets(/path.text, "value")
  Prior write: +sets(/path.text, "prior_value")
  Formula slot: F<N>
```

## Model Shape Heuristics

After formulas are written, expect these model shapes (from synthesis heuristics):

| Rule pattern | Expected model shape |
|---|---|
| Single ordering chain | Linear: `init → … → terminal` |
| Mutual exclusion after event | Branching with forbidden transitions |
| "Always allowed" sub-step | Self-loop with `+sets(/sub/path.text, …)` while parent status unchanged |
| Authorization only | Single state, predicate-guarded self-loops |

## Verification Workflow

1. Parse formulas with modality-lang parser
2. **Lint formulas:** `modal model lint rules/*.modality --model model/default.modality` (catches vacuous `[+X] true` guards and witness-node props)
3. Run `synthesize_from_formulas` on the formula set
4. Model-check synthesized model against each formula
5. If fail: inspect counterexample → fix formula or add missing obligation
6. Record result in `synthesis-notes.md`

## Out-of-Scope Reference List

Do not attempt to formalize:

- ASN.1, JSON, CBOR, JWT structure
- Cipher suites, key sizes, signature algorithms
- HTTP header syntax (unless signing *policy* per RFC 9421)
- Polling intervals, exponential backoff, timeout values
- Error response body formats

## Example: Ordering Extraction

**RFC text (paraphrased):** "The authorization server MUST NOT issue an access token before receiving authorization from the resource owner."

**NL obligation:** The authorization server must not issue a token until resource owner authorization is complete.

**Formula:**

```modality
always(<+sets(/token/status.text, "issued")> true -> !<+sets(/token/status.text, "requested")> true)
```

**Authorization variant:**

```modality
always(<+sets(/token/status.text, "issued")> true -> <+signed_by(/users/authorization_server.id)> true)
```

**Avoid:** `eventually(<+Y> true)` for ordering (forward reachability, not prior occurrence). Avoid `[<+X>]` / `![<+Y>]` committed forms for phase gates (miss skip edges). Also avoid bare witness-node identifiers and `[+X] true` vacuous box guards.
