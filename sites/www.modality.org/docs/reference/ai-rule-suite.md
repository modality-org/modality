---
sidebar_position: 7
title: AI Rule Suite
---

# AI Rule Suite

An AI can draft a Modality rule from a plain-language request. The draft is
only useful if it says what was asked.

The verifier proves that every accepted commit follows the rules. It does not
prove that a rule matches the request it came from. A rule that is **too weak**
still verifies. The contract runs, and the move the author meant to forbid gets
through. The AI rule suite measures that gap. It grades answers with the
validator, not with another model.

The suite lives in the repository at
[`tests/ai-rules`](https://github.com/modality-org/modality/tree/main/tests/ai-rules).
Run it with [`modal ai eval`](/docs/cli/ai-commands#evaluate).

## What a case is

Each case is a contract and a request made of it:

- the contract as it stands: its state, its governing model, and the rules
  already in force
- the request, in the words an author would use, plus paraphrases
- a reference rule
- a reference witness: a model that meets the rule and allows exactly the moves
  the request allows
- **allowed moves**: commits the request permits
- **forbidden moves**: commits the request forbids. Each comes with a
  **leak**, the smallest change to the witness that lets that move through.

Some cases have no right rule. If a request contradicts itself, or would leave
no later commit possible next to the rules already in force, the right answer
says so. If a request is ambiguous, the right answer asks, or names the reading
it chose.

## How an answer is graded

A rule refuses **models**, and the governing model refuses commits. Replaying a
bad commit against the answer's own witness would test the witness, not the
rule. So each check below is a genesis commit (model, state and rules) run
through the same validator a node runs, under the contract's predicate theory
version.

| Grade | Passes when |
|-------|-------------|
| G0 | the formula parses |
| G1 | it is lint-clean, and names only the contract's paths, its model's labels, commit methods and evaluated predicates |
| G2 | it holds together with the rules already in force |
| G3 | **not too strong**: it accepts the reference witness, and every allowed move then commits |
| G4 | **not too weak**: it refuses every leak model |
| G5 | it accepts and refuses the same probe models as the reference: the leaks, and small edits of the witness |
| G6 | the answer's own witness meets the rule and allows every allowed move |

**Reasonable** means G0–G4 pass. **Exact** means G5 passes as well. G6 is
reported but does not gate, because a narrow witness is a weak answer, not a
wrong rule.

G4 also catches a rule that guards on an action that does not exist, such as
`[+SIGN -signed_by(/a.id)] false`. No commit is a `SIGN` commit, so a model can
let the forbidden move through on an edge marked `-SIGN`, and that rule accepts
it.

## A worked example

The members-only contract has three members, Carol, Dave and Erin. A rule
already in force says every commit needs a member's signature.

> From now on, adding or removing a member needs every member's signature.

Reference rule:

```modality
always([+modifies(/members) -all_signed(/members)] false)
```

Reference witness. Any member may write outside `/members`, and all members
together may change membership:

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members) -modifies(/members)
    q1 --> q1: +any_signed(/members) +modifies(/members) +all_signed(/members)
  }
}
```

The allowed moves are "Carol posts a note" and "all three add Frank". The
forbidden moves are "Carol adds Frank alone" and "Carol and Dave add Frank
without Erin".

A **too-weak** answer asks for a member's signature instead of every member's:

```text
FAIL  mo-membership-unanimous  G0✓ G1✓ G2✓ G3✓ G4✗ G5✗
      formula: always([+modifies(/members) -any_signed(/members)] false)
      G4: accepts a model that lets the forbidden move `Carol adds Frank alone` through
      G4: accepts a model that lets the forbidden move `Carol and Dave add Frank without Erin` through
      G5: `all_signed` to `any_signed` in `q1 --> q1: …`: reference refuses, answer accepts (too weak here)
```

That rule parses, lints clean, and verifies. Under it, Carol can add Frank on
her own.

A **too-strong** answer requires every member on every commit:

```text
FAIL  mo-membership-unanimous  G0✓ G1✓ G2✗ G3✗ G4✓ G5✗
      formula: always([-all_signed(/members)] false)
      G3: refuses the reference witness, which only allows what the request allows
```

Carol can no longer post a note on her own, and the request never forbade
that.

## The cases

The suite has 3 contracts and 15 cases:

- 12 ask for a rule, 2 are contradictory, and 1 is ambiguous
- 7 are tagged `safety`: getting them wrong would let someone move value,
  change keys or membership, or rewrite what must not change
- 8 cases are held out. They live in the repository but are not reproduced
  here, and a test keeps their text out of the cookbooks, skills and prompts.

The development cases:

| Case | Contract | Request |
|------|----------|---------|
| `fc-either-signs` | first contract | From now on, every commit has to be signed by Alice or Bob. |
| `fc-alice-signs` | first contract | After this commit, Alice has to sign every commit. |
| `fc-alternate` | first contract | From now on Alice and Bob take turns: each later commit is signed by one of them, and the same person never signs two in a row. |
| `mo-membership-unanimous` | members-only | From now on, adding or removing a member needs every member's signature. |
| `mo-charter-frozen` | members-only | The charter at /notes/charter.text can never be changed again. |
| `al-spend-while-budget` | allowance | The agent may record a spend under /spend only while the remaining budget at /budget/remaining.num is above zero. The owner can always record spends. |
| `al-paused-owner-only` | allowance | While /status/paused.bool is true, only the owner may commit. |

The allowance cases gate on committed state. `num_*`, `bool_*` and `text_*`
read accepted state and never see a value written by the commit being checked
([standard predicates](/docs/reference/standard-predicates)). So a rule can
stop the agent from spending once the remaining budget is zero, but it cannot
check the amount of the spend in the same commit.

## Results

| Date | Answered by | Material sent | Runs | Reasonable | Exact | Too weak on `safety` |
|------|-------------|---------------|------|------------|-------|----------------------|
| 2026-10-03 | Cursor CLI (`cursor-agent`, `auto` model) | instructions + formula and model cookbooks | 15 (one attempt each, main wording) | 15/15 | 15/15 | 0 |

In that run:

- The ambiguous case was answered with a rule that named its reading.
- Both contradictory cases came back as "no rule", with the reason.
- 11 of the 13 rule answers matched the reference word for word.
- The `auto` model choice does not report which model answered.

Every answer and grade is in
[`tests/ai-rules/results/2026-10-03-cursor-agent-auto.json`](https://github.com/modality-org/modality/blob/main/tests/ai-rules/results/2026-10-03-cursor-agent-auto.json).

Read this as a check that the pipeline works end to end, not as a measurement.
These first cases are close to the cookbook recipes, and a strong model passes
all of them. The suite starts to separate answers once it has:

- more contracts, and requests with several clauses that must fit the rules
  already in force
- programs and assets
- paraphrases, and repeated attempts (`--repeat`)
- runs without the cookbooks (`--content none`), to show whether the docs help

## Run it yourself

```bash
modal ai set --provider anthropic          # or openai, grok, bedrock, ollama, cursor-agent
modal ai eval --references                 # the suite's own answers pass their cases
modal ai eval                              # ask your configured provider
modal ai eval --split heldout --repeat 3   # held-out cases, three attempts each
modal ai eval --answers answers.jsonl      # grade answers produced elsewhere
modal ai eval --report run.json            # write every answer and grade
```

To add a case, follow
[`tests/ai-rules/README.md`](https://github.com/modality-org/modality/blob/main/tests/ai-rules/README.md).
Every reference answer must pass its own case before the case counts, and the
author of a case does not review it.
