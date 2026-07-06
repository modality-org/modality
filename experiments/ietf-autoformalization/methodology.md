# IETF Normative Core Extraction Methodology

Operational checklist for extracting autoformalizable behavior from any IETF RFC.

## Extraction Checklist

For each RFC, work through these steps before writing formulas:

- [ ] Read the abstract and terminology — list all named roles
- [ ] Identify explicit or implied state machine (diagrams, sequence charts, "MUST be in state X")
- [ ] Collect all **MUST**, **MUST NOT**, **SHALL**, **SHALL NOT** sentences involving ordering or authorization
- [ ] Skip **SHOULD**, **MAY**, and implementation recommendations unless they define safety properties
- [ ] Map each role to a Modality identity path (`/users/<role>.id`)
- [ ] Name actions as `+UPPER_SNAKE_CASE` aligned with RFC terminology
- [ ] Mark wire format, crypto, encoding, and timer sections as **out of scope**
- [ ] Write one NL obligation per candidate MUST rule
- [ ] Review obligations with a domain expert before formula generation

## RFC Phrase → Modality Pattern

| RFC language (paraphrased) | Modality formula pattern |
|---|---|
| "X MUST NOT occur until Y has occurred" | `always([<+X>] true -> eventually(<+Y> true))` |
| "X MUST NOT occur until Y has occurred (committed)" | `always([<+X>] true -> eventually([<+Y>] true))` |
| "Only party P may perform X" | `always(<+X> true -> <+signed_by(/users/p.id)> true)` |
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
  Action X:    +ACTION_X
  Action Y:    +ACTION_Y
  Formula slot: F<N>
```

## Model Shape Heuristics

After formulas are written, expect these model shapes (from synthesis heuristics):

| Rule pattern | Expected model shape |
|---|---|
| Single ordering chain | Linear: `init → … → terminal` |
| Mutual exclusion after event | Branching with forbidden transitions |
| "Always allowed" action | Self-loop with `+ACTION` |
| Alternating parties | Two-state cycle |
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
always([<+ISSUE_TOKEN>] true -> eventually(<+AUTHORIZE> true))
```

**Authorization variant:**

```modality
always(<+ISSUE_TOKEN> true -> <+signed_by(/users/authorization_server.id)> true)
```

**Avoid:** bare identifiers like `authorized` or `init` in formulas — those match opaque witness LTS node ids, not contract state. Also avoid `[+X] true` as a guard (vacuous box).
