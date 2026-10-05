# Multi-agent chess: chat against a contract

Every piece is an agent: sixteen per side. On each turn the agents of the
side to move agree on a plan, the best move their shared engine finds. Some
agents go their own way, at the same rates on both sides:

| Fault | What the agent does |
|-------|---------------------|
| panic | A threatened piece runs, whatever the plan says |
| greed | A piece grabs a capture, whatever the plan says |
| amnesia | The piece the team chose forgets the plan and moves somewhere else |
| rogue | One compromised agent plays its worst move every turn (`--rogue`) |

The sides differ in how a move reaches the referee.

| Side | Option | How it coordinates |
|------|--------|--------------------|
| White | `--white chat` | Any White agent can send a move; the referee plays the first one it gets |
| White | `--white rules-md` | The same, plus a `RULES.md` any White agent can edit and nothing checks |
| Black | `--black contract` | A Modality contract with preset rules: four of sixteen signatures, roster locked |
| Black | `--black self-ruled` | A Modality contract with no rules; the agents write their own |

`--pawns personalities` gives each pawn its own fault rates and its own
compliance with `RULES.md`; the other pieces stay uniform.

The ten experiments, and their results, are on the
[Multi-Agent Chess](../../docs/multi-agent-chess.md) docs page. Three of the games can be stepped through on the
[demo page](https://www.modality.org/demos/multi-agent-chess).

## Black's contract

A Black move is the `/turn/move.text` of an accepted commit. The referee
plays the first Black commit the contract accepts for the ply and reads
nothing else. Every move a Black agent sends is a real commit that `modal`
checks. An agent that goes off plan signs its own commit alone; the plan is
signed by every agent that agrees with it.

The preset contract (`contract/`) starts with two rules:

- `team_moves`: `always([-threshold("4", /team/pieces)] false)`. A commit
  needs signatures from four of the sixteen Black agents.
- `roster_locked`: `always([+modifies(/team)] false)`. Nobody adds or removes
  an agent, so a stray cannot sign with a key it made up.

A self-ruled contract starts with the roster and no rules, so any commit is
accepted. After an off-plan move is played, the team holds a retro: one agent
writes a rule in plain language, `modal contract ai suggest-rule` turns it
into a formula, and the team commits it with a witness model. A rule commit
must meet the rules already there, and no rule comes off. A rogue tries to
get in first with a rule that only it can sign.

The witness model for a self-ruled contract (`coordination.epoch_model`) has
one epoch per rule. The model must replay the history from before the rule,
so epoch 0 accepts anything; the commit that adds rule *k* moves to epoch
*k*, whose edges carry the labels of rules 1 to *k* from
`modal model synthesize`.

After each game, the contract's log is read back and checked against the
moves the referee played.

## White's RULES.md

After an off-plan move is played, one White agent adds a rule to `RULES.md`
and says which off-plan moves it covers. An agent about to go off plan is
held back by a covering rule with probability `--compliance` (0.5 by
default). A rogue empties the file whenever it finds rules in it.

## Retros and the language model

Both sides' retros go to the same model, through the `agent` CLI (Cursor's;
set `AGENT_CLI` to change it), and Black's formulas come from
`modal contract ai suggest-rule`, as configured by `modal ai`. Both prompts
describe what happened in the same words; only the part about how the team
coordinates differs (`retro.py`). Every answer is cached by its prompt under
`--cache`, so a rerun asks nothing new and plays the same games.
`results/llm-cache` holds the answers behind the docs page, and
`results/results.json` its numbers.

## Run it

```bash
pip install -r requirements.txt     # python-chess
# a current modal on PATH, or MODAL=/path/to/modal

python3 multi_agent_chess.py play --seed 7                    # one narrated game
python3 multi_agent_chess.py play --white rules-md --black self-ruled --rogue --seed 3
python3 multi_agent_chess.py experiments --games 16 --jobs 8 --cache results/llm-cache
python3 report.py out/experiments                             # rewrites the docs page
python3 site.py                                               # rewrites the demo page's data
```

`play` writes `out/game-<seed>.json`, a `.pgn`, and an `.html` replay that
steps through each turn's plan, the moves agents sent, what the referee or
the contract did with them, and every rule written. `--results` puts an
experiments table above the replay.

`experiments` plays every setup in `EXPERIMENTS` in parallel, one contract
per game, and writes `out/experiments/results.json` with one
`game-<seed>.json` per game. A game that already has results is not played
again, unless `--fresh`. A game takes 20 seconds to a few minutes, most of it
in `modal` and the language model.

| File | What |
|------|------|
| `multi_agent_chess.py` | Games, experiments, replay and audit |
| `engine.py` | The engine every agent shares |
| `agents.py` | Agents, faults and pawn personalities |
| `coordination.py` | Chat, RULES.md, the contracts and their witness models |
| `retro.py` | Retro prompts and the language-model cache |
| `report.py` | Writes the docs page from an experiments run |
| `site.py` | Writes the games and results behind [the demo page](https://www.modality.org/demos/multi-agent-chess) |
| `replay.html` | The replay page template |
| `contract/` | The preset contract's rules and witness model |
