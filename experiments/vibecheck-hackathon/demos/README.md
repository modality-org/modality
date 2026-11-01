# Demos

Four walkthroughs of what this Lean 4 rebuild of modal contracts does. Each
one has a single command to run and quotes the output it explains.

| Demo | Command | What it shows | Time |
| --- | --- | --- | --- |
| [Alice and Bob](alice-bob/) | `lake exe alice-bob` | Rules, models and signatures between two parties: forged and replayed signatures refused, a model that satisfies the rules but favors its author, a counterexample, the predicate theory at work, a synthesized model | < 1 s |
| [One game of multi-agent chess](chess-game/) | `lake exe chess play --seed 7` | Sixteen agents a side. White coordinates over chat; Black through a signed modal contract that refuses every off-plan move | < 1 s |
| [The rogue's takeover](rogue-takeover/) | `lake exe chess play --white rules-md --black self-ruled --rogue --seed 4` | Why an empty contract belongs to whoever writes the first rule, and how a team's first rule or a preset contract stops a takeover | < 1 s each |
| [The experiments](chess-experiments/) | `lake exe chess experiments --games 64` | All 17 setups, 1,088 games, compared with the original Python run | 44 s on 12 cores |

Start with Alice and Bob. It introduces commits, predicates, rules, models
and signatures one at a time, and the chess demos build on all of them.

## Signatures

Every commit in these demos is signed, and the contract refuses any commit
with a signature that doesn't verify. There are two schemes:

- **`ed25519`:** real public-key signatures, as Modality uses. This is the
  default for `alice-bob`.
- **`oracle`:** fast ideal signatures for experiments. A signature is a
  keyed hash, checked by an oracle that holds the keys. This is the default
  for `chess`.

Any command takes `--crypto ed25519` or `--crypto oracle`. The stories come
out the same either way; only key fingerprints and commit ids change. For
how the oracle works and what it gives up, see the
[project README](../README.md).

## Before you run them

From `experiments/vibecheck-hackathon`:

```sh
lake build        # Lean 4.14.0 (see lean-toolchain); no other dependencies
```

To run the tests behind the demos (the model checker against brute force,
chess move generation, and the SHA-512 and Ed25519 vectors):

```sh
lake build tests && .lake/build/bin/tests
```

The [project README](../README.md) explains how each point of the plan was
interpreted, and what is left undone.
