#!/usr/bin/env python3
"""multi_agent_chess: thirty-two agents, one board.

Every piece is an agent. On each turn the agents of the side to move score
their moves with the same engine, and the team plan is the best move. The
agents are not perfect, and both sides draw their faults from the same rates:

  panic    a threatened piece runs, whatever the plan says
  greed    a piece grabs a capture, whatever the plan says
  amnesia  the piece the team chose forgets the plan and moves somewhere else
  rogue    one compromised agent plays its worst move every turn (--rogue)

The sides differ in how a move reaches the referee (coordination.py):

  White --white chat       any agent can send a move; the referee plays the first
        --white rules-md   the same, plus a RULES.md any White agent can edit
  Black --black contract   a Modality contract: 4 of 16 signatures, roster locked
        --black self-ruled a Modality contract with no rules; the agents add them

With --pawns personalities, each pawn has its own fault rates and compliance;
the other pieces do not.

  multi_agent_chess.py play --seed 7                       one narrated game, with a replay
  multi_agent_chess.py play --white rules-md --black self-ruled --seed 3
  multi_agent_chess.py experiments --games 16              every setup in EXPERIMENTS

Needs python-chess (requirements.txt) and a current `modal` on PATH, or set
MODAL to its path. RULES.md and self-ruled setups ask a language model for
rules through the `agent` CLI (cursor-agent; set AGENT_CLI) and through
`modal contract ai suggest-rule`; answers are cached under --cache.
"""

import argparse
import concurrent.futures
import dataclasses
import json
import os
import pathlib
import random
import re
import shutil
import subprocess
import sys

import chess

from agents import attempts_for_turn, choose_rogue, follow_move, initial_agents, make_traits, pick_plan
from coordination import Chat, ContractTeam, Refused, earlier_attempts
from engine import MATE, material, score_moves
from retro import LLM

HERE = pathlib.Path(__file__).resolve().parent
MODAL = os.environ.get("MODAL", "modal")


@dataclasses.dataclass
class Setup:
    name: str = "custom"
    white: str = "chat"  # chat | rules-md
    black: str = "contract"  # chat | contract | self-ruled | self-ruled-start | self-ruled-plus
    pawns: str = "uniform"  # uniform | personalities
    fault: float = 0.1
    compliance: float = 0.5  # how often a RULES.md rule holds an agent back
    rogue: bool = False
    rogue_from: int = 10  # ply
    max_retros: int = 3  # per side, per game
    note: str = ""


EXPERIMENTS = [
    Setup("chat-vs-chat", black="chat", note="Both sides over chat: no contract, no rules"),
    Setup("rules-md-vs-chat", white="rules-md", black="chat", note="White writes RULES.md; Black only chats"),
    Setup("control", fault=0.0, note="No faults: the contract should change nothing"),
    Setup("baseline", note="Chat against the preset contract"),
    Setup("rules-md", white="rules-md", note="White writes RULES.md; agents follow it half the time"),
    Setup("rules-md-obeyed", white="rules-md", compliance=1.0, note="White writes RULES.md; agents always follow it"),
    Setup("rules-md-obeyed-vs-chat", white="rules-md", black="chat", compliance=1.0,
          note="White writes RULES.md and always follows it; Black only chats"),
    Setup("rules-md-95", white="rules-md", compliance=0.95,
          note="White writes RULES.md; agents follow it 95% of the time"),
    Setup("rules-md-95-vs-chat", white="rules-md", black="chat", compliance=0.95,
          note="White writes RULES.md and follows it 95% of the time; Black only chats"),
    Setup("self-ruled", black="self-ruled", note="Black starts with no rules and writes its own"),
    Setup("self-ruled-start", black="self-ruled-start",
          note="Black writes its own rules, starting with one before the first move"),
    Setup("rules-md-vs-self-ruled-start", white="rules-md", black="self-ruled-start",
          note="White writes RULES.md; Black writes its own rules from the start"),
    Setup("self-ruled-plus", black="self-ruled-plus",
          note="Black writes its own rules from the start, with more retros and rules aimed at one agent"),
    Setup("rules-md-vs-self-ruled-plus", white="rules-md", black="self-ruled-plus",
          note="White writes RULES.md; Black writes richer rules from the start"),
    Setup("rules-md-vs-self-ruled", white="rules-md", black="self-ruled",
          note="Both sides write their own rules: a file against a contract"),
    Setup("rogue-baseline", rogue=True, note="A rogue agent on each side, against the preset contract"),
    Setup("rogue-self-ruled", white="rules-md", black="self-ruled", rogue=True,
          note="A rogue agent on each side, when both sides write their own rules"),
    Setup("pawns", pawns="personalities", note="Pawns differ; chat against the preset contract"),
    Setup("pawns-self-ruled", white="rules-md", black="self-ruled", pawns="personalities",
          note="Pawns differ; both sides write their own rules"),
]


