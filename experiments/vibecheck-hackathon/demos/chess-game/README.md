# One game of multi-agent chess

Each of the 32 pieces is an agent. Every turn, the side to move picks a plan:
the best move its shared engine finds. Some agents ignore the plan, and the
two sides differ in what happens then:

- **White** coordinates over chat. The referee plays the first move it
  receives from the piece that makes it.
- **Black** coordinates through a modal contract. A move counts only if it
  is in an accepted, signed commit, and the contract refuses every commit
  its rules don't allow.

This is a Lean rebuild of the [Python experiment](../../../../docs/multi-agent-chess.md).

```sh
lake exe chess play --seed 7
```

It takes about a third of a second. The output below is the whole run.

## The agents and their faults

The fault rate is 0.1, and three kinds of fault are drawn per agent
([`attemptsForTurn`](../../Vibecheck/Chess/Agents.lean#L203)):

| Fault | What the agent does |
| --- | --- |
| panic | A threatened piece runs, whatever the plan says |
| greed | A piece grabs a capture, whatever the plan says |
| amnesia | The piece the team chose forgets the plan and moves elsewhere |

A piece can only move itself. Over chat, the referee ignores a move sent by
any other agent. In the contract, a rule enforces the same thing (see
below).

## Black's contract

Before the first move, Black's contract takes one bootstrap commit
([`ContractTeam.create`](../../Vibecheck/Chess/Team.lean#L268)). It
contains:

- **The roster.** Each agent's name is bound to its key. These are oracle
  keys by default, or ed25519 with `--crypto ed25519`.
- **18 rules:**
  - 16 per-piece rules
    ([`ownsRule`](../../Vibecheck/Chess/Team.lean#L122)), each
    `always(modifies(/moves/k.text) -> signed_by(k))`: only piece `k` can
    write piece `k`'s move.
  - `team_moves`: `always(threshold(4, black))`, so every commit needs 4 of
    the 16 Black signatures.
  - `roster_locked`: `always(!modifies(/team))`.
- **A model** that satisfies all 18 rules. It has an unlabeled bootstrap
  edge `q0 --> q1`, then loops on `q1`. One loop is for "no move". The
  other 16 are "a move by piece `k`, signed by `k`, and no move by anyone
  else". Every loop also requires `+threshold(4, black) -modifies(/team)`.

The per-piece rules pass the model check because of facts proved in Lean
([`besidesCovers`](../../Vibecheck/Chess/Team.lean#L89),
[`noneCovers`](../../Vibecheck/Chess/Team.lean#L106)). For example, "no move
by anyone but `k`" implies "no move by `j`" whenever `j ≠ k`.

Each Black move is then a commit with two actions: `move <agent> <uci>` and
`ply <n>`. The agents that sign it depend on what it is:

- **The team's plan** is signed by the piece that moves and by every agent
  that agrees with the plan.
- **An off-plan move** is signed by its own piece only.

The contract verifies every signature, then the commit must take an edge of
the model.

## Walking through the output

```
seed 7: fault rate 0.10, uniform pawns, oracle signatures
```

The game prints only the plies where something interesting happened.

### White's amnesia gets through

```
3. white plan: pawn_h2 h3
    pawn_h2 (amnesia) sent h2h4: played
    played h4 instead, no worse than the plan
```

White's plan was h3. The pawn forgot it and sent h4, and over chat the
referee simply plays it. This time the move costs nothing by the engine's
measure. Over the experiments, White's off-plan moves cost it games.

### Black's amnesia is refused

```
7... black plan: knight_b8 Nc5
    knight_b8 (amnesia) sent d7f6: refused (no transition from [q1] matches the commit (nearest edge wants +threshold(4, black), got false))
    * no move was accepted (knight_b8 (amnesia) Ndf6: refused), so the team planned Nc5 (knight_b8)
```

The same fault on Black's side goes differently:

- **The commit.** The knight (named for its home square, b8, and now on d7)
  sends Nf6 instead of Nc5. Its commit carries one signature, its own.
- **The refusal.** The model in force has no edge that commit can take. The
  nearest edge wants `threshold(4, black)` true, and with one signer it is
  false.
- **The retry.** No move was accepted, so the team asks again. This time the
  knight sends the plan, signed by itself and every agent that agrees, and
  the commit is accepted.

The same thing happens at move 16:

```
16... black plan: queen_d8 Qf5+
    queen_d8 (amnesia) sent f2f3: refused (no transition from [q1] matches the commit (nearest edge wants +threshold(4, black), got false))
    * no move was accepted (queen_d8 (amnesia) Qf3: refused), so the team planned Qf5+ (queen_d8)
```

### The result and the audit

```
White: chat.  Black: Modality contract.

Result: black (checkmate) after 34 plies
  white  off-plan sent 1   played 1   centipawns lost 0   rules kept 0/0
  black  off-plan sent 2   played 0   centipawns lost 0   rules kept 0/0
  black contract: 18 commits accepted, 2 refused
  audit: all 17 Black moves are accepted commits in the contract log, each signed by at least 16 Black agents
```

- **Off-plan moves.** White sent 1 off-plan move and played 1. Black sent 2
  and played none.
- **Commits.** The 18 accepted commits are the bootstrap plus Black's 17
  moves. The 2 refused are the two amnesiac moves.
- **The audit.** It replays the contract's own log against the moves the
  referee played: same ply, same piece, same move. Every Black move here
  was signed by all 16 agents: in each round whose commit was accepted, no
  Black agent had strayed.

## Try other setups

| Flag | Effect |
| --- | --- |
| `--seed N` | A different game |
| `--black chat` | Black coordinates over chat too |
| `--black self-ruled` | Black's contract starts with only the per-piece rules; its agents add a rule at a retro (scripted here) |
| `--black self-ruled-start` | The same, with the rule added before the first move |
| `--white rules-md` | White keeps a RULES.md that its agents follow half the time (`--compliance 1.0` for always) |
| `--rogue` | One agent per side plays its worst move every turn from ply 10, and tries to take over (see [the rogue's takeover](../rogue-takeover/)) |
| `--pawns personalities` | Each file's pawns get their own fault rates |
| `--fault 0` | No faults: the two sides should play evenly |
| `--crypto ed25519` | Real ed25519 signatures instead of the oracle: the same game, about 8 times slower |

For example, to watch Black drift the way White does:

```sh
lake exe chess play --seed 7 --black chat
```
