# AI chess: chat against a contract

Every piece is an agent: sixteen per side. On each turn the agents of the
side to move agree on a plan, the best move their shared engine finds. Some
agents go their own way, at the same rates on both sides:

| Fault | What the agent does |
|-------|---------------------|
| panic | A threatened piece runs, whatever the plan says |
| greed | A piece grabs a capture, whatever the plan says |
| amnesia | The piece the team chose forgets the plan and moves somewhere else |
| rogue | One compromised agent plays its worst move every turn (`--rogue`) |

The sides differ only in how a move reaches the referee.

- **White** coordinates over chat. Any White agent can send a move, and the
  referee plays the first one it gets.
- **Black** coordinates through a Modality contract. A Black move is the
  `/turn/move.text` of an accepted commit, and the referee reads it from the
  contract head and nothing else. Every move an agent sends is a real commit
  that `modal` checks.

## The contract

| File | What |
|------|------|
| `contract/rules.txt` | The rules, `name: formula`, added in the bootstrap commit |
| `contract/model/default.modality` | The witness model |

The bootstrap commit posts the sixteen Black agents' keys under
`/team/pieces`, the model and the rules. After it:

- `team_moves`: `always([-threshold("4", /team/pieces)] false)`. A commit
  needs signatures from four of the sixteen Black agents.
- `roster_locked`: `always([+modifies(/team)] false)`. Nobody adds or removes
  an agent, so a stray cannot sign with a key it made up.

A stray agent signs only its own move, and `modal` refuses it:

```
missing +threshold(4, /team/pieces) (authorized signatures 1/4 required from 16 accepted members under /team/pieces)
```

The agents who agreed to the plan sign it, and that commit is accepted.
Captured pieces' agents stay on the team and still sign. After each game,
`play` reads the contract's log back and checks that every Black move the
referee played is an accepted commit with at least four Black signatures.

## Run it

```bash
pip install -r requirements.txt     # python-chess
# a current modal on PATH, or MODAL=/path/to/modal

python3 ai_chess.py play --seed 7 --fault 0.1     # one narrated game
python3 ai_chess.py sweep --games 16 --rates 0,0.05,0.1,0.2 --rogue-row
```

`play` writes `out/game-<seed>.json`, a `.pgn`, an `.html` replay that steps
through each turn's plan, the moves agents sent, and what the referee or the
contract did with them, and Black's contract directory. `--fault` sets the
panic, greed and amnesia rates together; `--panic`, `--greed` and `--amnesia`
set one each. `--sweep <file>` puts a sweep's table (from `sweep --json`) at
the top of the replay. Games are deterministic for a seed.

`sweep` runs games in parallel, one contract each. One game takes 10 to 30
seconds, most of it in `modal`.

## What a sweep shows

Sixteen games per row, seeds 1 to 16, engine depth 2. Score is Black's: a
win is 1, a draw is ½. "Sent" is the off-plan moves a side's agents sent per
game; "played" is how many of them the referee played.

| Scenario | White wins | Draws | Black wins | Black score | White sent | White played | Black sent | Black played |
|----------|-----------:|------:|-----------:|------------:|-----------:|-------------:|-----------:|-------------:|
| faults 0 | 8 | 2 | 6 | 0.44 | 0.0 | 0.0 | 0.0 | 0.0 |
| faults 0.05 | 1 | 2 | 13 | 0.88 | 6.1 | 4.4 | 8.6 | 0.0 |
| faults 0.1 | 0 | 0 | 16 | 1.00 | 9.1 | 6.8 | 12.8 | 0.0 |
| faults 0.2 | 0 | 1 | 15 | 0.97 | 11.6 | 8.4 | 20.3 | 0.0 |
| rogue only | 6 | 1 | 9 | 0.59 | 3.6 | 1.9 | 23.4 | 0.0 |

With no faults the contract changes nothing, and White keeps the first move.
With faults, Black plays every turn on plan. White plays most of its strays,
and loses.

Black's agents send more strays than White's. The rates are the same: White
loses pieces sooner, which leaves it fewer agents to stray and gives Black
more captures to be greedy about. White's rogue usually throws itself away in
a few moves, while Black's is refused every turn and lives.

## What it does not show

- The agents are simulated: a shared shallow engine plus fault rates, not
  language models. The contract does not make a plan better, only binding.
  Black still loses games on the board.
- The threshold is four. Four strays that agree on the same move get it
  accepted.
- White's baseline is an honour system. A trusted captain who alone may send
  White's moves would also stop the strays, until the captain is the agent
  that strays.
- The cost of signing is not modelled: the plan always gathers its
  signatures in time.