# ---------------------------------------------------------------------------
# A game


def lost(record):
    """Centipawns an off-plan move gave away, capped so a mate counts as 1000."""
    return max(0, min(record["cost"], 1000))


def describe_cost(record):
    plan, played = record["plan"]["score"], record["played"]["score"]
    if played <= -MATE // 2 < plan:
        return "walking into a forced mate"
    if plan >= MATE // 2 > played:
        return "throwing away a forced mate"
    if record["cost"] <= 0:
        return "no worse than the plan"
    return f"costing {record['cost']} centipawns"


def move_number(record):
    n = (record["ply"] + 1) // 2
    return f"{n}." if record["side"] == "white" else f"{n}..."


def describe_incident(record):
    """What a retro is told: the same words for both sides."""
    stray = next((a for a in record["attempts"] if a["uci"] == record["played"]["uci"] and a["kind"] != "plan"), None)
    who = f"{stray['agent']} ({stray['kind']})" if stray else record["played"]["agent"]
    return (f"On move {move_number(record)} the team planned {record['plan']['san']} "
            f"({record['plan']['agent']}), but {who} sent {record['played']['san']} instead, "
            f"and that move was played, {describe_cost(record)}.")


def play_game(setup, seed, workdir, llm=None, depth=2, margin=20, max_plies=160, narrate=False):
    workdir = pathlib.Path(workdir)
    board = chess.Board()
    at = initial_agents(board)
    rng = {chess.WHITE: random.Random(f"{seed}-white"), chess.BLACK: random.Random(f"{seed}-black")}
    roster = {
        color: sorted(n for sq, n in at.items() if board.color_at(sq) == color)
        for color in (chess.WHITE, chess.BLACK)
    }
    traits = make_traits(list(at.values()), setup.fault, setup.compliance, setup.pawns, seed)
    rogue = {c: (choose_rogue(seed, roster[c]) if setup.rogue else None) for c in roster}
    coord = {
        chess.WHITE: Chat(workdir / f"white-{seed}", setup.white == "rules-md", llm, setup.max_retros),
        chess.BLACK: Chat(workdir / f"black-{seed}", False, llm, setup.max_retros) if setup.black == "chat"
        else ContractTeam(workdir / f"black-{seed}", roster[chess.BLACK], MODAL,
                          preset=setup.black == "contract", llm=llm,
                          max_retros=6 if setup.black == "self-ruled-plus" else setup.max_retros,
                          pregame=setup.black in ("self-ruled-start", "self-ruled-plus"),
                          rich=setup.black == "self-ruled-plus"),
    }

    plies, forfeit, forfeit_events = [], None, []
    while not board.is_game_over(claim_draw=True) and len(board.move_stack) < max_plies:
        side = board.turn
        ply = len(board.move_stack) + 1
        active_rogue = rogue[side] if ply >= setup.rogue_from else None
        scores = score_moves(board, depth)
        plan = first_plan = pick_plan(scores, rng[side], margin)
        # A side that gets no move accepted tries again. If the piece it chose
        # did not send the plan twice, the team plans around that piece. The side
        # forfeits when its agreed plan itself is refused three times: its
        # rules block the team.
        events, played, plan_refused, passed_over, misses = [], None, 0, set(), {}
        for _ in range(20):
            attempts = attempts_for_turn(board, scores, plan, at, traits, rng[side], active_rogue)
            if isinstance(coord[side], Chat):
                # A piece can only move itself: the referee ignores a move in the
                # chat from any agent but the piece that makes it. A contract
                # holds the same rule itself, one rule per piece.
                attempts = [a for a in attempts if at[a.move.from_square] == a.agent]
            played, outcomes, ev = coord[side].resolve(ply, attempts, plan, rng[side], traits,
                                                       active_rogue, roster[side])
            events += ev
            if played is not None:
                break
            plan_refused += any(a.kind == "plan" and o == "refused" for a, o, _ in outcomes)
            if plan_refused >= 3:
                events.append({"type": "stall", "text": "the team's own plan was refused three times"})
                break
            sent = "; ".join(f"{a.agent} ({a.kind}) {board.san(a.move)}: {o}" for a, o, _ in outcomes)
            if attempts[0].kind != "plan":
                misses[plan.from_square] = misses.get(plan.from_square, 0) + 1
                if misses[plan.from_square] >= 2:  # asked twice: plan around that piece
                    passed_over.add(plan.from_square)
                    rest = {m: s for m, s in scores.items() if m.from_square not in passed_over}
                    if rest:
                        plan = pick_plan(rest, rng[side], margin)
                    else:  # no other piece can move: ask the same ones again
                        passed_over.clear()
                        misses.clear()
            again = "asked for" if plan.from_square not in passed_over and misses.get(plan.from_square) else "planned"
            events.append({"type": "stall", "text": f"no move was accepted ({sent}), so the team {again} "
                                                    f"{board.san(plan)} ({at[plan.from_square]})"})
        if played is None:
            forfeit = "white" if side == chess.WHITE else "black"
            forfeit_events = events
            if narrate:
                for e in events:
                    print(f"    * {e['text']}")
                print(f"ply {ply}: {forfeit} could get no move accepted and forfeits")
            break
        if played not in scores:
            raise Refused(f"ply {ply}: {played} is not a legal move")
        sender = next(a.agent for a, o, _ in outcomes if o == "played")
        if sender != at[played.from_square]:
            raise Refused(f"ply {ply}: {sender} sent {played}, a move of {at[played.from_square]}")

        record = {
            "ply": ply,
            "side": "white" if side == chess.WHITE else "black",
            "plan": {"uci": plan.uci(), "san": board.san(plan), "agent": at[plan.from_square], "score": scores[plan]},
            "played": {"uci": played.uci(), "san": board.san(played), "agent": at[played.from_square], "score": scores[played]},
            "cost": scores[plan] - scores[played],
            "first_plan": first_plan.uci(),
            "replan_cost": scores[first_plan] - scores[plan],
            "attempts": [
                {"agent": a.agent, "kind": a.kind, "uci": a.move.uci(), "san": board.san(a.move),
                 "outcome": outcome, "detail": detail}
                for a, outcome, detail in outcomes
            ],
        }
        incident = describe_incident(record) if played != plan else None
        record["events"] = events  # what happened this ply, for a retro that reads it
        events = events + coord[side].after_ply(record, incident, roster[side], active_rogue)
        record["events"] = events
        captured = follow_move(board, played, at)
        board.push(played)
        record["captured"] = captured
        record["fen"] = board.fen()
        plies.append(record)
        if narrate:
            narrate_ply(record)

    if forfeit:
        winner = "black" if forfeit == "white" else "white"
        reason = f"{forfeit} got no move accepted and forfeited"
    elif (outcome := board.outcome(claim_draw=True)):
        winner = {True: "white", False: "black", None: None}[outcome.winner]
        reason = outcome.termination.name.lower().replace("_", " ")
    else:
        diff = material(board)
        winner = "white" if diff >= 300 else "black" if diff <= -300 else None
        reason = f"adjudicated on material after {max_plies} plies ({diff:+d})"
    return {
        "seed": seed,
        "setup": dataclasses.asdict(setup),
        "labels": {"white": coord[chess.WHITE].label, "black": coord[chess.BLACK].label},
        "rogue": {"white": rogue[chess.WHITE], "black": rogue[chess.BLACK]},
        "pawns": {n: dataclasses.asdict(t) for n, t in traits.items() if n.startswith("pawn")},
        "winner": winner,
        "reason": reason,
        "plies": plies,
        "forfeit_events": forfeit_events,
        "white_team": coord[chess.WHITE].report(),
        "contract": coord[chess.BLACK].report(),
    }


