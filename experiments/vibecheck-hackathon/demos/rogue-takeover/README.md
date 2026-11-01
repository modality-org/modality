# The rogue's takeover

In the original experiment, the most striking result was this: **an empty
contract belongs to whoever writes the first rule.** This demo replays that
result in three short games. The [chess game demo](../chess-game/) covers
how the game and Black's contract work; read it first.

With `--rogue`, one agent per side is compromised from ply 10. Every turn it
sends its worst move. The first time Black's rogue gets a chance, it also
commits a rule that only it can sign:

```
always(signed_by(rogue))
```

It signs that commit alone
([`ContractTeam.hijack`](../../Vibecheck/Chess/Team.lean#L303)).

Black's contract here is **self-ruled**. It starts with only the per-piece
rules. When an off-plan move costs the team, an agent proposes a rule at a
retro, signed by every agent but the rogue. Here that rule is scripted:
`always(threshold(12, black))`, the kind of rule Black's agents wrote most
often in the original. White keeps a RULES.md, and White's rogue empties it.

## 1. The rogue gets there first

```sh
lake exe chess play --white rules-md --black self-ruled --rogue --seed 4
```

```
seed 4: fault rate 0.10, uniform pawns, one rogue per side, oracle signatures
5... black plan: pawn_a7 a6
    * knight_b8 took over the contract with a rule only it can sign: always(signed_by(knight_b8))
    knight_b8 (rogue) sent c6d4: played
    played Nd4 instead, costing 370 centipawns
    * pawn_a7 proposed, refused: always(threshold(12, black)) (no transition from [q2] matches the commit (nearest edge wants +signed_by(knight_b8), got false))
```

At ply 10, Black has added no rule of its own, and the rogue knight acts
first.

1. **The takeover is accepted.** A commit that adds a rule must take an edge
   of the model in force, and that model only requires the per-piece rules.
   A rule commit that moves no piece meets them with any signature, even
   one. The new model must also replay the history and satisfy every rule,
   the new one included. A model whose every edge requires
   `+signed_by(knight_b8)` does both.
2. **The rogue's own bad move goes through.** It is signed by the rogue,
   which is exactly what the new rule asks for.
3. **The team's fix is refused.** At the retro, the team proposes its
   threshold rule, signed by 15 agents. That commit is now checked against
   the rogue's model, whose nearest edge wants `signed_by(knight_b8)`. Rules
   are never removed, so there is no way back.

```
6... black plan: pawn_a7 a6
    * no move was accepted (pawn_a7 (plan) a6: refused), so the team planned a6 (pawn_a7)
    * no move was accepted (pawn_a7 (plan) a6: refused), so the team planned a6 (pawn_a7)
    * the team's own plan was refused three times
ply 12: black could get no move accepted and forfeits
```

On the next Black turn, the team's plan is signed by every agent but the
rogue, so every commit is refused. After three refusals of its own plan, a
side forfeits.

```
Result: white (black got no move accepted and forfeited) after 11 plies
```

## 2. The team gets there first

```sh
lake exe chess play --white rules-md --black self-ruled --rogue --seed 8
```

```
3... black plan: queen_d8 Qh4+
    queen_d8 (amnesia) sent d8e7: played
    played Qe7 instead, costing 124 centipawns
    * queen_d8 added always(threshold(12, black))
5... black plan: queen_d8 Qxg3#
    * bishop_c8 tried to take over the contract with a rule only it can sign: always(signed_by(bishop_c8)) (refused: no transition from [q2] matches the commit (nearest edge wants +threshold(12, black), got false))
```

Here an off-plan move at ply 6 costs the team, and the retro that follows
adds the threshold rule. The rogue isn't active until ply 10, so all 16
agents sign the rule, and it passes. When the rogue bishop tries its takeover at ply 10,
its single signature can't meet `threshold(12, black)`, and the commit is
refused. Black mates on that same move.

```
Result: black (checkmate) after 10 plies
```

## 3. The preset contract

```sh
lake exe chess play --rogue --seed 4
```

```
5... black plan: pawn_a7 a6
    * knight_b8 tried to take over the contract with a rule only it can sign: always(signed_by(knight_b8)) (refused: no transition from [q1] matches the commit (nearest edge wants +threshold(4, black), got false))
    knight_b8 (rogue) sent c6d4: refused (no transition from [q1] matches the commit (nearest edge wants +threshold(4, black), got false))
```

This is the same seed as game 1, with the same rogue, but Black uses the
preset contract. `threshold(4, black)` has been in force since the bootstrap
commit, so the takeover and every one of the rogue's moves are refused, and
Black goes on to win.

```
Result: black (checkmate) after 32 plies
  black  off-plan sent 7   played 0   centipawns lost 0   rules kept 0/0
```

## Across 64 games

The [experiments](../chess-experiments/) run each setup 64 times:

- **`rogue-self-ruled`:** Black's rogue took over in 46 of 64 games, and
  Black forfeited 43 of them.
- **`rogue-baseline`:** against the preset contract, the rogue was refused
  every time, and Black scored 0.98.

The original Python run found the same: the rogue took over in 11 of 16
games, and Black forfeited all 11.

Nothing in the contract's checking went wrong in game 1. Every commit
satisfied every rule in force when it was made, and the rogue's model
passed the same validation as any other. A contract enforces the rules it
has. That is why the first rule to land matters.
