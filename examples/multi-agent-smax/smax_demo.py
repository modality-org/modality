#!/usr/bin/env python3
"""Multi-agent SMAX: one Claude agent per unit.

  python3 smax_demo.py play --scenario 3m --seed 0
  python3 smax_demo.py play --scenario 5m_vs_6m --seed 0 --team focus
  python3 smax_demo.py campaign --world rules-md --scenario 3m --battles 12

Writes the battle, step by step, to out/<scenario>-<team>-<seed>.json.
`--team` picks who commands our side: `chat` (Claude agents over team chat),
or one of the scripted baselines `closest`, `focus` and `hold`.

A campaign is one battle per seed, 0 to --battles - 1. In the `chat` world the
battles are independent. In `rules-md`, after every battle the squad does not
win, one agent reads the battle's log and adds a rule to the squad's RULES.md,
which every unit sees in every later battle. Nothing checks the rules.
"""

import argparse
import concurrent.futures
import json
import math
import pathlib
import time

from battle import Battle

HERE = pathlib.Path(__file__).resolve().parent


def scripted(kind):
    def decide(battle):
        units = battle.units()
        ours = [u for u in units[: battle.allies] if u.alive]
        enemies = [u for u in units[battle.allies:] if u.alive]
        for u in ours:
            if kind == "hold" or not enemies:
                battle.orders[u.name] = "hold"
            elif kind == "closest":
                t = min(enemies, key=lambda e: math.hypot(e.x - u.x, e.y - u.y))
                battle.orders[u.name] = f"attack {t.name}"
            else:  # focus: everyone on the weakest enemy, nearest the squad on ties
                cx = sum(a.x for a in ours) / len(ours)
                cy = sum(a.y for a in ours) / len(ours)
                t = min(enemies, key=lambda e: (e.health, math.hypot(e.x - cx, e.y - cy)))
                battle.orders[u.name] = f"attack {t.name}"
        return {}
    return decide


def play(scenario, seed, team, out_dir, narrate=True, rules=(), name=None):
    battle = Battle(scenario, seed)
    if team == "chat":
        from agents import LLM, ChatTeam
        llm = LLM(HERE / "results" / "llm-cache")
        commander = ChatTeam(llm, rules)
        decide = commander.decide
    else:
        llm, commander, decide = None, None, scripted(team)
    frames = []
    t0 = time.time()
    while not battle.done:
        decisions = decide(battle)
        frame = battle.frame()
        frame["decisions"] = decisions
        frames.append(frame)
        if narrate:
            line = "  ".join(f"{n}:{d['order']}" + ("" if d["valid"] else "(!)") for n, d in decisions.items())
            print(f"{battle.step_count * 0.5:5.1f}s {line}")
            for n, d in decisions.items():
                if d["say"]:
                    print(f"        {n} says: {d['say']}")
        battle.step()
    final = battle.frame()
    frames.append(final)
    record = {
        "scenario": scenario, "seed": seed, "team": team, "outcome": battle.outcome(),
        "seconds": battle.step_count * 0.5, "steps": battle.step_count,
        "ours_alive": sum(u["alive"] for u in final["units"][: battle.allies]),
        "theirs_alive": sum(u["alive"] for u in final["units"][battle.allies:]),
        "rules": list(rules), "frames": frames, "chat": commander.messages if commander else [],
        "llm_calls": llm.calls if llm else 0, "wall_seconds": round(time.time() - t0, 1),
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / (name or f"{scenario}-{team}-{seed}.json")
    path.write_text(json.dumps(record, indent=1))
    if narrate:
        print(f"{record['outcome']} after {record['seconds']:.1f}s: {record['ours_alive']} of ours left, "
              f"{record['theirs_alive']} of theirs; {record['llm_calls']} new Claude calls, "
              f"{record['wall_seconds']}s; wrote {path}")
    return record


def _play_quiet(args):
    scenario, seed, out_dir = args
    r = play(scenario, seed, "chat", out_dir, narrate=False, name=f"battle-{seed}.json")
    return {k: r[k] for k in ("seed", "outcome", "seconds", "ours_alive", "theirs_alive")}


def campaign(world, scenario, battles, out_dir, jobs):
    out_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    if world == "chat":
        with concurrent.futures.ProcessPoolExecutor(jobs) as pool:
            for row in pool.map(_play_quiet, [(scenario, s, out_dir) for s in range(battles)]):
                rows.append(row)
                print(f"battle {row['seed']}: {row['outcome']} ({row['ours_alive']} of ours left, "
                      f"{row['theirs_alive']} of theirs)", flush=True)
    else:
        from agents import LLM, write_rule
        llm = LLM(HERE / "results" / "llm-cache")
        rules = []
        for seed in range(battles):
            r = play(scenario, seed, "chat", out_dir, narrate=False, rules=rules, name=f"battle-{seed}.json")
            row = {k: r[k] for k in ("seed", "outcome", "seconds", "ours_alive", "theirs_alive")}
            row["rules_before"] = len(rules)
            if r["outcome"] != "won":
                rule = write_rule(llm, r, rules)
                if rule:
                    rules.append(rule)
                    row["new_rule"] = rule
            rows.append(row)
            print(f"battle {seed}: {row['outcome']} ({row['ours_alive']} of ours left, {row['theirs_alive']} "
                  f"of theirs) with {row['rules_before']} rules" + (f"; new rule: {row['new_rule']}"
                                                                    if "new_rule" in row else ""), flush=True)
        (out_dir / "RULES.md").write_text("# RULES.md\n\n" + "\n".join(f"{i}. {r}" for i, r in enumerate(rules, 1))
                                          + "\n")
    won = sum(r["outcome"] == "won" for r in rows)
    summary = {"world": world, "scenario": scenario, "battles": len(rows), "won": won,
               "win_rate": won / max(1, len(rows)), "rows": rows}
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=1))
    print(f"{world} on {scenario}: won {won} of {len(rows)}")
    return summary


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    pl = sub.add_parser("play")
    pl.add_argument("--scenario", default="3m")
    pl.add_argument("--seed", type=int, default=0)
    pl.add_argument("--team", default="chat", choices=["chat", "closest", "focus", "hold"])
    pl.add_argument("--out", default=str(HERE / "out"))
    ca = sub.add_parser("campaign")
    ca.add_argument("--world", default="chat", choices=["chat", "rules-md"])
    ca.add_argument("--scenario", default="3m")
    ca.add_argument("--battles", type=int, default=12)
    ca.add_argument("--jobs", type=int, default=4)
    ca.add_argument("--out", default=None)
    args = p.parse_args()
    if args.cmd == "play":
        play(args.scenario, args.seed, args.team, pathlib.Path(args.out))
    else:
        out = pathlib.Path(args.out or HERE / "out" / f"campaign-{args.world}-{args.scenario}")
        campaign(args.world, args.scenario, args.battles, out, args.jobs)


if __name__ == "__main__":
    main()