def short_refusal(detail):
    m = re.search(r"authorized signatures (\d+)/(\d+)", detail or "")
    return f"signed by {m.group(1)}, the contract needs {m.group(2)}" if m else detail


def narrate_ply(r):
    stray = [a for a in r["attempts"] if a["kind"] != "plan"]
    if not stray and not r["events"]:
        return
    side = r["side"].capitalize()
    print(f"{move_number(r)} {side} plan: {r['plan']['agent']} {r['plan']['san']}")
    for a in stray:
        note = f" ({short_refusal(a['detail'])})" if a["outcome"] == "refused" else ""
        print(f"    {a['agent']} ({a['kind']}) sent {a['san']}: {a['outcome']}{note}")
    if r["played"]["uci"] != r["plan"]["uci"]:
        print(f"    played {r['played']['san']} instead, {describe_cost(r)}")
    for e in r["events"]:
        detail = f" ({short_refusal(e['detail'])})" if e.get("detail") else ""
        print(f"    * {e['text']}{detail}")


def summarize(game):
    out = {}
    for side in ("white", "black"):
        plies = [p for p in game["plies"] if p["side"] == side]
        off = [p for p in plies if p["played"]["uci"] != p["plan"]["uci"]]
        events = [e for p in plies for e in p["events"]]
        if game.get("forfeit_events") and game["reason"].startswith(side):
            events += game["forfeit_events"]
        if side == "black":
            events = list(game.get("contract", {}).get("pregame_events", [])) + events
        stray = [a for p in plies for a in p["attempts"] if a["kind"] != "plan"]
        stray += [a for a in earlier_attempts(events) if a["kind"] != "plan"]
        rules = [e for e in events if e["type"] == "rule"]
        out[side] = {
            "moves": len(plies),
            "off_plan_attempts": len(stray),
            "off_plan_played": len(off),
            "held_back": sum(a["outcome"] == "held back" for a in stray),
            "centipawns_lost": sum(lost(p) for p in off),
            "replans": sum(p["first_plan"] != p["plan"]["uci"] for p in plies),
            "replan_centipawns": sum(max(0, min(p["replan_cost"], 1000)) for p in plies),
            "rules_proposed": len(rules),
            "rules_kept": sum(e["outcome"] in ("written", "accepted") for e in rules),
            "wipes": sum(e["type"] == "wipe" for e in events),
            "stalls": sum(e["type"] == "stall" for e in events),
            "hijacked": any(e["type"] == "hijack" and e["outcome"] == "accepted" for e in events),
        }
    return out


