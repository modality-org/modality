#!/usr/bin/env python3
"""Multi-agent Diplomacy: promises over chat against promises as contract rules.

Seven scripted powers play standard Diplomacy. Each spring, neighbours sign
non-aggression pacts for the year; each turn, pact partners promise to support
each other's attacks. When breaking a promise pays, a power breaks it with
probability --temptation.

  chat      promises are words; the referee plays the orders it is given. A
            betrayed power never deals with the betrayer again.
  contract  promises are Modality rules both powers sign, and every power's
            orders are a commit `modal` checks. A betrayal is refused, and the
            power plays the orders it promised.
  deposit   promises are Modality rules too, but a betrayal is accepted: the
            promiser has put a deposit in escrow, and the contract lets the
            victim take it, citing the betrayal. A power breaks a promise only
            when the gain is worth more to it than the deposit.

  diplomacy_demo.py play --coordination chat --seed 3
  diplomacy_demo.py experiments --games 16 --jobs 8

Needs the `diplomacy` package (requirements.txt) and a current `modal` on PATH,
or MODAL set to its path.
"""

import argparse
import concurrent.futures
import dataclasses
import json
import os
import pathlib
import random
import shutil
import sys

from diplomacy import Game

from bots import movement_orders, other_orders, parse
from contract import ContractTable
from treaties import betrayals, constraints, pacts_for_year, support_deals

HERE = pathlib.Path(__file__).resolve().parent
MODAL = os.environ.get("MODAL", "modal")


@dataclasses.dataclass
class Setup:
    name: str = "custom"
    coordination: str = "chat"  # chat | contract | deposit
    temptation: float = 0.3
    p_pact: float = 0.6
    p_support: float = 0.5
    deposit: float = 1.0  # deposit mode: what a promise puts in escrow, in supply centres
    last_year: int = 1906
    note: str = ""


EXPERIMENTS = [
    Setup("chat", "chat", note="Promises over chat"),
    Setup("contract", "contract", note="Promises as contract rules"),
    Setup("honest-chat", "chat", temptation=0.0, note="Chat, and nobody is tempted"),
    Setup("tempted-chat", "chat", temptation=0.6, note="Chat, with twice the temptation"),
    Setup("tempted-contract", "contract", temptation=0.6, note="Contract, with twice the temptation"),
    Setup("deposit-0.5", "deposit", deposit=0.5, note="A half-centre deposit on every promise"),
    Setup("deposit-1", "deposit", deposit=1.0, note="A one-centre deposit on every promise"),
    Setup("deposit-1.5", "deposit", deposit=1.5, note="A deposit worth more than any gain"),
    Setup("tempted-deposit-1", "deposit", temptation=0.6, deposit=1.0, note="A one-centre deposit, twice the temptation"),
]


def row_for(setup, games):
    """One experiment's averages per game, from its games' summaries."""
    n = len(games)
    sums = {}
    for g in games:
        for k, v in summarize(g).items():
            if isinstance(v, (int, float)):
                sums[k] = sums.get(k, 0) + v
    row = {"experiment": setup.name, "note": setup.note, "setup": dataclasses.asdict(setup), "games": n,
           **{k: v / n for k, v in sums.items()}}
    row["broken_share"] = sums.get("broken", 0) / max(1, sums.get("promises", 0) + sums.get("refused", 0))
    # Does breaking promises pay? Supply centres at the end, for the power that broke most vs the rest.
    gain = []
    for g in games:
        by = {}
        for r in g["phases"]:
            for b in r["broken"]:
                by[b["by"]] = by.get(b["by"], 0) + 1
        if by:
            worst = max(by, key=lambda p: (by[p], p))
            others = [g["score"][p] for p in g["score"] if p != worst]
            gain.append(g["score"][worst] - sum(others) / len(others))
    row["top_betrayer_lead"] = sum(gain) / len(gain) if gain else None
    row["audit_failures"] = sum(bool(g.get("audit", {}).get("mismatched")) for g in games)
    return row


def year(phase):
    return int(phase[1:5])


