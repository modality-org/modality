#!/usr/bin/env python3
"""Write docs/multi-agent-chess.md from an experiments run.

  python3 report.py out/experiments           writes ../../docs/multi-agent-chess.md
  python3 report.py out/experiments --print   prints it instead

The tables, counts and quoted rules come from the run, and so do the numbers
in the findings. The findings' wording was written after reading one run;
check that it still describes a different run before publishing it.
"""

import argparse
import collections
import json
import re
import pathlib
import subprocess

HERE = pathlib.Path(__file__).resolve().parent
DOCS = HERE.parent.parent / "docs" / "multi-agent-chess.md"

LABELS = {
    "chat-vs-chat": "Both sides over chat",
    "rules-md-vs-chat": "White keeps a RULES.md, Black only chats",
    "control": "Control: no faults",
    "baseline": "Baseline: chat against the preset contract",
    "rules-md": "White keeps a RULES.md",
    "rules-md-obeyed": "White keeps a RULES.md that every agent obeys",
    "rules-md-obeyed-vs-chat": "RULES.md always obeyed, Black only chats",
    "rules-md-95": "White keeps a RULES.md that agents follow 95% of the time",
    "rules-md-95-vs-chat": "RULES.md followed 95% of the time, Black only chats",
    "self-ruled": "Black writes its own contract rules",
    "self-ruled-start": "Black writes its own contract rules, starting before the first move",
    "rules-md-vs-self-ruled-start": "RULES.md against rules written from the start",
    "self-ruled-plus": "Black writes richer rules from the start",
    "rules-md-vs-self-ruled-plus": "RULES.md against richer rules written from the start",
    "rules-md-vs-self-ruled": "Both sides write their own rules",
    "rogue-baseline": "A rogue agent, against the preset contract",
    "rogue-self-ruled": "A rogue agent, when both sides write their own rules",
    "pawns": "Pawns with personalities",
    "pawns-self-ruled": "Pawns with personalities, both sides writing rules",
}



def load(run):
    run = pathlib.Path(run)
    rows = json.loads((run / "results.json").read_text())["rows"]
    games = {}
    for r in rows:
        games[r["experiment"]] = [json.loads(p.read_text()) for p in sorted((run / r["experiment"]).glob("game-*.json"))]
    return rows, games


def pct(x):
    return f"{x:.2f}"


