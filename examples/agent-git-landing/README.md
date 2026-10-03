# Agent on a git repo

Edits and local commits are free. `main` on the origin moves only along the
contract's path: an agent proposes a sha, a runner attests to that sha, a
reviewer signs it when it touches a protected path, and the agent lands it.

| File | What |
|------|------|
| `modal-git` | The tool: `init`, `propose`, `attest`, `review`, `land`, `status`, `audit`, and `verify-ref` (the origin's `update` hook). Python 3, `git` and `modal` on `PATH` |
| `model/default.modality` | The witness model |
| `rules.txt` | The rules, `name: formula`, added in the owner's bootstrap |

The walkthrough is [docs/tutorials/agent-git-landing.md](../../docs/tutorials/agent-git-landing.md).
The rules and model are checked under theory `v2` and `v3` by `landing_tests`
in `rust/modality-common`, which reads these files. The end-to-end run is
`tests/network/agent-git-landing/test.sh`.