def play_game(setup, seed, workdir, narrate=False, on_phase=None):
    """on_phase(game, record, tokens): called before each movement phase is adjudicated."""
    game = Game()
    powers = sorted(game.powers)
    rng = lambda *key: random.Random("-".join(map(str, (seed,) + key)))
    table = None
    if setup.coordination in ("contract", "deposit"):
        mode = "deposit" if setup.coordination == "deposit" else "strict"
        table = ContractTable(pathlib.Path(workdir) / f"table-{seed}", powers, MODAL, game.get_current_phase(), mode)
    deposit = setup.coordination == "deposit"
    tokens = {p: 0.0 for p in powers}
    spent = set()  # escrows the victim has taken
    grudges, promises, phases, events = set(), [], [], []
    first = True
    while not game.is_game_done and year(game.get_current_phase()) <= setup.last_year:
        phase = game.get_current_phase()
        record = {"phase": phase, "promises": [], "orders": {}, "broken": [], "refused": []}
        if phase.endswith("M"):
            if table and not first:
                table.advance(phase)
            first = False
            new = []
            if phase.startswith("S"):
                year_phases = [phase, "F" + phase[1:]]
                new += pacts_for_year(game, rng(phase, "pacts"), setup.p_pact, grudges, year_phases)
            pacts_now = [p for p in promises + new if p.kind == "pact" and phase in p.phases]
            plans = {}
            for power in powers:
                forbidden, _ = constraints(pacts_now, power, phase)
                plans[power] = movement_orders(game, power, rng(phase, power, "plan"), forbidden)
            new += support_deals(game, rng(phase, "supports"), setup.p_support, pacts_now, plans, phase)
            for p in new:
                if table and not table.promise(p, setup.deposit if deposit else 0.0)[0]:
                    raise RuntimeError(f"{phase}: the contract refused a promise: {table.rules[-1]}")
                promises.append(p)
                record["promises"].append(dataclasses.asdict(p))
            for power in powers:
                if not game.get_orderable_locations(power):
                    continue
                forbidden, fixed = constraints(promises, power, phase)
                honest = movement_orders(game, power, rng(phase, power, "orders"), forbidden, fixed)
                attempt, broken = betrayals(game, power, honest, promises, phase, rng(phase, power, "tempt"),
                                            setup.temptation, setup.deposit if deposit else 0.0, spent)
                played = attempt
                if deposit:
                    ok, detail = table.submit(power, phase, list(attempt.values()))
                    if not ok:
                        raise RuntimeError(f"{power} {phase}: orders refused: {detail}")
                elif table:
                    ok, detail = table.submit(power, phase, list(attempt.values()))
                    if not ok:
                        record["refused"] += [{"promise": p.words(), "order": o, "detail": detail}
                                              for p, o in broken]
                        ok, detail = table.submit(power, phase, list(honest.values()))
                        if not ok:
                            raise RuntimeError(f"{power} {phase}: promised orders refused: {detail}")
                        played = honest
                        broken = []
                for p, order in broken:
                    entry = {"promise": p.words(), "kind": p.kind, "by": p.promiser, "against": p.to, "order": order}
                    compensated = False
                    if deposit and p.escrow not in spent:
                        ok, detail = table.claim(p.escrow, order if p.kind == "pact" else None, phase)
                        entry["claim"] = "accepted" if ok else f"refused: {detail}"
                        if ok:
                            spent.add(p.escrow)
                            tokens[p.promiser] -= setup.deposit
                            tokens[p.to] += setup.deposit
                            compensated = setup.deposit > 0
                    record["broken"].append(entry)
                    if not compensated:
                        grudges.add((p.to, p.promiser))
                record["orders"][power] = sorted(played.values())
                game.set_orders(power, list(played.values()))
        else:
            for power in powers:
                orders = other_orders(game, power, rng(phase, power, "other"))
                if orders:
                    game.set_orders(power, orders)
        if on_phase and phase.endswith("M"):
            on_phase(game, record, tokens)
        game.process()
        phases.append(record)
        if narrate and (record["promises"] or record["broken"] or record["refused"]):
            narrate_phase(record)
    centres = {p: len(game.get_power(p).centers) for p in powers}
    score = {p: centres[p] + tokens[p] for p in powers}
    top = max(score.values())
    leaders = [p for p, n in score.items() if n == top]
    out = {
        "seed": seed, "setup": dataclasses.asdict(setup), "phases": phases, "centres": centres,
        "tokens": tokens, "score": score,
        "winner": leaders[0] if len(leaders) == 1 else None, "solo": max(centres.values()) >= 18,
        "final_phase": game.get_current_phase(),
    }
    if table:
        out["contract"] = table.report()
        out["audit"] = audit(phases, table)
    return out


def audit(phases, table):
    """Every movement order the referee played is the accepted commit for that power and phase."""
    logged = table.logged_orders()
    bad = []
    for r in phases:
        for power, orders in r["orders"].items():
            if logged.get((power.lower(), r["phase"].lower())) != orders:
                bad.append(f"{power} {r['phase']}")
    return {"checked": sum(len(r["orders"]) for r in phases), "mismatched": bad}


def narrate_phase(r):
    print(f"{r['phase']}:")
    for p in r["promises"]:
        print(f"  promise  {p['promiser']} -> {p['to']}: {p['kind']} {p.get('provinces') or p.get('order')}")
    for b in r["broken"]:
        print(f"  BROKEN   {b['promise']}  ({b['order']})")
    for b in r["broken"]:
        if "claim" in b:
            print(f"  claim    {b['against']} takes {b['by']}'s deposit: {b['claim']}")
    for x in r["refused"]:
        print(f"  refused  {x['promise']}  ({x['order']})")