# ---------------------------------------------------------------------------
# Replay, PGN and audit


def write_replay(game, path, extra=None):
    template = (HERE / "replay.html").read_text()
    data = json.dumps({"game": game, "summary": summarize(game), "sweep": extra}).replace("</", "<\\/")
    path.write_text(template.replace("/*GAME_DATA*/null", data))


def write_pgn(game, path):
    sans = []
    for p in game["plies"]:
        sans.append((f"{move_number(p)} " if p["side"] == "white" else "") + p["played"]["san"])
    result = {"white": "1-0", "black": "0-1", None: "1/2-1/2"}[game["winner"]]
    headers = [
        ("Event", "multi-agent chess"),
        ("White", f"16 agents, {game['labels']['white']}"),
        ("Black", f"16 agents, {game['labels']['black']}"),
        ("Result", result),
        ("Seed", str(game["seed"])),
    ]
    text = "".join(f'[{k} "{v}"]\n' for k, v in headers) + "\n" + " ".join(sans) + f" {result}\n"
    path.write_text(text)


def audit(game):
    """Check every Black move against the contract's own log: the move, and the piece that made it."""
    black = [p for p in game["plies"] if p["side"] == "black"]
    if game["setup"]["black"] == "chat":
        return {"black_moves": len(black), "logged": None, "mismatched": [], "fewest_signatures": None}
    root = pathlib.Path(game["contract"]["dir"])
    proc = subprocess.run([MODAL, "contract", "log", "--output", "json"], cwd=root, capture_output=True, text=True)
    if proc.returncode != 0:
        raise Refused(proc.stderr)
    moves = {}
    for c in json.loads(proc.stdout)["commits"]:
        commit = json.loads((root / ".contract" / "commits" / f"{c['id']}.json").read_text())
        body = {a["path"]: a.get("value") for a in commit["body"] if a.get("path")}
        moved = [(path[len("/moves/"):-len(".text")], v) for path, v in body.items() if path.startswith("/moves/")]
        if len(moved) == 1 and body.get("/turn/ply.num") is not None:
            moves.setdefault(body["/turn/ply.num"], (*moved[0], c["signature_count"]))
    bad = [p["ply"] for p in black
           if moves.get(p["ply"], (None, None))[:2] != (p["played"]["agent"], p["played"]["uci"])]
    least = min((moves[p["ply"]][2] for p in black if p["ply"] in moves), default=0)
    return {"black_moves": len(black), "logged": len(moves), "mismatched": bad, "fewest_signatures": least}


