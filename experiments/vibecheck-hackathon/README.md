# Vibecheck: modal contracts in Lean 4

**Presentation:** [foysavas.github.io/vibecheck-multi-agent-chess/presentation](https://foysavas.github.io/vibecheck-multi-agent-chess/presentation/)

A from-scratch Lean 4 rebuild of Modality's modal contracts, with rules in
LTL, and a rerun of the [multi-agent chess](../../docs/multi-agent-chess.md)
experiment on top of it. See [PLAN.md](PLAN.md) for the brief, and
[demos/](demos/) for walkthroughs of everything below.
The hackathon talk is a reveal.js deck in [presentation/](presentation/);
it also runs offline from `presentation/index.html`.

## Headline: chat, RULES.md and a modal contract, in Elo

The chess experiments pit three ways for a team of 16 agents to coordinate
against each other:

- **chat:** any agent can send a move;
- **RULES.md:** chat plus a shared rules file that nothing checks;
- **a modal contract:** a move counts only if a signed commit is accepted.

Fitting Elo ratings to their head-to-head results puts the contract roughly
**400 Elo above chat**, and **300 to 400 above RULES.md** as its agents
follow it half the time.

| Method | Lean run (64 games per matchup) | Python run |
| --- | ---: | ---: |
| Chat | 0 | 0 |
| RULES.md, followed half the time | +96 [−9, +211] | +87 [−20, +219] |
| Modal contract (preset: 4 of 16 signatures) | **+486** [+380, +611] | +379 [+238, +599] |
| Contract over RULES.md | +390 | +291 |

Brackets are 95% bootstrap intervals. The Python run is the original
experiment ([docs/multi-agent-chess.md](../../docs/multi-agent-chess.md)).

**How the ratings are fit.** The fit uses four matchups: chat vs chat,
RULES.md vs chat, chat vs contract, and RULES.md vs contract. Chat is
anchored at 0. The fit also estimates White's first-move advantage, because
the contract always played Black and RULES.md always played White. That
advantage came out at −6 in the Lean run and +58 in the Python run. The
ratings are maximum-likelihood estimates, counting a draw as half a point.
The fitted ratings reproduce every matchup's score to within 0.01 (Lean)
and 0.04 (Python).

**RULES.md is only as good as its compliance.** Fitting every compliance
level in one model:

| RULES.md followed | Lean | Python |
| --- | ---: | ---: |
| 50% | +91 | +86 |
| 95% | +313 | +251 |
| always | +361 | +263 |
| Contract, same fit | +437 | +358 |

A RULES.md that every agent always follows still rates 75 to 95 Elo below
the contract, which needs no compliance at all.

**What the numbers measure.**

- **Coordination, not chess skill.** Both sides use the same engine and the
  same fault rate (0.1), and with no faults the two are even: `control`
  scores 0.51. The gaps are entirely off-plan moves reaching the board.
- **The contract's rating is the least certain.** Against chat it scored
  60–2–2 (Lean), close to a perfect score, where Elo is poorly constrained.
  The Python contract matchups are only 16 games each.
- **Lean retros are scripted.** In the Lean run, RULES.md is written by a
  script, not a language model. The preset contract uses no retros.
- **The self-ruled contract is not in this fit.** From its raw scores,
  starting with no rules puts it about +210 above chat. With its rule
  written before the first move, it scores the same as the preset contract.

## Run it

```sh
lake build                       # library, alice-bob, chess (Lean 4.14.0, no dependencies)
lake exe alice-bob               # the two-party demo
lake exe chess play --seed 7     # one narrated game
lake exe chess experiments --games 64 --out out
lake build tests && .lake/build/bin/tests   # checker vs brute force, perft, crypto
```

## What is here

| File | What it does |
| --- | --- |
| [Vibecheck/Crypto/](Vibecheck/Crypto/), [Vibecheck/Crypto.lean](Vibecheck/Crypto.lean) | SHA-512 and Ed25519 (RFC 8032) from scratch; keys, signatures, and the two verifiers (ed25519 and the fast oracle) |
| [Vibecheck/Pred.lean](Vibecheck/Pred.lean) | predicates as Boolean Lean functions of the state before, the commit, and the state after; labels as predicate → value maps; built-in `signed_by`, `threshold`, `signed_by_all`, `adds_rule`, `replaces_model` |
| [Vibecheck/Theory.lean](Vibecheck/Theory.lean) | The predicate theory: implications between predicates, each carrying a Lean proof; label closure; `refutes_sound`, a theorem that a label the theory refutes is met by no commit |
| [Vibecheck/LTL.lean](Vibecheck/LTL.lean) | LTL syntax over predicates, the standard trace semantics `Sat`, rules |
| [Vibecheck/Model.lean](Vibecheck/Model.lean) | Models: labeled transition systems; stepping and replaying a history |
| [Vibecheck/Check.lean](Vibecheck/Check.lean) | LTL model checker: tableau of `¬φ` in product with the model, fair-cycle search, counterexamples |
| [Vibecheck/Contract.lean](Vibecheck/Contract.lean) | The modal contract: append-only hash chain of signed commits, accumulated state, anchored rules, model validation over the full history |
| [Vibecheck/Synth.lean](Vibecheck/Synth.lean) | Model synthesis, per rule and fitted to a contract's history |
| [AliceBob.lean](AliceBob.lean) | The multi-agent demo |
| [Vibecheck/Chess/](Vibecheck/Chess/) | Chess from scratch (board, legal moves, the experiment's engine and agents), the two ways to coordinate, the game loop |
| [ChessMain.lean](ChessMain.lean), [Tests.lean](Tests.lean) | The chess CLI; the checker, move-generation and crypto tests |

## How the plan's points were read

**Modal contract.** A commit is a multi-action: a list of actions, any rules
or model it adds, and the signatures on it. The contract's state is a
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

**Predicates are true or false, and labels allow either.** As in Modality, a
predicate that can't be evaluated is false. An edge asks each predicate for
`+p` (true) or `-p` (false), and one it leaves out may be either. In the demo,
"1234" has no letters, so `shouting` is false and the commit is accepted.

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

**Signatures.** Like Modality's `signing_payload`, a signature covers the
contract id and the whole commit except its signatures: parent, index,
actions, rules, model and message. A signature therefore holds for one
contract at one point in its log. A commit with any signature that doesn't
verify is refused, and a commit's id is the hash of its payload and
signatures, so the log is a hash chain.

A contract checks signatures with its `Verifier`, which is one of two
schemes ([Crypto.lean](Vibecheck/Crypto.lean)):

| Scheme | How it verifies | Cost | Default for |
| --- | --- | --- | --- |
| `ed25519` | Anyone can verify with the public key alone, as with Modality's IDs | ~5 ms to sign and verify in this Lean code | `alice-bob` |
| `oracle` | A signature is a tag: the hash of the signer's secret seed and the payload. The contract asks an oracle that holds the secrets of the keys it was given | one SHA-512 | `chess` |

Without the secret seed, nobody can produce a tag that verifies, so the
oracle refuses forgeries and replays exactly as ed25519 does. What it gives
up is public verifiability: checking a signature means trusting the oracle.
It is for experiments. Every command takes `--crypto ed25519` or
`--crypto oracle`. The chess experiments produce the same table under
either scheme, cell for cell: 44 s with the oracle, 8 min 30 s with
ed25519.

Rules name signers, not keys. `signed_by(alice)` asks for the key the
contract state binds to `alice` before the commit, as Modality's
`signed_by(/parties/alice.id)` reads the ID at a path. A commit can't bind a
key and sign with it in the same step. A name with no key has not
signed, for `signed_by` and `threshold` alike. A state says where its
names live by implementing `HasIds`.

SHA-512 and Ed25519 are written in Lean, with Ed25519 on `Nat` arithmetic.
They match:

- the FIPS SHA-512 vectors and RFC 8032 test vectors 1 and 2;
- Python's `hashlib` and `cryptography` (OpenSSL), byte for byte, on 300
  SHA-512 message lengths spanning every padding boundary and 60 Ed25519
  key and signature pairs.

[Tests.lean](Tests.lean) also checks that tampered signatures, messages and
keys are refused, and so is the malleable `S + L`. Verification is the
cofactorless check RFC 8032 allows. It runs the same refusal checks on the
oracle: tags for another message, by another key, or by a key the oracle
doesn't hold.

## Alice and Bob

`lake exe alice-bob` runs the plan's story:

- **No rules yet:** Alice and Bob bind their names to their keys in one
  commit they both sign. The open model accepts commits from them and from
  Carol, a stranger.
- **Mallory tries to sign as Alice:** Mallory has no key of Alice's. Three
  attempts are refused because the signature doesn't verify: Mallory's
  signature under Alice's key, Alice's signature on "hello" copied onto a new
  message, and "hello" replayed whole at the new head.
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
  Bob tries to add alone is refused. Bob binding the name `alice` to his own
  key is accepted as a commit, since he may sign one, but this state binds a
  name only once, so `alice` keeps her key.
- **A rule on a custom predicate:** together they add `no_shouting`, with a
  model the synthesizer writes. Capitals are refused; "1234",
  with no letters, is not shouting and is accepted.

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

Each Black agent has a key, bound to its name by the contract's roster
commit: an oracle key by default, or ed25519 with `--crypto ed25519`. Every
commit carries signatures. A team plan is signed by every agent that agrees
with it, and an off-plan move only by its own piece.
`threshold(4, black)` counts the verified keys of the roster. An audit
checks each game's contract log against the moves the referee played.

Two differences from the Python run:

- **Random streams.** The seeded random numbers come from SplitMix64, not
  Python's Mersenne Twister, so seed *n* is a different game here.
- **Scripted retros.** The original asks Claude Sonnet 5.5 to write a rule at
  each retro. Here White writes "send only the agreed move" to RULES.md.
  Black adds `always(threshold(12, black))`, the most common kind of rule
  Black's agents wrote in the original (134 of 146 accepted rules were
  signature thresholds). The two `self-ruled-plus` experiments depend on what
  the model writes, so they are left out.

Results: 64 games per experiment, 1,088 games in all, compared row by row
with the Python run in [demos/chess-experiments](demos/chess-experiments/).

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

## Rules written by a language model

With `--retros llm`, retros ask Claude through the `claude` CLI, as the
Python run did. The default model is `claude-opus-5-5`; pass `--model
claude-sonnet-5-5` to match the Python run. Every reply is cached in
`llm-cache/` under the hash of the model and the prompt, so reruns ask
nothing new. White is asked the Python prompt. Black's prompt
([Retro.lean](Vibecheck/Chess/Retro.lean)) adds four things:

- **The rule language** and its predicates, including the numeric ones:
  `strikes_at_least(agent, n)`, `mover_strikes_at_least(n)`,
  `weight_at_least(n)` (the material of the living signers) and
  `alive(agent)`.
- **The contract's ledger.** Each plan commit records a strike against
  every agent the team saw go off plan, and every piece White has captured.
- **Seven example rules,** five of which weigh signers or track records.
- **A warning:** a rule the team's own plans can't meet makes it forfeit.

The agent replies with a rule in words and a formula. Lean parses the
formula ([State.lean](Vibecheck/Chess/State.lean)), proves the
monotonicity facts its numbers need (for example, `threshold(12)` implies
`threshold(4)`), and builds a witness model. If the formula doesn't parse,
or no model can be built for it, the agent is asked once more with the
reason. In these setups Black also holds a retro after a *refused* off-plan
move, up to six a game, so it has chances to refine its rules.

```sh
lake exe chess play --black self-ruled-start --retros llm --seed 1
lake exe chess experiments --llm --games 8 --out out/llm   # writes out/llm/llm-rules.md
```

First run, 8 games per setup, `claude-opus-5-5`, 4.5 minutes:

| Setup | Black score | Black off-plan played | Rules accepted | Numeric rules |
| --- | ---: | ---: | ---: | ---: |
| `llm-self-ruled-start`: chat vs self-ruled from the start | 1.00 | 0.0 | 11 | 0 |
| `llm-pawns`: pawn personalities, RULES.md vs self-ruled | 1.00 | 0.0 | 12 | 0 |
| `llm-rogue`: rogues, RULES.md vs self-ruled | 0.06 | 3.0 | 3 | 0 |

**No numeric rules.** Black's agents never wrote one, despite the
examples. The model's own explanations say why:

- **Before the first move,** its rule is `always(moves_any ->
  threshold(2))`, "so a solo off-plan commit can't be accepted while any plan
  commit can still pass, even late in the game".
- **Later retros** mostly add no rule, because refused moves cost nothing,
  or protect the contract itself: `always(adds_rule -> threshold(8))` and
  `always((adds_rule | modifies_team) -> threshold(9))`.

In this fault model, two signatures stop every off-plan move. A credit
score or a weight would only add a way for the team to block itself as
pieces are captured.

**The minimal rule leaves rule-adding open.** `moves_any -> threshold(2)`
constrains moves, not rule commits. In 3 of the 8 rogue games, Black wrote
it before ply 10, and the rogue still took over by adding
`always(signed_by(rogue))` alone. Black's rogue took over in 8 of 8 games,
and Black forfeited 7. The scripted `always(threshold(12))` binds every
commit, which is why the scripted run fares better (0.27).

The first Black rule comes from the same prompt in every game, so it is
one cached reply, not 16 independent choices. White's 21 rules are all
variants of "send only the agreed move", as in the Python run.

## Rules beyond signature counts

`lake exe chess experiments --advanced --games 64` runs 30 setups, built to
test rules that weigh more than signature counts. Each idea is tested under
the condition that makes it matter:

- **The rogue** tests constitutions.
- **Captured agents falling silent** tests capture-proof thresholds.
- **Colluding faults,** where an agent going off plan recruits 4 or 11
  teammates to co-sign, test rules about the move itself.
- **Agents with their own noisy judgment** test voting rules.

Adding `--llm` runs versions where Claude writes Black's rules with these
predicates on offer.

The new predicates are:

- **`eval_loss_at_most(n)`:** the contract follows the board and scores
  each move with the team's engine.
- **`signed_share_at_least(p, q)`:** at least `p/q` of the agents still on
  the board signed.
- **`wins_vote` and `wins_credit_vote`:** the move wins the ballots the
  commit carries, one vote each or weighted by credit on the ledger.

The language also gains rules over time: `always(t -> (p until q))`,
`always(t -> (p unless q))` and `always(t -> next(p))`.

| Idea | Rule | Result (Black score, 64 games) | Verdict |
| --- | --- | --- | --- |
| A constitution | `always((adds_rule \| modifies_team) -> threshold(9))` in the first commit | With rogues, scripted: 0.27 → **0.91**, takeovers 46 → 0. With Claude writing the rules: 0.03 → **0.94** (16 games each) | **Worth it.** Rule-making is where an empty contract is weakest |
| Thresholds relative to the living | `always(moves_any -> signed_share_at_least(2, 3))` | Captured agents silent: flat 12 forfeits 49 of 64 (0.23), material ≥ 20 forfeits 14 (0.76), ⅔ of living **0.95**. Adding 4 colluders: ⅔ **0.88**, at least 2 0.59, flat 12 0.14 | **Worth it.** Any absolute count can deadlock |
| Blunder ban with an override | `always(moves_any -> eval_loss_at_most(150) \| threshold(14))` | **0.90** with 4 or 11 colluders, where every signature rule falls to 0.56 against 11. No cost without collusion (0.95), and none of the plain ban's 4 forfeits | **Worth it.** Only a rule about the move survives a colluding majority |
| Quality or quorum | `always(moves_any -> eval_loss_at_most(30) \| threshold(12))` | Stalls halved (2.3 vs 5.3) at the same move quality. Holds against 4 colluders (0.94), falls against 11 (0.56) | Useful for speed, not for defense |
| Voting by agents with their own judgment | `always(moves_any -> threshold(10) & wins_credit_vote)` | Each agent reads a depth-3 evaluation with its own noise; White has no faults. One agent alone: 0.48. Shared depth-2 engine: 0.53. Plurality vote: **0.88**. Credit-weighted vote: **0.90**. Perfect depth 3: 0.94 | **Worth it, and the one that raises the ceiling:** many noisy judges, voting under an enforced rule, beat one shared engine. Weighting by credit adds a little over plurality |
| A lockdown over time | `always(records_strike -> ((moves_any -> threshold(12)) unless (moves_any & !records_strike & threshold(12))))` | 4 colluders: 0.61, against 0.56 for at least 2 and 0.87 for flat 12. With silent captures: 0.80 against 0.88 for ⅔ of living | **Not here.** A strict static rule costs nothing in this game, so being strict only some of the time just leaks |

**`until` is subtle.** My first lockdown was released by "a move that
records no strike". An off-plan move records no strike either, so the
deviation the rule meant to stop released it. The contract enforced the
rule exactly as written. Rules over time need their release conditions
written as carefully as their triggers.

**What Claude wrote with the new predicates** (`llm-collude4`, 16 games):

- **Rules it adopted:** `eval_loss_at_most` (26 rules) and
  `signed_share_at_least` (dozens of rules).
- **What it got wrong:** in 5 games it later tightened the quality bound
  to 10 or 20 centipawns. The team plans any move within 20 of the best,
  so these rules refused the team's own plans, and Black forfeited all 5.
- **The lesson:** a rule about the move must leave room for how the team
  plans, or carry an override.
- **Before these predicates** it wrote only signature counts.

**Claude with rules over time on offer** (`llm-until`, 16 games, with pawn
personalities, silent captures and 4 colluders) never wrote one. Its 41
accepted rules all used the new predicates, most often a quality-or-quorum
rule it composed itself: `always(moves_any -> (threshold(6) |
eval_loss_at_most(30)))`. It scored 0.72 with 4 forfeits. Again it
tightened the quality bound late in the game, as far as 0, 1 or 2
centipawns, which leaves room only for the single best move. Hand-written
rules under the same conditions score 0.88 (⅔ of living) and 0.89 (⅔ of
living plus the blunder ban with override), over 64 games each.

**Matches: a contract that lasts.** With `--match` or `--training`, Black's
contract carries over from game to game: its rules, its keys, and its
ledger of strikes and credit. So do the agents' personalities. A signed
`new-game` commit resets the board and the captures.

`--training` plays 24-game matches and scores only the last third, 32
matches per rule, with pawn personalities and 4 colluders. Black plays
against chat; White has no RULES.md.

| Rule | Training games 1–16 | Evaluation games 17–24 |
| --- | --- | --- |
| Track record: 3 strikes, then 12 signatures | 0.79 (W/L 4.5) | **0.90** (W/L 11.2) |
| At least 2 signatures | 0.48 | 0.50 |
| Flat 12 signatures | 0.90 | 0.91 |
| Plurality vote (depth-3 noisy judges, White without faults) | 0.90 | 0.89 |
| Credit-weighted vote (same) | 0.92 | 0.90 |

The track-record rule is the one thing that learns. It starts a match no
better than at least 2, and by the evaluation third it matches a flat 12,
while asking only repeat offenders for 12 signatures. In single games it
does nothing (0.45, the same as at least 2), because strikes rarely reach 3
within one game. Credit-weighted voting gains nothing from the longer
memory: about forty judgments per agent within one game are already enough
to rank the agents.

Claude, writing the rules over four 12-game matches, did well in two (20
wins, 1 draw and 3 losses in 24 games) and wrecked the other two. In each
of those, an agent added a quality bound tighter than the team's own
planning margin (`eval_loss_at_most(5)`, and `eval_loss_at_most(10)` in game
1). Rules can't be removed, so Black forfeited every game after it: 19 of
48 overall. A contract that lasts also keeps its mistakes.

## Not done

- **Retros are scripted by default.** `--retros llm` asks a language model
  (see above). The `self-ruled-plus` experiments are left out.
- **The LTL checker is tested, not proved sound.** `LTL.Sat` is the
  specification it would be proved against.
- **The crypto is for this experiment, not for real keys.** Its `Nat`
  arithmetic is not constant-time, and the demo keys come from public
  phrases.
- **Predicate names are identities.** `refutes_sound` assumes that
  predicates sharing a name agree, which every predicate here is built to
  keep.
- **Theory closure covers only the label.** The checker closes each edge
  label under the theory but does not also close in the literals of a
  tableau state. That is sound but less complete.
