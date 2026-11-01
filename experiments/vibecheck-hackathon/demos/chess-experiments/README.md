# The experiments

The [chess game demo](../chess-game/) plays one game. This demo runs every
setup of the original experiment that can run without a language model, 64
games each, and compares the results with the
[Python run](../../../../docs/multi-agent-chess.md).

```sh
lake exe chess experiments --games 64 --out out
```

The run plays 1,088 games, spread across your cores. It writes the results
table to the terminal and to `out/results.md`.

| Signatures | Time on 12 cores |
| --- | --- |
| oracle (the default) | 44 s |
| `--crypto ed25519` | 8 min 30 s |

The two runs produce the same table, cell for cell. No game's outcome
depends on the signature scheme, only on which signatures verify.

## The setups

| Experiment | White | Black |
| --- | --- | --- |
| `chat-vs-chat` | chat | chat |
| `rules-md-vs-chat` | chat + RULES.md, obeyed half the time | chat |
| `control` | chat | preset contract; no agent ever faults |
| `baseline` | chat | preset contract |
| `rules-md` | chat + RULES.md, obeyed half the time | preset contract |
| `rules-md-obeyed` | chat + RULES.md, always obeyed | preset contract |
| `rules-md-obeyed-vs-chat` | chat + RULES.md, always obeyed | chat |
| `rules-md-95` | chat + RULES.md, obeyed 95% of the time | preset contract |
| `rules-md-95-vs-chat` | chat + RULES.md, obeyed 95% of the time | chat |
| `self-ruled` | chat | contract with only the per-piece rules; agents add one at a retro |
| `self-ruled-start` | chat | the same, with the rule added before the first move |
| `rules-md-vs-self-ruled-start` | chat + RULES.md | self-ruled from the start |
| `rules-md-vs-self-ruled` | chat + RULES.md | self-ruled |
| `rogue-baseline` | chat, with a rogue | preset contract, with a rogue |
| `rogue-self-ruled` | chat + RULES.md, with a rogue | self-ruled, with a rogue |
| `pawns` | chat | preset contract; each file's pawns have their own fault rates |
| `pawns-self-ruled` | chat + RULES.md | self-ruled; pawns as above |

The list is [`experiments`](../../Vibecheck/Chess/Play.lean#L41). The preset
contract requires 4 of 16 signatures on every commit and locks the roster.
The rogue and the self-ruled contract are explained in
[the rogue's takeover](../rogue-takeover/).

Two setups are left out: `self-ruled-plus` and
`rules-md-vs-self-ruled-plus`. They depend on the rules a language model
writes, and this rebuild scripts its retros instead:

- **White** writes "send only the agreed move" to RULES.md.
- **Black** adds `always(threshold(12, black))`, the most common kind of
  rule Black's agents wrote in the original.

## Reading the table

Each row averages its games. The columns:

- **Black score** counts a win as 1 and a draw as ½. Above 0.5, Black did
  better than White.
- **Off-plan played** is the number of off-plan moves per game that
  actually reached the board. An agent may send more than this; the rest
  were refused, came too late, or were held back by RULES.md.
- **White rules / Black rules** is the number of rules kept per game: lines
  in RULES.md, or rules the contract accepted.
- **Hijacks** counts games where Black's rogue took over the contract.
- **Forfeits W/B** counts games a side lost because none of its moves could
  be accepted.
- **Audit failures** counts games where a Black move did not match the
  contract's own log. It should always be 0.

## Against the Python run

The Python numbers come from
[docs/multi-agent-chess.md](../../../../docs/multi-agent-chess.md), which ran
16, 32 or 64 games per setup, as noted. Off-plan columns are moves played
per game, Python / Lean.

| Experiment | Python games | Black score, Python | Black score, Lean | White off-plan | Black off-plan |
| --- | ---: | ---: | ---: | ---: | ---: |
| chat-vs-chat | 64 | 0.41 | 0.50 | 8.1 / 7.2 | 8.2 / 6.4 |
| rules-md-vs-chat | 64 | 0.31 | 0.38 | 4.4 / 4.2 | 7.0 / 6.9 |
| control | 16 | 0.44 | 0.51 | 0.0 / 0.0 | 0.0 / 0.0 |
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

What to look for:

1. **`control` against `baseline`.** With no faults, the contract changes
   nothing, and the sides score about even (0.51). With faults, White plays
   about 6 off-plan moves a game and Black plays none, and Black's score
   rises to 0.95.
2. **`rules-md`, `rules-md-95`, `rules-md-obeyed`.** White's RULES.md cuts
   its off-plan moves only as far as its agents obey it: 3.9 moves a game at
   half compliance, 1.0 at full. The contract needs no compliance.
3. **`self-ruled`.** Black plays exactly 1.0 off-plan move a game: the one
   that triggers its first retro. After that rule, it plays none.
   `self-ruled-start` adds the rule before the first move, and Black plays
   none at all.
4. **`rogue-self-ruled` against `rogue-baseline`.** These are the
   takeovers: 46 hijacks and 43 Black forfeits with an empty contract. The
   preset contract refuses every attempt.
5. **Audit failures.** 0 in every row.

These findings are the same as the original's. The contract side scores
somewhat higher here (0.90 against 0.75 in `rules-md`), even though White's
off-plan rates match closely. The cause hasn't been looked into. Most Python
rows are 16 games, so part of the gap may be sampling. The rebuild also
draws its random numbers from a different generator than Python's, so seed
*n* is a different game in each.

## Options

| Flag | Effect |
| --- | --- |
| `--games N` | Games per experiment (default 16) |
| `--seed N` | First seed (default 1); games use seeds N to N + games − 1 |
| `--only a,b` | Run only the named experiments, e.g. `--only baseline,control` |
| `--out DIR` | Where to write `results.md` (default `out`) |
| `--crypto ed25519` | Sign with ed25519 instead of the oracle |

A quick check takes a few seconds:

```sh
lake exe chess experiments --games 8 --only control,baseline,rogue-self-ruled
```

To look at any single game from a run, replay its seed with the matching
flags, for example
`lake exe chess play --white rules-md --black self-ruled --rogue --seed 12`.