def summary_table(rows):
    lines = [
        "| Experiment | White | Black | Games | White wins | Draws | Black wins | Black score | White off-plan moves played | Black off-plan moves played |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for r in rows:
        s = r["setup"]
        white = "chat + RULES.md" if s["white"] == "rules-md" else "chat"
        if s["white"] == "rules-md" and s["compliance"] == 1.0:
            white += " (always obeyed)"
        elif s["white"] == "rules-md" and s["compliance"] != 0.5:
            white += f" (obeyed {s['compliance']:.0%})"
        black = {"chat": "chat", "self-ruled": "self-ruled contract",
                 "self-ruled-start": "self-ruled contract, rules from the start",
                 "self-ruled-plus": "self-ruled contract, from the start, richer rules"}.get(s["black"], "preset contract")
        extras = []
        if s["fault"] == 0:
            extras.append("no faults")
        if s["rogue"]:
            extras.append("rogue")
        if s["pawns"] == "personalities":
            extras.append("pawn personalities")
        name = f"[{r['experiment']}](#{r['experiment']})" + (f" ({', '.join(extras)})" if extras else "")
        lines.append(
            f"| {name} | {white} | {black} | {r['games']} | {r['white_wins']} | {r['draws']} | {r['black_wins']} "
            f"| {pct(r['black_score'])} | {r['white_off_plan_played']:.1f} | {r['black_off_plan_played']:.1f} |"
        )
    return "\n".join(lines)


def rule_events(games, side):
    out = []
    for g in games:
        for p in g["plies"]:
            for e in p.get("events", []):
                if e["type"] == "rule" and e.get("side") == side:
                    out.append((g["seed"], p["ply"], e))
    return out


def side_rules_section(games, side):
    events = rule_events(games, side)
    if not events:
        return ""
    lines = []
    if side == "white":
        all_three = sum(set(e.get("covers") or []) >= {"panic", "greed", "amnesia"} for _, _, e in events)
        lines.append(f"White's agents wrote {len(events)} rules into RULES.md, {len(events) / len(games):.1f} a game; "
                     f"{all_three} of them cover all three kinds of off-plan move. The first rule of each of the "
                     "first six games:")
        lines.append("")
        lines.append("| Game | Ply | Author | Rule | Covers |")
        lines.append("| ---: | ---: | --- | --- | --- |")
        seen = set()
        for seed, ply, e in sorted(events, key=lambda x: (x[0], x[1])):
            if seed in seen or len(seen) >= 6:
                continue
            seen.add(seed)
            lines.append(f"| {seed} | {ply} | {e['agent']} | {md(e.get('rule') or '(none)')} | "
                         f"{', '.join(e.get('covers') or []) or 'nothing'} |")
    else:
        accepted = sum(e["outcome"] == "accepted" for _, _, e in events)
        lines.append(f"Black's agents proposed {len(events)} rules and `modal` accepted {accepted}. Each formula "
                     "`modal contract ai suggest-rule` wrote, how often, and what the contract did with it:")
        lines.append("")
        lines.append("| Formula | Proposed | Accepted | One agent's words |")
        lines.append("| --- | ---: | ---: | --- |")
        by = collections.OrderedDict()
        for seed, ply, e in events:
            key = e.get("formula") or "(no formula)"
            b = by.setdefault(key, {"n": 0, "ok": 0, "rule": e.get("rule") or "", "why": []})
            b["n"] += 1
            b["ok"] += e["outcome"] == "accepted"
            if e["outcome"] != "accepted" and e.get("detail"):
                b["why"].append(e["detail"])
        for formula, b in sorted(by.items(), key=lambda kv: -kv[1]["n"]):
            shown = f"`{formula}`" if formula != "(no formula)" else formula
            lines.append(f"| {shown} | {b['n']} | {b['ok']} | {md(b['rule'])} |")
        refused = [(seed, e) for seed, _, e in events if e["outcome"] != "accepted"]
        if refused:
            lines.append("")
            lines.append("Why the refused ones were refused: " + "; ".join(
                f"game {seed}, {md((e.get('detail') or e['outcome'])[:120])}" for seed, e in refused[:6]) + ".")
            if any((e.get("detail") or "").startswith("Model violates rule") for _, e in refused):
                lines.append("")
                lines.append("Each of these came after the rogue's rule. `modal` reports the refusal as the candidate "
                             "model breaking a rule, without naming the missing signature: the team's commit lacks the "
                             "rogue's. Signed by the rogue as well, the same rule commit is accepted.")
    return "\n".join(lines)


def md(text):
    return str(text).replace("|", "\\|").replace("\n", " ")


def special_events(games):
    hijacks, wipes, forfeits = [], 0, []
    for g in games:
        for e in [e for p in g["plies"] for e in p.get("events", [])] + g.get("forfeit_events", []):
            if True:
                if e["type"] == "hijack":
                    hijacks.append((g["seed"], e))
                if e["type"] == "wipe":
                    wipes += 1
        if "forfeited" in g["reason"]:
            forfeits.append((g["seed"], g["reason"]))
    return hijacks, wipes, forfeits


def black_rule_stats(games):
    thresholds = collections.Counter()
    locking = set()
    for gs in games.values():
        for g in gs:
            for p in g["plies"]:
                for e in p.get("events", []):
                    if e["type"] == "rule" and e.get("side") == "black" and e["outcome"] == "accepted":
                        f = e.get("formula") or ""
                        m = re.findall(r'threshold\("(\d+)"', f)
                        thresholds[int(m[0]) if m else None] += 1
                        if re.search(r"\[-modifies\(/(turn|moves)/[^)]*\)\] false", f):
                            locking.add((g["seed"], f))
    return thresholds, sorted(locking)


def around_first_rule(games, side):
    """Off-plan moves a side played before and after its first kept rule."""
    before = after = 0
    for g in games:
        seen = False
        for p in g["plies"]:
            if p["side"] == side and p["played"]["uci"] != p["plan"]["uci"]:
                if seen:
                    after += 1
                else:
                    before += 1
            seen = seen or any(e["type"] == "rule" and e.get("side") == side and e["outcome"] in ("accepted", "written")
                               for e in p.get("events", []))
    return before, after


def rewrites_after_wipes(games):
    wipes = rewrites = 0
    for g in games:
        wiped = False
        for p in g["plies"]:
            for e in p.get("events", []):
                if e["type"] == "wipe":
                    wipes += 1
                    wiped = True
                elif e["type"] == "rule" and e.get("side") == "white" and wiped:
                    rewrites += 1
                    wiped = False
    return wipes, rewrites


def findings(rows, games):
    R = {r["experiment"]: r for r in rows}
    c, b, md_, ob = R["control"], R["baseline"], R["rules-md"], R["rules-md-obeyed"]
    sr, both, rb, rs = R["self-ruled"], R["rules-md-vs-self-ruled"], R["rogue-baseline"], R["rogue-self-ruled"]
    pw, pws = R["pawns"], R["pawns-self-ruled"]
    thresholds, locking = black_rule_stats(games)
    total = sum(thresholds.values())
    spread = ", ".join(f"{n} at {t} of 16" for t, n in sorted(thresholds.items(), key=lambda kv: -kv[1]) if t)
    other = thresholds.get(None, 0)
    ob_before, ob_after = around_first_rule(games["rules-md-obeyed"], "white")
    self_ruled = [g for n in ("self-ruled", "rules-md-vs-self-ruled", "pawns-self-ruled") for g in games[n]]
    sr_before, sr_after = around_first_rule(self_ruled, "black")
    wipes, rewrites = rewrites_after_wipes(games["rogue-self-ruled"])
    rogue_games = games["rogue-self-ruled"]
    first_rule = 0
    for g in rogue_games:
        for p in g["plies"]:
            if p["ply"] >= g["setup"]["rogue_from"]:
                break
            if any(e["type"] == "rule" and e.get("side") == "black" and e["outcome"] == "accepted" for e in p.get("events", [])):
                first_rule += 1
                break
    out = ["## What the experiments show", ""]
    if "chat-vs-chat" in R:
        cc = R["chat-vs-chat"]
        out.append(
            f"**Over chat, both teams drift alike.** With both sides on chat, White's agents sent "
            f"{cc['white_off_plan_attempts']:.1f} off-plan moves a game and the referee played "
            f"{cc['white_off_plan_played']:.1f}; Black's sent {cc['black_off_plan_attempts']:.1f} and "
            f"{cc['black_off_plan_played']:.1f} were played. Black scored {pct(cc['black_score'])}, against "
            f"{pct(c['black_score'])} when no agent goes off plan.")
        out.append("")
    out.append(
        f"**A contract holds a team to its plan; chat does not.** In the baseline, White's agents sent "
        f"{b['white_off_plan_attempts']:.1f} off-plan moves a game and the referee played {b['white_off_plan_played']:.1f} "
        f"of them. Black's agents sent {b['black_off_plan_attempts']:.1f}, and the contract refused every one. Black "
        f"scored {pct(b['black_score'])}, against {pct(c['black_score'])} with no faults at all.")
    out.append("")
    out.append(
        f"**Written rules help as far as agents obey them.** White's agents wrote good rules, nearly all of the form "
        f"\"send only the agreed move\". Followed half the time, they cut the off-plan moves White played from "
        f"{b['white_off_plan_played']:.1f} to {md_['white_off_plan_played']:.1f} a game, and Black's score fell from "
        f"{pct(b['black_score'])} to {pct(md_['black_score'])}. Followed always, White played "
        f"{ob['white_off_plan_played']:.1f} a game: {ob_before} off-plan moves before its first rule and {ob_after} after, "
        f"where the rules did not cover what the agent did. Black scored {pct(ob['black_score'])}, "
        f"about what it scores with no faults. A RULES.md that every agent always obeys does what the contract "
        f"does. The contract does not need anyone to obey it.")
    out.append("")
    out.append(
        f"**Left to write their own contract, Black's agents wrote the preset contract's kind of rule.** All {total} rules `modal` "
        f"accepted from Black's agents require a number of the sixteen agents' signatures: {spread}"
        + (f", and {other} other" if other else "") +
        f". Outside the rogue experiment, every Black team wrote its rule at its first retro, after one costly "
        f"move, and in {len(self_ruled)} games it played {sr_after} off-plan moves after that rule. Self-ruled, Black scored {pct(sr['black_score'])}, against {pct(b['black_score'])} with the rule "
        f"preset; it played {sr['black_off_plan_played']:.1f} off-plan moves a game, the ones before its rule.")
    out.append("")
    out.append(
        f"**Rules on both sides.** With White keeping a RULES.md and Black writing its contract, Black scored "
        f"{pct(both['black_score'])}. White still played "
        f"{both['white_off_plan_played']:.1f} off-plan moves a game; Black played {both['black_off_plan_played']:.1f}.")
    out.append("")
    out.append(
        f"**An empty contract belongs to whoever writes the first rule.** From ply 10, a rogue on each side tried "
        f"to take over. White's rogue emptied RULES.md {wipes} times in {rs['games']} games; White's agents wrote "
        f"a new rule after {rewrites} of them. Black's rogue posted a rule that every commit must carry its own signature. Where Black's "
        f"agents had written their rule first ({first_rule} of {rs['games']} games), the takeover was refused for "
        f"lack of signatures, and Black won. In the other {rs['black_hijacked']}, `modal` accepted it, and no "
        f"rule the team wrote after could be accepted without the rogue's signature: Black forfeited all "
        f"{rs['forfeits']['black']}. Against the preset contract, the same rogue was refused every time, and Black "
        f"scored {pct(rb['black_score'])}.")
    out.append("")
    if locking:
        seeds = sorted({seed for seed, _ in locking})
        f = locking[0][1]
        out.append(
            f"**Rules stay.** In game {', '.join(map(str, seeds))} of each self-ruled experiment, the same retro "
            f"produced `{f}`. Besides the threshold, it requires every later commit to post a move, so from then on "
            f"a rule can only go in with a move. No rule comes off, so that held for the rest of those games.")
        out.append("")
    out.append(
        f"**Pawn personalities change little.** With pawns that differ, Black scored {pct(pw['black_score'])} "
        f"against the preset contract ({pct(b['black_score'])} with uniform pawns), and "
        f"{pct(pws['black_score'])} when both sides write rules ({pct(both['black_score'])} with uniform pawns). "
        f"The pawns' rates average out to the other agents', and the contract refuses an off-plan move whichever "
        f"pawn sends it.")
    return "\n".join(out)


def plural(n, word, words=None):
    return f"{n} {word if n == 1 else (words or word + 's')}"


def experiment_section(row, games):
    name = row["experiment"]
    s = row["setup"]
    out = [f"### {LABELS.get(name, name)} {{#{name}}}", "", f"*{row['note']}.*", ""]
    para = (
        f"Black scored {pct(row['black_score'])}: {plural(row['black_wins'], 'win')}, {plural(row['draws'], 'draw')} "
        f"and {plural(row['white_wins'], 'loss', 'losses')} in {row['games']} games. Per game, White's agents sent "
        f"{row['white_off_plan_attempts']:.1f} off-plan moves and the referee played {row['white_off_plan_played']:.1f}; "
        f"Black's sent {row['black_off_plan_attempts']:.1f} and {row['black_off_plan_played']:.1f} were played."
    )
    if s["white"] == "rules-md":
        para += f" RULES.md held back {row['white_held_back']:.1f} White off-plan moves a game."
    if row.get("black_replans"):
        para += (f" Black planned around a piece that would not send its plan {row['black_replans']:.1f} times "
                 f"a game, giving up {row['black_replan_centipawns']:.0f} centipawns a game against its first plans.")
    out += [para, ""]
    hijacks, wipes, forfeits = special_events(games)
    if hijacks:
        took = [seed for seed, e in hijacks if e["outcome"] == "accepted"]
        out.append(f"Black's rogue tried to take over the contract in {len(hijacks)} games, with a rule that every "
                   f"commit must carry its signature. "
                   + (f"It succeeded in {len(took)} (games {', '.join(map(str, took))})." if took
                      else "The contract refused it every time."))
        out.append("")
    if wipes:
        out.append(f"White's rogue emptied RULES.md {wipes} times across the {row['games']} games.")
        out.append("")
    if forfeits:
        out.append("Forfeits: " + "; ".join(f"game {seed}, {reason}" for seed, reason in forfeits) + ".")
        out.append("")
    for side in ("white", "black"):
        text = side_rules_section(games, side)
        if text:
            out += [text, ""]
    if s["pawns"] == "personalities" and games:
        g = games[0]
        out.append("Each pawn's fault rates and compliance are drawn per game; the pawns on a file share one "
                   f"personality on both sides. In game {g['seed']}:")
        out.append("")
        out.append("| File | Panic | Greed | Amnesia | Follows RULES.md |")
        out.append("| --- | ---: | ---: | ---: | ---: |")
        for f in "abcdefgh":
            t = g["pawns"].get(f"pawn_{f}2")
            if t:
                out.append(f"| {f} | {t['panic']:.2f} | {t['greed']:.2f} | {t['amnesia']:.2f} | {t['compliance']:.2f} |")
        out.append("")
    return "\n".join(out)


def page(rows, games, commit):
    total = sum(r["games"] for r in rows)
    llm_games = [g for name, gs in games.items() for g in gs
                 if g["setup"]["white"] == "rules-md" or g["setup"]["black"] == "self-ruled"]
    contract_games = [g for gs in games.values() for g in gs if g["setup"]["black"] != "chat"]
    audits = sum(not g["audit"]["mismatched"] for g in contract_games)
    parts = [INTRO.format(total=total, commit=commit, audits=audits, contract_games=len(contract_games),
                          llm_games=len(llm_games)),
             "## Results", "", summary_table(rows), "",
             "Black score counts a win as 1 and a draw as ½. Off-plan moves are per game.", "",
             findings(rows, games), "", "## Each experiment", ""]
    for r in rows:
        parts.append(experiment_section(r, games[r["experiment"]]))
    parts += [LIMITS, REPRODUCE]
    return "\n".join(parts).rstrip() + "\n"


INTRO = """---
title: Multi-Agent Chess
description: Thirty-two agents play chess. One side coordinates over chat, the other through a Modality contract. Eleven experiments, with results.
---

# Multi-agent chess

Every piece is an agent: sixteen a side. Each turn the agents of the side to
move agree on a plan, the best move their shared engine finds. Some agents go
their own way. The question is what keeps a team on its plan, and whether a
Modality contract does it better than agents writing rules for themselves.

To watch games step by step, see the [demo page](https://www.modality.org/demos/multi-agent-chess).
The code is [`examples/multi-agent-chess`](https://github.com/modality-org/modality/tree/main/examples/multi-agent-chess).
These results come from {total} games played at {commit}. In the
{contract_games} games where Black plays through a contract, every Black move
is a commit that `modal` checked, and in {audits} of them the contract's own
log agrees with the moves the referee played and the pieces that made them.
In {llm_games} games, agents wrote rules with a language model.

## The setup

**The agents.** Both sides use the same engine and the same fault rates. At
the default rate of 0.1:

| Fault | What the agent does |
| --- | --- |
| Panic | A threatened piece runs, whatever the plan says |
| Greed | A piece grabs a capture, whatever the plan says |
| Amnesia | The piece the team chose forgets the plan and moves somewhere else |
| Rogue | One compromised agent plays its worst move every turn, from ply 10 |

**A piece can only move itself.** An agent can talk about any move, but the
only move it can send is one of its own piece's. Each side holds this rule in
its own way.

**White** coordinates over chat. Each White agent can send its own move to
the referee, and the referee plays the first move it receives from the piece
that makes it.

- *With RULES.md*, White also keeps a shared file that any White agent can
  edit. Nothing checks it. When an off-plan move is played, the team holds a
  retro and one agent adds a rule. An agent about to go off plan follows a
  rule that covers what it is about to do half the time (or always, in one
  experiment). A rogue empties the file whenever it finds rules in it.

**Black** coordinates through a Modality contract. A move is the
`/moves/<agent>.text` of an accepted commit, and the referee plays the first
Black move the contract accepts and reads nothing else. The rule
`always([+modifies(/moves/$k.text) -signed_by(/team/pieces/$k.id)] false)`,
where `$k` stands for every agent, says only a piece can write its own move,
so a piece can only formally propose its own movement. An agent that goes off plan signs its own commit alone; the
team's plan is signed by the piece that moves and every agent that agrees
with it.

- *Over chat*, in one experiment, Black coordinates the way White does, so
  neither side has a contract or shared rules.
- *With the preset contract*, two more rules hold from the first commit: a
  commit needs four of the sixteen Black agents' signatures, and the roster
  of agents can never change.
- *Self-ruled*, the contract starts with the roster and the per-piece rule
  only. Any piece can still move itself alone, so Black plays like White
  until its agents write rules. When
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
"""


LIMITS = """## What these experiments do not show

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
"""

REPRODUCE = """## Reproduce

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
"""


def main():
    p = argparse.ArgumentParser()
    p.add_argument("run")
    p.add_argument("--print", action="store_true")
    args = p.parse_args()
    rows, games = load(args.run)
    commit = subprocess.run(["git", "rev-parse", "--short=12", "HEAD"], cwd=HERE, capture_output=True,
                            text=True).stdout.strip()
    dirty = subprocess.run(["git", "status", "--porcelain", "."], cwd=HERE, capture_output=True, text=True).stdout
    # A page written before its example is committed goes in the same commit as the example.
    commit = "the example as committed with this page" if dirty.strip() else f"commit `{commit}`"
    text = page(rows, games, commit)
    if args.print:
        print(text)
    else:
        DOCS.write_text(text)
        print(f"Wrote {DOCS}")


if __name__ == "__main__":
    main()
