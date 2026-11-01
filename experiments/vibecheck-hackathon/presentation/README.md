# Multi-agent chess: the hackathon deck

A [reveal.js](https://revealjs.com) deck with everything vendored, so it runs
offline. Open `index.html` in a browser. No server and no build step needed.

```
open presentation/index.html
```

The deck runs in 13 slides:

- the players, and a Chat vs Chat game where the agents talk and post `/move`;
- RULES.md, and RULES.md vs Chat;
- the FV Contract, which ties with RULES.md;
- one rogue a side, the FV Contract with rogue vs RULES.md with rogue;
- the head-to-head matrix, showing only the half below the diagonal.

## Keys

| Key | Does |
| --- | --- |
| → ← or Space | Next or previous slide |
| S | Speaker view with notes and a timer |
| `[` `]` | On a game slide: previous or next move |
| I | On a game slide: next incident (a move off plan, refused or held back) |
| T | Light or dark |
| ? | Every key |

Slides change with no transition. `index.html?game=rogue-vs-rogue&ply=10#/11`
opens a replay at a given move.

## Files

| File | What it is |
| --- | --- |
| `index.html` | The slides and their speaker notes |
| `deck.css` | The theme, on top of reveal.js core; system fonts only |
| `deck.js` | The matrix, the one-turn demo, the game replays, and the reveal.js setup |
| `games.js` | The replayed games, as `window.GAMES` |
| `img/ackchyually.jpg` | The Ackchyually meme, from Know Your Meme |
| `vendor/reveal.js/` | reveal.js 5.1.0 (MIT): core, reset, and the notes plugin |

## Regenerating the games

The opening game is `chess talk`: chat against chat, where a language model
writes each turn's team chat (from the simulation's decisions) and the referee
plays the `/move` lines. Its answers are cached under `llm-cache/`, so a rerun
asks nothing new. The rest are `chess play` runs with `--json`. `games.js` wraps them in a
list, each game given a `key` the slides refer to:

```
lake exe chess talk --seed 3 --json talk3.json
lake exe chess talk --seed 2 --rules-md white --json talk-rmc2.json
lake exe chess play --setup rm-rulesmd-rogue-vs-contract-rogue --seed 1 --json a1.json
lake exe chess play --setup rm-contract-rogue-vs-rulesmd-rogue --seed 1 --json b1.json
lake exe chess play --setup rm-rulesmd-vs-contract --seed 3 --json c3.json
```

The matrix numbers are in `deck.js` (`CELLS`), from
`lake exe chess experiments --rogue-matrix --games 128`.
