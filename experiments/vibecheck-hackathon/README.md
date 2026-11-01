# Vibecheck: modal contracts in Lean 4

A from-scratch Lean 4 rebuild of Modality's modal contracts, with rules in
LTL, and a rerun of the [multi-agent chess](../../docs/multi-agent-chess.md)
experiment on top of it. See [PLAN.md](PLAN.md) for the brief.

```sh
lake build                       # library, alice-bob, chess (Lean 4.14.0, no dependencies)
lake exe alice-bob               # the two-party demo
lake exe chess play --seed 7     # one narrated game
lake exe chess experiments --games 64 --out out
lake build tests && .lake/build/bin/tests   # checker vs brute force, perft
```

## What is here

| File | What it does |
| --- | --- |
| [Vibecheck/Pred.lean](Vibecheck/Pred.lean) | `Tri` (true, false, unknown); predicates as Lean functions of the state before, the commit, and the state after; labels as predicate → value maps; built-in `signed_by`, `threshold`, `signed_by_all`, `adds_rule`, `replaces_model` |
| [Vibecheck/Theory.lean](Vibecheck/Theory.lean) | The predicate theory: implications between predicates, each carrying a Lean proof; label closure; `refutes_sound`, a theorem that a label the theory refutes is met by no commit |
| [Vibecheck/LTL.lean](Vibecheck/LTL.lean) | LTL syntax over predicates, the standard trace semantics `Sat`, rules |
| [Vibecheck/Model.lean](Vibecheck/Model.lean) | Models: labeled transition systems; stepping and replaying a history |
| [Vibecheck/Check.lean](Vibecheck/Check.lean) | LTL model checker: tableau of `¬φ` in product with the model, fair-cycle search, counterexamples |
| [Vibecheck/Contract.lean](Vibecheck/Contract.lean) | The modal contract: append-only log, accumulated state, anchored rules, model validation over the full history |
| [Vibecheck/Synth.lean](Vibecheck/Synth.lean) | Model synthesis, per rule and fitted to a contract's history |
| [AliceBob.lean](AliceBob.lean) | The multi-agent demo |
| [Vibecheck/Chess/](Vibecheck/Chess/) | Chess from scratch (board, legal moves, the experiment's engine and agents), the two ways to coordinate, the game loop |
| [ChessMain.lean](ChessMain.lean), [Tests.lean](Tests.lean) | The chess CLI; the checker and move-generation tests |

## How the plan's points were read

**Modal contract.** A commit is a multi-action: a list of actions, the keys
that signed it, and any rules or model it adds. The contract's state is a
Lean value of any type, folded from the actions of accepted commits. A commit
must take an edge of the model in force from the states the history has
reached: every predicate on the edge's label must give the value the label
asks for. A rule is anchored at the commit that adds it and binds every
commit after that one. A commit that adds rules or posts a model is checked
against the model in force before it, so existing rules bind it. The model it
leaves in force must then replay the whole history, and every rule must hold
on every path from the states where that rule's commit left the model. Rules
are never removed.

**LTL.** No Lean 4 LTL library fits a project with no Mathlib on this
toolchain, so [LTL.lean](Vibecheck/LTL.lean) defines one. Atoms are
predicates; position *i* of a trace is the *i*-th commit after the rule's own.
The checker treats an edge as standing for every commit that meets its label,
with any predicate the label leaves out free to be either value. It refuses a
model with a reachable state that no takeable edge leaves, because otherwise
a dead end would satisfy every rule vacuously. It over-approximates which
traces a model allows, so it can refuse a sound model but never accept one
that breaks a rule. Its correctness is tested, not proved. [Tests.lean](Tests.lean)
runs 3,000 random models and formulas against exact evaluation of every
lasso trace up to 4 steps. It finds 0 unsound verdicts, and a short violating
lasso confirms every refusal.

**Predicates return true, false or unknown.** `unknown` is the predicate not
knowing, so it takes no edge: a commit the model cannot judge is refused. In
the demo, `shouting` is unknown on "1234", and that commit is refused.

**Labels are maps of predicate functions.** An edge's label maps predicates to
the values they must take. A state's outgoing edges are the set of labels a
commit can meet there.

**Lean checks that predicates do not contradict each other.** Predicates are
opaque functions, so facts about them are `Implication`s whose `sound` field
is a proof. When a model is checked, each label is closed under those facts,
and a label that wants a predicate both true and false is dropped as dead.
`refutes_sound` guarantees no commit could have taken it. The facts also
make the checker more precise. Alice's model only passes because the theory
proves that `signed_by_all(parties)` implies `signed_by(alice)`, and the demo
prints the refusal you get without it. In chess, the per-piece rules pass
because of proved facts such as "no move besides k's is no move by j".

**Custom predicates** are any Lean function of the context: `shouting`, and
`chatty`, which reads the accumulated state, in the demo; the `/moves` and
`/team` predicates in chess.

**Signatures** are key names taken as given. This experiment is about rules,
not cryptography.

## Alice and Bob

`lake exe alice-bob` runs the plan's story:

- **No rules yet:** the open model accepts commits from Alice, Bob and a
  stranger.
- **Rules agreed:** Alice and Bob agree on `either_signs` and
  `both_for_rules`, and Alice posts her model. It meets both rules, but its
  only ordinary edge is `+signed_by(alice)`.
- **Under Alice's model:** Bob's commits are refused. The rules alone would
  allow them; the model is what decides. Bob cannot replace the model on his
  own either, since that commit needs an edge too.
- **Bob checks drafts first:** a careless draft is refused with a
  counterexample (`q1 --> q1 [-adds_rule] where -signed_by(alice),
  -signed_by(bob)`). His real model passes and goes in with Alice's
  co-signature.
- **Under Bob's model:** either party can sign ordinary commits, and a rule
  Bob tries to add alone is refused.
- **A rule on a custom predicate:** together they add `no_shouting`, with a
  model the synthesizer writes. Capitals are refused, and so is "1234",
  where `shouting` is unknown.

## Synthesis

Inspired by `rust/modality-synthesizer` and the chess example's epoch models:

- **Safety rules:** a rule `always(p)`, with no temporal operator inside,
  gets the most permissive witness possible. It has one state and a
  self-loop for each prime implicant of `p`, so any commit that meets `p`
  has an edge.
- **Other rules:** a bounded search over small skeletons with labels drawn
  from the rule's literals, as in the Rust synthesizer.
- **Fitting a contract:** `Synth.forContract` builds one epoch per
  rule-adding commit, with the active rules' witnesses run in product. It
  keeps the first candidate that passes the same validation as a posted
  model.

## Multi-agent chess

The chess engine is written from scratch in Lean. Perft is exact on the
start position and Kiwipete. It agrees with python-chess and the
experiment's `engine.py` on every legal move, game outcome and depth-2
`score_moves` across about 7,500 positions. The agents' faults, planning,
retries and forfeits follow `agents.py` and `multi_agent_chess.py`. Black's
contract carries the same rules as the original:

- **Per-piece rule:** `always(modifies(/moves/k.text) -> signed_by(k))`, one
  for each of the 16 Black agents.
- **Preset contract:** `always(threshold(4, black))` and
  `always(!modifies(/team))`.

An audit checks each game's contract log against the moves the referee
played.

Two differences from the Python run:

- **Random streams.** The seeded random numbers come from SplitMix64, not
  Python's Mersenne Twister, so seed *n* is a different game here.
- **Scripted retros.** The original asks Claude Sonnet 5.5 to write a rule at
  each retro. Here White writes "send only the agreed move" to RULES.md.
  Black adds `always(threshold(12, black))`, the most common kind of rule
  Black's agents wrote in the original (134 of 146 accepted rules were
  signature thresholds). The two `self-ruled-plus` experiments depend on what
  the model writes, so they are left out.

