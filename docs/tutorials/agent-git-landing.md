---
sidebar_position: 17
title: An Agent May Edit, and May Not Land
---

# An Agent May Edit, and May Not Land

This is the sixteenth tutorial, after
[The program and the invariant](constant-product-pool). Edits are free.
Landing on the governed branch is propose, attest, review, land. A land
that skips the path is the refusal.

A coding agent works in a git repo. It edits any file, runs anything, and
commits locally as often as it likes. None of that touches a contract. What
the contract governs is the shared branch: `main` on the origin moves only
when an agent proposes a sha, a test runner attests to exactly that sha, a
reviewer signs it when the change touches a protected path, and the agent
then lands it. A land that skips a step, or that brings evidence for some
other sha, is refused. It is not logged and flagged afterwards.

The agent holds one key, its own. It cannot produce the runner's or the
reviewer's signature, however it is prompted.

The network test `tests/network/agent-git-landing` runs everything below
against a local hub. The tool, model and rules are in
`examples/agent-git-landing`.
Every check and its latest result is listed on
[Test Results](agent-git-landing-results.md).

## The pieces

| Piece | Role |
|-------|------|
| Contract on a hub | Holds keys, policy, the accepted head and the evidence; refuses commits the rules forbid |
| Bare origin | The shared repo. Its `update` hook lets `main` move only to the contract's head |
| Agent's clone | An ordinary git repo. Edits and local commits are free |
| `modal-git` | Writes the signed commits for each step and reads the contract back |

| Key | Holder | May |
|-----|--------|-----|
| `/keys/steward.id` | The repo owner | Change keys, policy, the model and the rules |
| `/keys/agents/*.id` | Coding agents | Propose; land |
| `/keys/ci.id` | The test runner | Attest |
| `/keys/reviewers/*.id` | Reviewers | Review |

The contract's state:

| Path | Holds |
|------|-------|
| `/main/head.text` | The sha the contract accepts as `main` |
| `/main/candidate/sha.text` | The proposed sha |
| `/main/candidate/ci/sha.text` | The sha the runner checked out |
| `/main/candidate/ci/base.text` | The head the candidate fast-forwards from |
| `/main/candidate/ci/passed.bool` | Whether the tests passed |
| `/main/candidate/ci/protected.bool` | Whether the diff touches a protected path |
| `/main/candidate/review/sha.text` | The sha a reviewer approved |
| `/policy/protected.text` | Protected globs, one per line |
| `/policy/test.text` | The command the runner runs |

Every piece of evidence names the sha it is about. Git shas are content
addresses, so an attestation of one sha cannot be spent on another.

## The rules

All of them are in `examples/agent-git-landing/rules.txt`. The land is
these, together:

```modality
always([+modifies(/main/head.text) -any_signed(/keys/agents)] false)
always([+modifies(/main/head.text) -sets_from(/main/head.text, /main/candidate/sha.text)] false)
always([+modifies(/main/head.text) -text_eq(/main/candidate/ci/sha.text, /main/candidate/sha.text)] false)
always([+modifies(/main/head.text) -text_eq(/main/candidate/ci/base.text, /main/head.text)] false)
always([+modifies(/main/head.text) -bool_true(/main/candidate/ci/passed.bool)] false)
always([+modifies(/main/head.text) -bool_false(/main/candidate/ci/protected.bool) -text_eq(/main/candidate/review/sha.text, /main/candidate/sha.text)] false)
```

[`sets_from`](../reference/standard-predicates.md#sets_from) binds the new
head to the candidate. `text_eq` reads accepted state, so at the land
`/main/head.text` is still the old head: the fast-forward check comes from
that. The other rules say who writes what: only the runner writes under
`ci/`, and every attestation writes all four facts; only a reviewer writes
under `review/`; only the steward changes keys, policy, the model or the
rules.

The witness model is `examples/agent-git-landing/model/default.modality`:
an unlabeled bootstrap, then one self-loop per move. The rule set and the
witness are checked together under theory `v2` and `v3` by
`landing_tests` in `modality-common`, which reads the example's own files.

## Run it

```bash
# The owner: bootstrap the contract and the origin's hook
modal-git init --hub http://127.0.0.1:8080 --origin ./origin.git \
  --steward steward.mod_passfile --agent agent.mod_passfile \
  --ci ci.mod_passfile --reviewer reviewer.mod_passfile \
  --protected 'tests/**' --protected '.github/**' --test 'sh tests/test.sh'
export MODAL_GIT_HUB=http://127.0.0.1:8080/contracts/<id>   # printed by init

# The agent: edit, commit locally, then propose
git commit -am "Add a feature"
modal-git propose --sign agent.mod_passfile

# The runner, outside the agent's reach
modal-git attest --origin ./origin.git --sign ci.mod_passfile

# What the candidate still needs
modal-git status

# A reviewer, when the attestation marks it protected
modal-git review --sign reviewer.mod_passfile

# The agent: move the head, then push main
modal-git land --sign agent.mod_passfile

# Anyone with the log and the origin
modal-git audit --origin ./origin.git
```

Each command takes a fresh copy of the contract from the hub, writes one
signed commit, and pushes it. A step the rules forbid is refused on the
agent's machine by local verify, and again by the hub, so a client that
skips local verify gets nowhere. The origin's hook replays the log
(`modal c replay --dir`) before `main` moves.

## What each part enforces

- **The hub** refuses any commit the accepted model and rules do not take.
  This is the enforcement point for the path.
- **The origin's hook** refuses a `main` the contract did not accept, a
  push that is not a fast-forward, and deleting `main`.
- **The audit** catches what the hook cannot: whoever runs the origin
  could move `main` around the hook. The audit replays the log, checks
  that each land builds on the last, and that `main` is the contract's
  head.

## Limits

- `passed.bool` is the runner's word. An auditor can re-run the tests but
  cannot replay them. `protected.bool` can be recomputed from the clone
  and the posted globs.
- The agent can edit the tests. Protect the test and CI paths so that
  changing them needs a review.
- One candidate is in flight at a time, and any agent may replace it.
  Give each agent its own slot (`/main/candidates/$k/`) for several at
  once.
- The steward holds `/keys` and could rotate the runner's key to their
  own. The log shows it when they do.
- Rules only accumulate. Decide the rules before the bootstrap; none can
  be loosened later.

Next: [Hand it over](hand-it-over).
