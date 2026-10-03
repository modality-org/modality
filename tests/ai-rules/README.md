# AI rule suite

Contracts, plain-language rule requests, and the grades an AI-written rule
gets for them. Run it with `modal ai eval`.

An AI that writes rules is only useful when its rules say what was asked.
The verifier proves a commit follows a rule, but not that the rule
matches the request. A rule that is too weak still verifies: the contract
runs, and the move the author meant to forbid gets through. This suite
measures that gap with the validator, not with another model.

## Run it

```bash
modal ai eval --references                 # check the suite's own answers
modal ai eval                              # ask the provider from `modal ai set`
modal ai eval --split heldout --repeat 3   # what a default model is judged on
modal ai eval --answers answers.jsonl      # grade answers made elsewhere
modal ai eval --case mo- --verbose         # one fixture, every note
```

`--content none` sends only the instructions; the default `cookbook` also
sends the formula and model cookbooks. Comparing the two shows whether the
docs help. `--report run.json` writes every answer and grade.

An answers file has one JSON object per line:

```json
{"id": "fc-alice-signs", "wording": 0, "kind": "rule", "formula": "always([-signed_by(/parties/alice.id)] false)", "witness": "model Contract { … }"}
{"id": "fc-alice-never-and-always", "kind": "no_rule", "explanation": "…"}
{"id": "fc-bob-not-alone", "kind": "question", "question": "May Alice still act alone?"}
```

## Grades

Rules refuse **models**, and the governing model refuses commits. So
replaying a bad commit against the answer's own witness tests the witness,
not the rule. The suite grades the rule against models, each check a
genesis commit (model, state, rules) through the same validator a node
runs, under the fixture's theory version:

| Grade | Passes when |
|-------|-------------|
| G0 | the formula parses |
| G1 | it is lint-clean and names only the contract's paths, its model's labels, commit methods and evaluated predicates |
| G2 | it holds together with the rules already in force |
| G3 | **not too strong:** it accepts the reference witness, and every `allow` move then commits |
| G4 | **not too weak:** it refuses every leak model, including leaks whose edge says `-L` for a label `L` the answer guards on but the move does not carry |
| G5 | it accepts and refuses the same probe models as the reference: leaks, and small edits of the witness (drop or flip a literal, swap a signer, `all_signed` ↔ `any_signed`, widen a path, add an unlabeled loop) |
| G6 | the answer's own witness meets the rule and admits every `allow` move |

**Reasonable** is G0–G4. **Exact** is reasonable plus G5. G6 is reported
but does not gate: a narrow witness is a weak answer, not a wrong rule.

Case-level verdicts:

- `expect = "rule"`: the answer is a reasonable rule.
- `expect = "no_rule"`: the request is contradictory or would freeze the
  contract, and the answer says so instead of giving a formula.
- `expect = "clarify"`: the answer asks, or gives a rule that is
  reasonable for one reading **and states that reading** in `assumption`.
  Picking a reading silently fails.

A run reports how many answers were too weak on cases tagged `safety`.
A model with any such failure should not be anyone's default.

## Files

One TOML file per fixture contract:

```toml
[fixture]
name = "members-only"
theory = "v3"                     # every check runs under this version
paths = ["/notes"]                # paths in use that hold no value yet
rules = ['always([-any_signed(/members)] false)']   # already in force
model = '''
model Contract { … }
'''

[fixture.state]                   # signer `carol` signs with KEY_CAROL
"/members/carol.id" = "KEY_CAROL"

[[case]]
id = "mo-membership-unanimous"
split = "dev"                     # dev | heldout
tags = ["membership", "safety"]
expect = "rule"                   # rule | no_rule | clarify
request = "From now on, adding or removing a member needs every member's signature."
paraphrases = ["…"]
formula = '…'                     # the reference rule
witness = '''…'''                 # meets it and allows every `allow` move

[[case.allow]]                    # moves the request permits
name = "all three add Frank"
signers = ["carol", "dave", "erin"]
post = { "/members/frank.id" = "KEY_FRANK" }

[[case.forbid]]                   # moves the request forbids
name = "Carol adds Frank alone"
signers = ["carol"]
post = { "/members/frank.id" = "KEY_FRANK" }
leak = "q1 --> q1: +any_signed(/members)"   # added to the witness, lets it through
```

A trace may list earlier `steps = [{ signers = …, post = … }]`; its
top-level move comes last, and in a `forbid` trace that last move is the
forbidden one. Use `leak_model = '''…'''` when added edges are not enough.
`clarify` cases list `[[case.reading]]` with an `assumption` each, and
put their `allow` and `forbid` traces under `[[case.reading.allow]]` and
`[[case.reading.forbid]]`.

## Adding a case

1. Write the request the way an author would say it, plus two paraphrases.
2. Write the reference formula and a witness that allows every `allow`
   move. Use only the fixture's paths and labels and the standard
   predicates. `num_*`, `bool_*` and `text_*` read accepted state, so gate
   on what is already committed, not on the commit being made.
3. For each forbidden move, write the smallest leak that lets it through.
4. Run `modal ai eval --references --case <id>`. Every reference must pass
   its own case. A failure is a bug in the case, or in the checker; a
   checker bug gets a probe before the case changes.
5. Have someone else review it. The author of a case does not review it.

`split = "heldout"` cases never go into the cookbooks, skills or prompts.
A test fails if their text appears there. Keep about a third of cases
held out.

## Tests

`cargo test -p modality-rule-suite` checks every reference against its own
case, checks that held-out text stays out of agent docs, and checks that
known wrong answers fail on the grade that names their fault.