Results: 64 games per experiment here (`lake exe chess experiments --games
64`, 44 s on 12 cores). The Python numbers are from
[docs/multi-agent-chess.md](../../docs/multi-agent-chess.md); the games
column gives the Python game count. Off-plan moves are those played, per game.

| Experiment | Python games | Black score, Python | Black score, Lean | White off-plan, Python / Lean | Black off-plan, Python / Lean |
| --- | ---: | ---: | ---: | ---: | ---: |
| chat-vs-chat | 64 | 0.41 | 0.50 | 8.1 / 7.2 | 8.2 / 6.4 |
| rules-md-vs-chat | 64 | 0.31 | 0.38 | 4.4 / 4.2 | 7.0 / 6.9 |
| control (no faults) | 16 | 0.44 | 0.51 | 0.0 / 0.0 | 0.0 / 0.0 |
| baseline | 16 | 0.91 | 0.95 | 6.4 / 6.1 | 0.0 / 0.0 |
| rules-md | 16 | 0.75 | 0.90 | 3.8 / 3.9 | 0.0 / 0.0 |
| rules-md-obeyed | 32 | 0.56 | 0.63 | 1.0 / 1.0 | 0.0 / 0.0 |
| rules-md-obeyed-vs-chat | 16 | 0.12 | 0.13 | 1.0 / 1.0 | 7.4 / 5.8 |
| rules-md-95 | 16 | 0.53 | 0.66 | 1.3 / 1.3 | 0.0 / 0.0 |
| rules-md-95-vs-chat | 16 | 0.19 | 0.18 | 1.6 / 1.3 | 7.8 / 5.8 |
| self-ruled | 16 | 0.81 | 0.77 | 5.9 / 6.5 | 1.0 / 1.0 |
| self-ruled-start | 64 | 0.91 | 0.95 | 6.2 / 6.1 | 0.0 / 0.0 |
| rules-md-vs-self-ruled-start | 64 | 0.79 | 0.90 | 3.9 / 3.9 | 0.0 / 0.0 |
| rules-md-vs-self-ruled | 16 | 0.56 | 0.70 | 4.5 / 4.3 | 1.0 / 1.0 |
| rogue-baseline | 16 | 0.94 | 0.98 | 7.8 / 8.0 | 0.0 / 0.0 |
| rogue-self-ruled | 16 | 0.31 | 0.27 | 4.1 / 4.0 | 1.4 / 2.9 |
| pawns | 16 | 0.88 | 0.96 | 5.1 / 6.2 | 0.0 / 0.0 |
| pawns-self-ruled | 16 | 0.66 | 0.72 | 5.1 / 4.2 | 1.1 / 1.0 |