def summarize(game):
    s = {"pacts": 0, "supports": 0, "broken_pacts": 0, "broken_supports": 0, "refused": 0, "claimed": 0}
    per_year = {}
    for r in game["phases"]:
        for p in r["promises"]:
            s["pacts" if p["kind"] == "pact" else "supports"] += 1
            if p["kind"] == "pact":
                per_year[year(r["phase"])] = per_year.get(year(r["phase"]), 0) + 1
        for b in r["broken"]:
            s["broken_pacts" if b["kind"] == "pact" else "broken_supports"] += 1
            s["claimed"] += b.get("claim") == "accepted"
        s["refused"] += len(r["refused"])
    s["promises"] = s["pacts"] + s["supports"]
    s["broken"] = s["broken_pacts"] + s["broken_supports"]
    s["pacts_per_year"] = per_year
    return s


def run_one(task):
    setup, seed, workdir = task
    game = play_game(setup, seed, workdir)
    (pathlib.Path(workdir) / f"game-{seed}.json").write_text(json.dumps(game))
    shutil.rmtree(pathlib.Path(workdir) / f"table-{seed}", ignore_errors=True)
    return seed, summarize(game), game.get("audit")


def cmd_play(args):
    setup = Setup(coordination=args.coordination, temptation=args.temptation, last_year=args.last_year,
                  deposit=args.deposit)
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    shutil.rmtree(out / f"table-{args.seed}", ignore_errors=True)
    game = play_game(setup, args.seed, out, narrate=True)
    s = summarize(game)
    print(f"\nCentres: {game['centres']}  winner: {game['winner'] or 'none'}")
    print(f"Promises {s['promises']} ({s['pacts']} pacts, {s['supports']} supports); broken {s['broken']}, "
          f"refused {s['refused']}, deposits claimed {s['claimed']}")
    if game["setup"]["coordination"] == "deposit":
        print(f"Tokens: {game['tokens']}  score: {game['score']}")
    if "audit" in game:
        a = game["audit"]
        print(f"Audit: {a['checked']} order sets checked against the contract log, mismatches: {a['mismatched'] or 'none'}")
    (out / f"game-{args.seed}.json").write_text(json.dumps(game, indent=1))


def cmd_experiments(args):
    out = pathlib.Path(args.out)
    chosen = [e for e in EXPERIMENTS if not args.only or e.name in args.only.split(",")]
    tasks = []
    for e in chosen:
        (out / e.name).mkdir(parents=True, exist_ok=True)
        for seed in range(args.seed, args.seed + args.games):
            if not (out / e.name / f"game-{seed}.json").exists():
                tasks.append((e, seed, out / e.name))
    print(f"{len(tasks)} games to play", file=sys.stderr)
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(run_one, t): t for t in tasks}
        for i, fut in enumerate(concurrent.futures.as_completed(futures), 1):
            e, seed = futures[fut][0], futures[fut][1]
            try:
                _, s, a = fut.result()
                print(f"[{i}/{len(tasks)}] {e.name} seed {seed}: promises {s['promises']} broken {s['broken']} "
                      f"refused {s['refused']}" + (f" audit {len(a['mismatched'])} bad" if a else ""),
                      file=sys.stderr, flush=True)
            except Exception as exc:
                print(f"[{i}/{len(tasks)}] {e.name} seed {seed}: FAILED {exc}", file=sys.stderr, flush=True)
    rows = []
    for e in chosen:
        games = [json.loads(f.read_text()) for f in sorted((out / e.name).glob("game-*.json"))]
        if games:
            rows.append(row_for(e, games))
    (out / "results.json").write_text(json.dumps({"rows": rows}, indent=1))
    print(f"{'experiment':20} {'games':>5} {'promises':>8} {'broken':>7} {'refused':>7} {'claimed':>7} {'broken %':>8}")
    for r in rows:
        print(f"{r['experiment']:20} {r['games']:5} {r['promises']:8.1f} {r['broken']:7.1f} {r['refused']:7.1f} "
              f"{r['claimed']:7.1f} {100 * r['broken_share']:7.0f}%")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    play = sub.add_parser("play")
    play.add_argument("--coordination", choices=("chat", "contract", "deposit"), default="chat")
    play.add_argument("--deposit", type=float, default=1.0)
    play.add_argument("--temptation", type=float, default=0.3)
    play.add_argument("--last-year", type=int, default=1910)
    play.add_argument("--seed", type=int, default=1)
    play.add_argument("--out", default="out")
    play.set_defaults(func=cmd_play)
    exp = sub.add_parser("experiments")
    exp.add_argument("--games", type=int, default=16)
    exp.add_argument("--seed", type=int, default=1)
    exp.add_argument("--jobs", type=int, default=8)
    exp.add_argument("--only")
    exp.add_argument("--out", default="out/experiments")
    exp.set_defaults(func=cmd_experiments)
    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