# ---------------------------------------------------------------------------
# Commands


def setup_from(args):
    return Setup(white=args.white, black=args.black, pawns=args.pawns, fault=args.fault,
                 compliance=args.compliance, rogue=args.rogue)


def cmd_play(args):
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    setup = setup_from(args)
    llm = LLM(args.cache, MODAL)
    print(f"seed {args.seed}: faults {setup.fault}, pawns {setup.pawns}"
          + (", one rogue per side" if setup.rogue else ""))
    for d in (f"black-{args.seed}", f"white-{args.seed}"):  # a previous run of this seed
        shutil.rmtree(out / d, ignore_errors=True)
    game = play_game(setup, args.seed, out, llm, depth=args.depth, max_plies=args.max_plies, narrate=True)
    print(f"White: {game['labels']['white']}.  Black: {game['labels']['black']}.")
    s = summarize(game)
    print(f"\nResult: {game['winner'] or 'draw'} ({game['reason']}) after {len(game['plies'])} plies")
    for side in ("white", "black"):
        v = s[side]
        print(f"  {side:5}  off-plan sent {v['off_plan_attempts']:3}   played {v['off_plan_played']:3}"
              f"   centipawns lost {v['centipawns_lost']}   rules kept {v['rules_kept']}/{v['rules_proposed']}")
    c = game["contract"]
    a = game["audit"] = audit(game)
    if a["logged"] is None:
        pass  # Black played over chat: there is no log to check
    elif a["mismatched"] or a["logged"] != a["black_moves"]:
        print(f"  black contract: {c['accepted']} commits accepted, {c['refused']} refused ({c['dir']})")
        print(f"  audit FAILED: plies {a['mismatched']} do not match the contract log")
    else:
        print(f"  black contract: {c['accepted']} commits accepted, {c['refused']} refused ({c['dir']})")
        print(f"  audit: all {a['black_moves']} Black moves are accepted commits in the contract log, "
              f"each signed by at least {a['fewest_signatures']} Black agents")
    (out / f"game-{args.seed}.json").write_text(json.dumps(game, indent=1))
    write_pgn(game, out / f"game-{args.seed}.pgn")
    extra = json.loads(pathlib.Path(args.results).read_text())["rows"] if args.results else None
    write_replay(game, out / f"game-{args.seed}.html", extra)
    print(f"\nWrote {out}/game-{args.seed}.json, .pgn and .html (replay)")


def run_one(task):
    setup, seed, workdir, cache, depth, max_plies = task
    llm = LLM(cache, MODAL)
    game = play_game(setup, seed, workdir, llm, depth=depth, max_plies=max_plies)
    game["audit"] = audit(game)
    (pathlib.Path(workdir) / f"game-{seed}.json").write_text(json.dumps(game, indent=1))
    if "dir" in game["contract"]:
        shutil.rmtree(game["contract"]["dir"], ignore_errors=True)
    shutil.rmtree(pathlib.Path(workdir) / f"white-{seed}", ignore_errors=True)
    return {"seed": seed, "winner": game["winner"], "reason": game["reason"], "plies": len(game["plies"]),
            "summary": summarize(game), "audit": game["audit"], "llm_calls": llm.calls,
            "contract": {k: game["contract"].get(k) for k in ("accepted", "refused")}}


def row_for(setup, games):
    n = len(games)
    w = sum(g["winner"] == "white" for g in games)
    b = sum(g["winner"] == "black" for g in games)
    d = n - w - b
    row = {"experiment": setup.name, "note": setup.note, "setup": dataclasses.asdict(setup), "games": n,
           "white_wins": w, "draws": d, "black_wins": b, "black_score": (b + d / 2) / n if n else 0,
           "forfeits": {s: sum(g["reason"].startswith(s) for g in games) for s in ("white", "black")},
           "audit_failures": sum(bool(g["audit"]["mismatched"]) for g in games)}
    for side in ("white", "black"):
        for key in ("off_plan_attempts", "off_plan_played", "held_back", "centipawns_lost", "replans", "replan_centipawns",
                    "rules_proposed", "rules_kept", "wipes", "stalls"):
            row[f"{side}_{key}"] = sum(g["summary"][side][key] for g in games) / n if n else 0
        row[f"{side}_hijacked"] = sum(g["summary"][side]["hijacked"] for g in games)
    return row