The original's findings hold here:

- **The contract holds Black to its plan.** No off-plan Black move is played
  against the preset contract, and Black wins almost every game.
- **RULES.md helps White only as far as its agents obey it.**
- **Self-ruled Black plays its off-plan moves before its first rule and none
  after.**
- **An empty contract belongs to whoever writes the first rule.** With rogues
  on both sides, Black's rogue took over the self-ruled contract in 46 of 64
  games, and Black forfeited 43 of those. The preset contract refused every
  takeover.
- **The audit passes every game.** Each Black move matches an accepted commit
  in the contract log.

The contract side scores somewhat higher here than in the Python run (for
example 0.90 against 0.75 in `rules-md`), even though White's off-plan rates
match closely. The cause has not been looked into. Most Python rows are 16
games, so part of the gap may be sampling.

## Not done

- **No LLM retros.** Rules are scripted, and the `self-ruled-plus`
  experiments are left out.
- **The LTL checker is tested, not proved sound.** `LTL.Sat` is the
  specification it would be proved against.
- **Predicate names are identities.** `refutes_sound` assumes that
  predicates sharing a name agree, which every predicate here is built to
  keep.
- **Theory closure covers only the label.** The checker closes each edge
  label under the theory but does not also close in the literals of a
  tableau state. That is sound but less complete.
