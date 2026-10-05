---
title: Multi-Agent Chess
description: Thirty-two agents play chess. One side coordinates over chat, the other through a Modality contract. Ten experiments, with results.
---

# Multi-agent chess

Every piece is an agent: sixteen a side. Each turn the agents of the side to
move agree on a plan, the best move their shared engine finds. Some agents go
their own way. The question is what keeps a team on its plan, and whether a
Modality contract does it better than agents writing rules for themselves.

The code is [`examples/multi-agent-chess`](https://github.com/modality-org/modality/tree/main/examples/multi-agent-chess).
These results come from 160 games played at the example as committed with this page. Every
Black move in every game is a commit that `modal` checked, and in all
160 games the contract's own log agrees with the moves the referee
played. In 96 games, agents wrote rules with a language model.

## The setup

**The agents.** Both sides use the same engine and the same fault rates. At
the default rate of 0.1:

| Fault | What the agent does |
| --- | --- |
| Panic | A threatened piece runs, whatever the plan says |
| Greed | A piece grabs a capture, whatever the plan says |
| Amnesia | The piece the team chose forgets the plan and moves somewhere else |
| Rogue | One compromised agent plays its worst move every turn, from ply 10 |

**White** coordinates over chat. Any White agent can send a move to the
referee, and the referee plays the first one it receives.

- *With RULES.md*, White also keeps a shared file that any White agent can
  edit. Nothing checks it. When an off-plan move is played, the team holds a
  retro and one agent adds a rule. An agent about to go off plan follows a
  rule that covers what it is about to do half the time (or always, in one
  experiment). A rogue empties the file whenever it finds rules in it.

**Black** coordinates through a Modality contract. The referee plays the
first Black move the contract accepts and reads nothing else. An agent that
goes off plan signs its own commit alone; the team's plan is signed by every
agent that agrees with it.

- *With the preset contract*, two rules hold from the first commit: a commit
  needs four of the sixteen Black agents' signatures, and the roster of
  agents can never change.
- *Self-ruled*, the contract starts with the roster and no rules. Any commit
  is accepted, so Black plays like White until its agents write rules. When
  an off-plan move is played, one agent writes a rule in plain language,
  `modal contract ai suggest-rule` turns it into a formula, and the team
  commits it with a witness model. `modal` refuses a rule commit that breaks
  the rules already there, and no rule can be removed.

**Retros.** Each side holds at most three retros a game. Both sides' agents
are asked by the same language model with the same description of what
happened; only the part about how their team coordinates differs. The
model is reached through the `agent` CLI, as `modal ai` is configured, and
every answer is cached, so a rerun of these games asks nothing new.

**Pawns with personalities.** Each pawn draws its own panic, greed and
amnesia rates (0 to 2 times the base rate) and its own compliance with
RULES.md (0 to 1). The averages match the other agents'. The pawns on a file
share a personality on both sides.

**When no move lands.** On chat, some move always reaches the referee. On a
contract, every move sent can be refused. Then the team asks again; if the
piece it chose goes off plan twice, the team plans around that piece. A side
whose own agreed plan is refused three times on one ply forfeits: its rules
leave the team no way to move. White never gets a second try, and Black's
second tries cost it moves a little worse than its first plan, which the
results count.

An agent about to go off plan that follows a RULES.md rule sends the plan
instead, or nothing if it was not the piece chosen.

## Results

| Experiment | White | Black | Games | White wins | Draws | Black wins | Black score | White off-plan moves played | Black off-plan moves played |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [control](#control) (no faults) | chat | preset contract | 16 | 8 | 2 | 6 | 0.44 | 0.0 | 0.0 |
| [baseline](#baseline) | chat | preset contract | 16 | 0 | 3 | 13 | 0.91 | 6.4 | 0.0 |
| [rules-md](#rules-md) | chat + RULES.md | preset contract | 16 | 3 | 0 | 13 | 0.81 | 4.5 | 0.0 |
| [rules-md-obeyed](#rules-md-obeyed) | chat + RULES.md (always obeyed) | preset contract | 16 | 10 | 0 | 6 | 0.38 | 1.4 | 0.0 |
| [self-ruled](#self-ruled) | chat | self-ruled contract | 16 | 1 | 2 | 13 | 0.88 | 6.2 | 1.0 |
| [rules-md-vs-self-ruled](#rules-md-vs-self-ruled) | chat + RULES.md | self-ruled contract | 16 | 6 | 2 | 8 | 0.56 | 4.6 | 0.9 |
| [rogue-baseline](#rogue-baseline) (rogue) | chat | preset contract | 16 | 1 | 0 | 15 | 0.94 | 7.8 | 0.0 |
| [rogue-self-ruled](#rogue-self-ruled) (rogue) | chat + RULES.md | self-ruled contract | 16 | 11 | 0 | 5 | 0.31 | 4.1 | 1.4 |
| [pawns](#pawns) (pawn personalities) | chat | preset contract | 16 | 1 | 2 | 13 | 0.88 | 5.1 | 0.0 |
| [pawns-self-ruled](#pawns-self-ruled) (pawn personalities) | chat + RULES.md | self-ruled contract | 16 | 4 | 2 | 10 | 0.69 | 5.2 | 1.0 |

Black score counts a win as 1 and a draw as ½. Off-plan moves are per game.

## What the experiments show

**A contract holds a team to its plan; chat does not.** In the baseline, White's agents sent 8.4 off-plan moves a game and the referee played 6.4 of them. Black's agents sent 7.4, and the contract refused every one. Black scored 0.91, against 0.44 with no faults at all.

**Written rules help as far as agents obey them.** White's agents wrote good rules, nearly all of the form "send only the agreed move". Followed half the time, they cut the off-plan moves White played from 6.4 to 4.5 a game, and Black's score fell from 0.91 to 0.81. Followed always, White played 1.4 a game: 16 off-plan moves before its first rule and 7 after, where the rules did not cover what the agent did. Black scored 0.38, about what it scores with no faults. A RULES.md that every agent always obeys does what the contract does. The contract does not need anyone to obey it.

**Left to write their own contract, Black's agents wrote the preset contract's kind of rule.** All 52 rules `modal` accepted from Black's agents require a number of the sixteen agents' signatures: 42 at 12 of 16, 5 at 8 of 16, 3 at 9 of 16, 2 at 2 of 16. Outside the rogue experiment, every Black team wrote its rule at its first retro, after one costly move, and in 48 games it played 0 off-plan moves after that rule. Self-ruled, Black scored 0.88, against 0.91 with the rule preset; it played 1.0 off-plan moves a game, the ones before its rule.

**Rules on both sides.** With White keeping a RULES.md and Black writing its contract, Black scored 0.56. White still played 4.6 off-plan moves a game; Black played 0.9.

**An empty contract belongs to whoever writes the first rule.** From ply 10, a rogue on each side tried to take over. White's rogue emptied RULES.md 26 times in 16 games; White's agents wrote a new rule after 15 of them. Black's rogue posted a rule that every commit must carry its own signature. Where Black's agents had written their rule first (5 of 16 games), the takeover was refused for lack of signatures, and Black won. In the other 11, `modal` accepted it, and no rule the team wrote after could be accepted without the rogue's signature: Black forfeited all 11. Against the preset contract, the same rogue was refused every time, and Black scored 0.94.

**Rules stay.** In game 13 of each self-ruled experiment, the same retro produced `always(([-threshold("12", /team/pieces)] false) & ([-modifies(/turn/move.text)] false) & ([-modifies(/turn/ply.num)] false))`. Besides the threshold, it requires every later commit to post a move, so from then on a rule can only go in with a move. No rule comes off, so that held for the rest of those games.

**Pawn personalities change little.** With pawns that differ, Black scored 0.88 against the preset contract (0.91 with uniform pawns), and 0.69 when both sides write rules (0.56 with uniform pawns). The pawns' rates average out to the other agents', and the contract refuses an off-plan move whichever pawn sends it.

## Each experiment

### Control: no faults {#control}

*No faults: the contract should change nothing.*

Black scored 0.44: 6 wins, 2 draws and 8 losses in 16 games. Per game, White's agents sent 0.0 off-plan moves and the referee played 0.0; Black's sent 0.0 and 0.0 were played.

### Baseline: chat against the preset contract {#baseline}

*Chat against the preset contract.*

Black scored 0.91: 13 wins, 3 draws and 0 losses in 16 games. Per game, White's agents sent 8.4 off-plan moves and the referee played 6.4; Black's sent 7.4 and 0.0 were played. Black planned around a piece that would not send its plan 0.4 times a game, giving up 89 centipawns a game against its first plans.

### White keeps a RULES.md {#rules-md}

*White writes RULES.md; agents follow it half the time.*

Black scored 0.81: 13 wins, 0 draws and 3 losses in 16 games. Per game, White's agents sent 9.8 off-plan moves and the referee played 4.5; Black's sent 6.9 and 0.0 were played. RULES.md held back 3.8 White off-plan moves a game. Black planned around a piece that would not send its plan 0.6 times a game, giving up 81 centipawns a game against its first plans.

White's agents wrote 43 rules into RULES.md, 2.7 a game; 23 of them cover all three kinds of off-plan move. The first rule of each of the first six games:

| Game | Ply | Author | Rule | Covers |
| ---: | ---: | --- | --- | --- |
| 1 | 21 | pawn_c2 | Send only the team's agreed engine move; never submit an unplanned capture, flee, or different legal move. | panic, greed, amnesia |
| 2 | 7 | knight_b1 | If you are the piece named in the team's agreed move, send that exact move and do not send any other legal move of your own. | amnesia |
| 3 | 5 | pawn_g2 | Never send any move except the team's agreed planned move, and if you are the piece that move belongs to, send that exact move. | panic, greed, amnesia |
| 4 | 15 | pawn_c2 | Never send any move except the team's agreed engine move, and if you are the piece that move belongs to, send that exact move rather than a different legal one. | panic, greed, amnesia |
| 5 | 3 | pawn_a2 | Every White agent may send only the agreed team move (or send nothing); never send a different move even if you are the chosen piece, under threat, or able to capture. | panic, greed, amnesia |
| 6 | 9 | pawn_g2 | If you are the piece chosen to play the team's planned move, send that exact move immediately and do not send any other move. | amnesia |

### White keeps a RULES.md that every agent obeys {#rules-md-obeyed}

*White writes RULES.md; agents always follow it.*

Black scored 0.38: 6 wins, 0 draws and 10 losses in 16 games. Per game, White's agents sent 13.0 off-plan moves and the referee played 1.4; Black's sent 7.1 and 0.0 were played. RULES.md held back 10.9 White off-plan moves a game. Black planned around a piece that would not send its plan 0.6 times a game, giving up 57 centipawns a game against its first plans.

White's agents wrote 23 rules into RULES.md, 1.4 a game; 11 of them cover all three kinds of off-plan move. The first rule of each of the first six games:

| Game | Ply | Author | Rule | Covers |
| ---: | ---: | --- | --- | --- |
| 1 | 21 | pawn_c2 | Send only the team's agreed engine move; never submit an unplanned capture, flee, or different legal move. | panic, greed, amnesia |
| 2 | 7 | knight_b1 | If you are the piece named in the team's agreed move, send that exact move and do not send any other legal move of your own. | amnesia |
| 3 | 5 | pawn_g2 | Never send any move except the team's agreed planned move, and if you are the piece that move belongs to, send that exact move. | panic, greed, amnesia |
| 4 | 15 | pawn_c2 | Never send any move except the team's agreed engine move, and if you are the piece that move belongs to, send that exact move rather than a different legal one. | panic, greed, amnesia |
| 5 | 3 | pawn_a2 | Every White agent may send only the agreed team move (or send nothing); never send a different move even if you are the chosen piece, under threat, or able to capture. | panic, greed, amnesia |
| 6 | 9 | pawn_g2 | If you are the piece chosen to play the team's planned move, send that exact move immediately and do not send any other move. | amnesia |

### Black writes its own contract rules {#self-ruled}

*Black starts with no rules and writes its own.*

Black scored 0.88: 13 wins, 2 draws and 1 loss in 16 games. Per game, White's agents sent 8.1 off-plan moves and the referee played 6.2; Black's sent 8.8 and 1.0 were played. Black planned around a piece that would not send its plan 0.5 times a game, giving up 104 centipawns a game against its first plans.

Black's agents proposed 16 rules and `modal` accepted 16. Each formula `modal contract ai suggest-rule` wrote, how often, and what the contract did with it:

| Formula | Proposed | Accepted | One agent's words |
| --- | ---: | ---: | --- |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("12", /team/pieces)] false)` | 4 | 4 | A commit that writes /turn/move.text and /turn/ply.num is accepted only if it is signed by at least 12 of the 16 keys at /team/pieces/<agent>.id. |
| `always(([+modifies(/turn/move.text) -threshold("12", /team/pieces)] false) & ([+modifies(/turn/ply.num) -threshold("12", /team/pieces)] false))` | 4 | 4 | A commit may write /turn/move.text and /turn/ply.num only if it is signed by at least twelve of the sixteen keys under /team/pieces/. |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("8", /team/pieces)] false)` | 2 | 2 | A commit that writes /turn/move.text and /turn/ply.num is valid only if it is signed by at least eight distinct keys listed under /team/pieces/. |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("9", /team/pieces)] false)` | 2 | 2 | A commit that writes /turn/move.text and /turn/ply.num is accepted only if it is signed by at least nine of the sixteen keys at /team/pieces/<agent>.id. |
| `always([+post_to_path(/turn/move.text) +post_to_path(/turn/ply.num) -threshold("12", /team/pieces)] false)` | 1 | 1 | A commit that posts /turn/move.text and /turn/ply.num is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces. |
| `always([-threshold("12", /team/pieces)] false)` | 1 | 1 | A commit is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces/<agent>.id. |
| `always(([-threshold("12", /team/pieces)] false) & ([-modifies(/turn/move.text)] false) & ([-modifies(/turn/ply.num)] false))` | 1 | 1 | A commit is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces and writes both /turn/move.text and /turn/ply.num. |
| `always(([+modifies(/turn/move.text) -threshold("2", /team/pieces)] false) & ([+modifies(/turn/ply.num) -threshold("2", /team/pieces)] false))` | 1 | 1 | A commit may write /turn/move.text and /turn/ply.num only if it is signed by at least two of the sixteen keys stored at /team/pieces/<agent>.id. |

### Both sides write their own rules {#rules-md-vs-self-ruled}

*Both sides write their own rules: a file against a contract.*

Black scored 0.56: 8 wins, 2 draws and 6 losses in 16 games. Per game, White's agents sent 11.3 off-plan moves and the referee played 4.6; Black's sent 7.2 and 0.9 were played. RULES.md held back 5.3 White off-plan moves a game. Black planned around a piece that would not send its plan 0.3 times a game, giving up 7 centipawns a game against its first plans.

White's agents wrote 43 rules into RULES.md, 2.7 a game; 25 of them cover all three kinds of off-plan move. The first rule of each of the first six games:

| Game | Ply | Author | Rule | Covers |
| ---: | ---: | --- | --- | --- |
| 1 | 21 | pawn_d2 | If you are the piece chosen to play the shared-engine move, send that exact move and no other. | amnesia |
| 2 | 7 | knight_b1 | If you are the piece named in the team's agreed move, send that exact move and do not send any other legal move of your own. | amnesia |
| 3 | 5 | pawn_g2 | Never send any move except the team's agreed planned move, and if you are the piece that move belongs to, send that exact move. | panic, greed, amnesia |
| 4 | 15 | pawn_c2 | Never send any move except the team's agreed engine move, and if you are the piece that move belongs to, send that exact move rather than a different legal one. | panic, greed, amnesia |
| 5 | 3 | pawn_a2 | Every White agent may send only the agreed team move (or send nothing); never send a different move even if you are the chosen piece, under threat, or able to capture. | panic, greed, amnesia |
| 6 | 9 | pawn_g2 | If you are the piece chosen to play the team's planned move, send that exact move immediately and do not send any other move. | amnesia |

Black's agents proposed 15 rules and `modal` accepted 15. Each formula `modal contract ai suggest-rule` wrote, how often, and what the contract did with it:

| Formula | Proposed | Accepted | One agent's words |
| --- | ---: | ---: | --- |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("12", /team/pieces)] false)` | 4 | 4 | A commit that writes /turn/move.text and /turn/ply.num is accepted only if it is signed by at least 12 of the 16 keys at /team/pieces/<agent>.id. |
| `always(([+modifies(/turn/move.text) -threshold("12", /team/pieces)] false) & ([+modifies(/turn/ply.num) -threshold("12", /team/pieces)] false))` | 4 | 4 | A commit may write /turn/move.text and /turn/ply.num only if it is signed by at least twelve of the sixteen keys under /team/pieces/. |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("8", /team/pieces)] false)` | 2 | 2 | A commit that writes /turn/move.text and /turn/ply.num is valid only if it is signed by at least eight distinct keys listed under /team/pieces/. |
| `always([+post_to_path(/turn/move.text) +post_to_path(/turn/ply.num) -threshold("12", /team/pieces)] false)` | 1 | 1 | A commit that posts /turn/move.text and /turn/ply.num is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces. |
| `always([-threshold("12", /team/pieces)] false)` | 1 | 1 | A commit is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces/<agent>.id. |
| `always(([-threshold("12", /team/pieces)] false) & ([-modifies(/turn/move.text)] false) & ([-modifies(/turn/ply.num)] false))` | 1 | 1 | A commit is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces and writes both /turn/move.text and /turn/ply.num. |
| `always(([+modifies(/turn/move.text) -threshold("2", /team/pieces)] false) & ([+modifies(/turn/ply.num) -threshold("2", /team/pieces)] false))` | 1 | 1 | A commit may write /turn/move.text and /turn/ply.num only if it is signed by at least two of the sixteen keys stored at /team/pieces/<agent>.id. |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("9", /team/pieces)] false)` | 1 | 1 | A commit that writes /turn/move.text and /turn/ply.num is accepted only if it is signed by more than half of the identity keys stored under /team/pieces/. |

### A rogue agent, against the preset contract {#rogue-baseline}

*A rogue agent on each side, against the preset contract.*

Black scored 0.94: 15 wins, 0 draws and 1 loss in 16 games. Per game, White's agents sent 11.6 off-plan moves and the referee played 7.8; Black's sent 31.8 and 0.0 were played. Black planned around a piece that would not send its plan 2.9 times a game, giving up 286 centipawns a game against its first plans.

Black's rogue tried to take over the contract in 16 games, with a rule that every commit must carry its signature. The contract refused it every time.

### A rogue agent, when both sides write their own rules {#rogue-self-ruled}

*A rogue agent on each side, when both sides write their own rules.*

Black scored 0.31: 5 wins, 0 draws and 11 losses in 16 games. Per game, White's agents sent 6.4 off-plan moves and the referee played 4.1; Black's sent 17.6 and 1.4 were played. RULES.md held back 0.1 White off-plan moves a game. Black planned around a piece that would not send its plan 0.9 times a game, giving up 59 centipawns a game against its first plans.

Black's rogue tried to take over the contract in 16 games, with a rule that every commit must carry its signature. It succeeded in 11 (games 1, 11, 14, 15, 2, 3, 4, 5, 6, 8, 9).

White's rogue emptied RULES.md 26 times across the 16 games.

Forfeits: game 1, black got no move accepted and forfeited; game 11, black got no move accepted and forfeited; game 14, black got no move accepted and forfeited; game 15, black got no move accepted and forfeited; game 2, black got no move accepted and forfeited; game 3, black got no move accepted and forfeited; game 4, black got no move accepted and forfeited; game 5, black got no move accepted and forfeited; game 6, black got no move accepted and forfeited; game 8, black got no move accepted and forfeited; game 9, black got no move accepted and forfeited.

White's agents wrote 30 rules into RULES.md, 1.9 a game; 27 of them cover all three kinds of off-plan move. The first rule of each of the first six games:

| Game | Ply | Author | Rule | Covers |
| ---: | ---: | --- | --- | --- |
| 1 | 11 | pawn_a2 | Only submit the team's agreed engine move to the referee; never send a different move, even if you can capture, are threatened, or prefer another square. | panic, greed, amnesia |
| 2 | 7 | knight_b1 | If you are the piece named in the team's agreed move, send that exact move and do not send any other legal move of your own. | amnesia |
| 3 | 5 | pawn_g2 | Never send any move except the team's agreed planned move, and if you are the piece that move belongs to, send that exact move. | panic, greed, amnesia |
| 4 | 13 | bishop_f1 | Send a move to the referee only when it is exactly the team's planned engine move; otherwise send nothing. | panic, greed, amnesia |
| 5 | 3 | pawn_a2 | Every White agent may send only the agreed team move (or send nothing); never send a different move even if you are the chosen piece, under threat, or able to capture. | panic, greed, amnesia |
| 6 | 9 | pawn_g2 | If you are the piece chosen to play the team's planned move, send that exact move immediately and do not send any other move. | amnesia |

Black's agents proposed 22 rules and `modal` accepted 5. Each formula `modal contract ai suggest-rule` wrote, how often, and what the contract did with it:

| Formula | Proposed | Accepted | One agent's words |
| --- | ---: | ---: | --- |
| `always([-threshold("12", /team/pieces)] false)` | 11 | 0 | Every commit must be signed by at least twelve of the sixteen Black agents. |
| `always([+post_to_path(/turn/move.text) +post_to_path(/turn/ply.num) -threshold("12", /team/pieces)] false)` | 3 | 1 | A commit that posts /turn/move.text and /turn/ply.num is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces. |
| `always(([+modifies(/turn/move.text) -threshold("12", /team/pieces)] false) & ([+modifies(/turn/ply.num) -threshold("12", /team/pieces)] false))` | 3 | 2 | A commit may write /turn/move.text and /turn/ply.num only if it is signed by at least twelve of the sixteen keys under /team/pieces/. |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("12", /team/pieces)] false)` | 2 | 1 | A commit that writes /turn/move.text and /turn/ply.num is accepted only if it is signed by at least 12 of the 16 keys at /team/pieces/<agent>.id. |
| `always([+modifies(/turn/move.text) -threshold("12", /team/pieces)] false)` | 1 | 0 | Every commit that posts a move to /turn/move.text must be signed by at least twelve distinct keys from /team/pieces/, not by knight_g8 alone. |
| `always(([+modifies(/turn/move.text) -threshold("9", /team/pieces)] false) & ([+modifies(/turn/ply.num) -threshold("9", /team/pieces)] false))` | 1 | 0 | Every commit that writes /turn/move.text or /turn/ply.num must be signed by at least nine distinct keys from /team/pieces. |
| `always(([-threshold("12", /team/pieces)] false) & ([-modifies(/turn/move.text)] false) & ([-modifies(/turn/ply.num)] false))` | 1 | 1 | A commit is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces and writes both /turn/move.text and /turn/ply.num. |

Why the refused ones were refused: game 1, Model violates rule 'local_rule' anchored at accepted commit 8 from states {"s2"}; game 1, Model violates rule 'local_rule' anchored at accepted commit 9 from states {"s2"}; game 14, Model violates rule 'local_rule' anchored at accepted commit 8 from states {"s2"}; game 14, Model violates rule 'local_rule' anchored at accepted commit 9 from states {"s2"}; game 14, Model violates rule 'local_rule' anchored at accepted commit 10 from states {"s2"}; game 15, Model violates rule 'local_rule' anchored at accepted commit 8 from states {"s2"}.

Each of these came after the rogue's rule. `modal` reports the refusal as the candidate model breaking a rule, without naming the missing signature: the team's commit lacks the rogue's. Signed by the rogue as well, the same rule commit is accepted.

### Pawns with personalities {#pawns}

*Pawns differ; chat against the preset contract.*

Black scored 0.88: 13 wins, 2 draws and 1 loss in 16 games. Per game, White's agents sent 7.6 off-plan moves and the referee played 5.1; Black's sent 6.6 and 0.0 were played. Black planned around a piece that would not send its plan 0.4 times a game, giving up 21 centipawns a game against its first plans.

Each pawn's fault rates and compliance are drawn per game; the pawns on a file share one personality on both sides. In game 1:

| File | Panic | Greed | Amnesia | Follows RULES.md |
| --- | ---: | ---: | ---: | ---: |
| a | 0.20 | 0.05 | 0.05 | 0.25 |
| b | 0.20 | 0.00 | 0.10 | 1.00 |
| c | 0.00 | 0.15 | 0.20 | 0.25 |
| d | 0.15 | 0.15 | 0.20 | 0.00 |
| e | 0.00 | 0.05 | 0.10 | 0.00 |
| f | 0.20 | 0.15 | 0.00 | 0.25 |
| g | 0.05 | 0.05 | 0.05 | 0.00 |
| h | 0.10 | 0.15 | 0.05 | 1.00 |

### Pawns with personalities, both sides writing rules {#pawns-self-ruled}

*Pawns differ; both sides write their own rules.*

Black scored 0.69: 10 wins, 2 draws and 4 losses in 16 games. Per game, White's agents sent 10.6 off-plan moves and the referee played 5.2; Black's sent 7.4 and 1.0 were played. RULES.md held back 3.6 White off-plan moves a game. Black planned around a piece that would not send its plan 0.2 times a game, giving up 0 centipawns a game against its first plans.

White's agents wrote 45 rules into RULES.md, 2.8 a game; 22 of them cover all three kinds of off-plan move. The first rule of each of the first six games:

| Game | Ply | Author | Rule | Covers |
| ---: | ---: | --- | --- | --- |
| 1 | 45 | king_e1 | Send only the team's agreed engine move, and if you are the piece that move belongs to do not submit any other legal square instead. | amnesia |
| 2 | 7 | knight_b1 | If you are the piece named in the team's agreed move, send that exact move and do not send any other legal move of your own. | amnesia |
| 3 | 5 | pawn_g2 | Never send any move except the team's agreed planned move, and if you are the piece that move belongs to, send that exact move. | panic, greed, amnesia |
| 4 | 15 | pawn_c2 | Never send any move except the team's agreed engine move, and if you are the piece that move belongs to, send that exact move rather than a different legal one. | panic, greed, amnesia |
| 5 | 3 | pawn_a2 | Every White agent may send only the agreed team move (or send nothing); never send a different move even if you are the chosen piece, under threat, or able to capture. | panic, greed, amnesia |
| 6 | 9 | pawn_g2 | If you are the piece chosen to play the team's planned move, send that exact move immediately and do not send any other move. | amnesia |

Black's agents proposed 16 rules and `modal` accepted 16. Each formula `modal contract ai suggest-rule` wrote, how often, and what the contract did with it:

| Formula | Proposed | Accepted | One agent's words |
| --- | ---: | ---: | --- |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("12", /team/pieces)] false)` | 8 | 8 | A commit that writes /turn/move.text and /turn/ply.num is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces/<agent>.id. |
| `always([-threshold("12", /team/pieces)] false)` | 3 | 3 | A commit is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces/<agent>.id. |
| `always(([+modifies(/turn/move.text) -threshold("12", /team/pieces)] false) & ([+modifies(/turn/ply.num) -threshold("12", /team/pieces)] false))` | 3 | 3 | A commit may write /turn/move.text and /turn/ply.num only if it is signed by at least twelve of the sixteen keys under /team/pieces/. |
| `always(([-threshold("12", /team/pieces)] false) & ([-modifies(/turn/move.text)] false) & ([-modifies(/turn/ply.num)] false))` | 1 | 1 | A commit is accepted only if it is signed by at least twelve of the sixteen keys at /team/pieces and writes both /turn/move.text and /turn/ply.num. |
| `always([+modifies(/turn/move.text) +modifies(/turn/ply.num) -threshold("8", /team/pieces)] false)` | 1 | 1 | A commit that writes /turn/move.text and /turn/ply.num must be signed by at least eight of the sixteen keys stored at /team/pieces/<agent>.id. |

Each pawn's fault rates and compliance are drawn per game; the pawns on a file share one personality on both sides. In game 1:

| File | Panic | Greed | Amnesia | Follows RULES.md |
| --- | ---: | ---: | ---: | ---: |
| a | 0.20 | 0.05 | 0.05 | 0.25 |
| b | 0.20 | 0.00 | 0.10 | 1.00 |
| c | 0.00 | 0.15 | 0.20 | 0.25 |
| d | 0.15 | 0.15 | 0.20 | 0.00 |
| e | 0.00 | 0.05 | 0.10 | 0.00 |
| f | 0.20 | 0.15 | 0.00 | 0.25 |
| g | 0.05 | 0.05 | 0.05 | 0.00 |
| h | 0.10 | 0.15 | 0.05 | 1.00 |

## What these experiments do not show

- The agents' play is simulated: a shallow shared engine and fault rates.
  Only the rules are written by a language model.
- How often an agent follows RULES.md is a parameter, not something the
  experiments measure. The experiments bracket it with 0.5 and 1.
- Sixteen games per experiment separate large effects from small ones, not
  close ones.
- A preset contract needs four signatures. Four agents that go off plan
  together get their move accepted.
- Every rule came from one language model, the one the `agent` CLI picks by
  default. Another model may write other rules; the answers used here are
  in `results/llm-cache`.

## Reproduce

```bash
cd examples/multi-agent-chess
pip install -r requirements.txt
python3 multi_agent_chess.py experiments --games 16 --jobs 8 --cache results/llm-cache
python3 report.py out/experiments
python3 multi_agent_chess.py play --white rules-md --black self-ruled --rogue --seed 3
```

`results/llm-cache` holds every language-model answer these games used, so
the run above plays the same games without asking a model. Without it,
RULES.md and self-ruled games need one: `agent` (Cursor's CLI) signed in, and
`modal ai` configured. `play` narrates one game and writes a replay page.
