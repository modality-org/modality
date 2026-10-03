---
sidebar_position: 7
title: Agent on a Git Repo — Test Results
---

# Agent on a Git Repo: Test Results

All 46 checks passed at commit `f271e6c6b005` on 2026-10-03.

These are the results of the two suites behind
[An Agent on a Git Repo](agent-git-landing.md). The page is written by
`tests/network/agent-git-landing/report.py` from a real run; it is not
edited by hand.

| Suite | What it checks | Checks | Passed | Failed |
| --- | --- | --- | --- | --- |
| Contract proofs | The rules and witness model under theory `v2` and `v3`, commit by commit | 7 | 7 | 0 |
| End-to-end run | A local hub, a bare origin, five keys, and `modal-git` doing each step | 39 | 39 | 0 |

## Contract proofs

`landing_tests` in `rust/modality-common` reads the example's own
`model/default.modality` and `rules.txt`. Each case runs under theory
`v2` and again under `v3`.

| Case | Result |
| --- | --- |
| Sets from writes the value accepted at another path | Pass |
| The landing rules hold for the witness | Pass |
| A replacement model cannot drop the path | Pass |
| A clean change lands along the path | Pass |
| A protected change needs a review of that sha | Pass |
| A land without the right evidence is refused | Pass |
| The agent cannot write evidence or change the contract | Pass |

## End-to-end run

`tests/network/agent-git-landing/test.sh`. A check that something is
refused passes when it is refused. For the forged land, the run also
checks that the hub's refusal names the rule it breaks.

| Stage | Check | Result |
| --- | --- | --- |
| Setup | Should create steward's passfile | Pass |
| Setup | Should create agent's passfile | Pass |
| Setup | Should create ci's passfile | Pass |
| Setup | Should create reviewer's passfile | Pass |
| Setup | Should create mallory's passfile | Pass |
| Setup | The hub should listen on 18571 | Pass |
| Setup | The steward bootstraps the contract and the origin hook | Pass |
| Editing and local commits are free | The agent commits locally with no contract step | Pass |
| Editing and local commits are free | Pushing main straight to the origin is refused | Pass |
| A clean change lands along the path | A key that is not an agent's cannot propose | Pass |
| A clean change lands along the path | The agent proposes its commit | Pass |
| A clean change lands along the path | Landing before any attestation is refused | Pass |
| A clean change lands along the path | The agent cannot attest its own change | Pass |
| A clean change lands along the path | The runner attests | Pass |
| A clean change lands along the path | The attestation: tests pass, nothing protected | Pass |
| A clean change lands along the path | The runner cannot land | Pass |
| A clean change lands along the path | The agent lands | Pass |
| A clean change lands along the path | The origin's main is the landed sha | Pass |
| A protected change needs a review of that sha | The agent proposes a change to the tests | Pass |
| A protected change needs a review of that sha | The runner attests | Pass |
| A protected change needs a review of that sha | The attestation marks it protected | Pass |
| A protected change needs a review of that sha | Landing a protected change without a review is refused | Pass |
| A protected change needs a review of that sha | The agent cannot review its own change | Pass |
| A protected change needs a review of that sha | The reviewer approves that sha | Pass |
| A protected change needs a review of that sha | The reviewed change lands | Pass |
| A protected change needs a review of that sha | The origin's main is the reviewed sha | Pass |
| A failing change does not land | The agent proposes a breaking change | Pass |
| A failing change does not land | The runner attests | Pass |
| A failing change does not land | The attestation records the failure | Pass |
| A failing change does not land | A failed run is refused | Pass |
| A failing change does not land | The origin refuses the unlanded sha | Pass |
| A failing change does not land | Status names what is missing | Pass |
| A client that skips local verify is refused by the hub | A client that skips the rules signs a head write | Pass |
| A client that skips local verify is refused by the hub | The hub refuses the forged land | Pass |
| A client that skips local verify is refused by the hub | The hub's refusal names the rules it breaks | Pass |
| A client that skips local verify is refused by the hub | The origin still refuses the unlanded sha | Pass |
| Audit: the log, the evidence, and the origin agree | The audit replays the log and finds no problems | Pass |
| Audit: the log, the evidence, and the origin agree | The audit lists both lands | Pass |
| Audit: the log, the evidence, and the origin agree | The audit catches a main the contract did not accept | Pass |

## Run it yourself

```bash
# Both suites, and this page
python3 tests/network/agent-git-landing/report.py

# Or each on its own
(cd rust && cargo test -p modality-common --features model-governance --lib landing_tests)
tests/network/agent-git-landing/test.sh
```