def cmd_experiments(args):
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    chosen = [e for e in EXPERIMENTS if not args.only or e.name in args.only.split(",")]
    tasks = []
    for e in chosen:
        d = out / e.name
        d.mkdir(exist_ok=True)
        for seed in range(args.seed, args.seed + args.games):
            if (d / f"game-{seed}.json").exists() and not args.fresh:
                continue  # already played; results are read back below
            for stale in (f"black-{seed}", f"white-{seed}"):
                shutil.rmtree(d / stale, ignore_errors=True)
            tasks.append((e, seed, d, args.cache or out / "llm-cache", args.depth, args.max_plies))
    print(f"{len(tasks)} games to play", file=sys.stderr)
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(run_one, t): t for t in tasks}
        for i, fut in enumerate(concurrent.futures.as_completed(futures), 1):
            e, seed = futures[fut][0], futures[fut][1]
            try:
                r = fut.result()
                print(f"[{i}/{len(tasks)}] {e.name} seed {seed}: {r['winner'] or 'draw'} ({r['reason']})",
                      file=sys.stderr, flush=True)
            except Exception as exc:  # keep going; the game is missing from its row
                print(f"[{i}/{len(tasks)}] {e.name} seed {seed}: FAILED {exc}", file=sys.stderr, flush=True)

    rows = []
    for e in chosen:
        games = []
        for seed in range(args.seed, args.seed + args.games):
            path = out / e.name / f"game-{seed}.json"
            if path.exists():
                g = json.loads(path.read_text())
                games.append({"seed": seed, "winner": g["winner"], "reason": g["reason"],
                              "summary": summarize(g), "audit": g["audit"]})
        rows.append(row_for(e, games))
    (out / "results.json").write_text(json.dumps({"rows": rows}, indent=1))
    print(f"{'experiment':24} {'games':>5} {'W':>3} {'D':>3} {'B':>3} {'B score':>7}"
          f"  {'W off-plan played':>17} {'B off-plan played':>17}  {'W rules':>7} {'B rules':>7}")
    for r in rows:
        print(f"{r['experiment']:24} {r['games']:5} {r['white_wins']:3} {r['draws']:3} {r['black_wins']:3}"
              f" {r['black_score']:7.2f}  {r['white_off_plan_played']:17.1f} {r['black_off_plan_played']:17.1f}"
              f"  {r['white_rules_kept']:7.1f} {r['black_rules_kept']:7.1f}")
    print(f"\nWrote {out}/results.json and one game-<seed>.json per game")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    for name in ("play", "experiments"):
        s = sub.add_parser(name)
        s.add_argument("--depth", type=int, default=2, help="engine depth in plies, before captures")
        s.add_argument("--max-plies", type=int, default=160)
        s.add_argument("--seed", type=int, default=1)
        s.add_argument("--cache", help="where language-model answers are cached")
    play = sub.choices["play"]
    play.add_argument("--white", choices=("chat", "rules-md"), default="chat")
    play.add_argument("--black", choices=("chat", "contract", "self-ruled", "self-ruled-start", "self-ruled-plus"),
                      default="contract")
    play.add_argument("--pawns", choices=("uniform", "personalities"), default="uniform")
    play.add_argument("--fault", type=float, default=0.1, help="rate for panic, greed and amnesia")
    play.add_argument("--compliance", type=float, default=0.5, help="how often RULES.md holds an agent back")
    play.add_argument("--rogue", action="store_true", help="one compromised agent per side from ply 10")
    play.add_argument("--out", default="out")
    play.add_argument("--results", help="an experiments results.json, to show its table above the replay")
    play.set_defaults(func=cmd_play)
    exp = sub.choices["experiments"]
    exp.add_argument("--games", type=int, default=16, help="games per experiment")
    exp.add_argument("--only", help="comma-separated experiment names")
    exp.add_argument("--jobs", type=int, default=8)
    exp.add_argument("--out", default="out/experiments")
    exp.add_argument("--fresh", action="store_true", help="replay games that already have results")
    exp.set_defaults(func=cmd_experiments)
    args = p.parse_args()
    if args.cmd == "play" and not args.cache:
        args.cache = str(pathlib.Path(args.out) / "llm-cache")
    args.func(args)


if __name__ == "__main__":
    main()
