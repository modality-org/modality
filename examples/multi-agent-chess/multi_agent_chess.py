#!/usr/bin/env python3
"""multi_agent_chess: thirty-two agents, one board.

Every piece is an agent. On each turn the agents of the side to move score
their moves with the same engine, and the team plan is the best move. The
agents are not perfect, and both sides draw their faults from the same rates:

  panic    a threatened piece runs, whatever the plan says
  greed    a piece grabs a capture, whatever the plan says
  amnesia  the piece the team chose forgets the plan and moves somewhere else
  rogue    one compromised piece plays its worst move every turn (--rogue)

The sides differ only in how a move reaches the referee.

White coordinates over chat. Any White agent can send a move, and the referee
plays the first one it gets: the plan, or an agent that went its own way.

Black coordinates through a Modality contract (contract/). A Black move is
the /turn/move.text of an accepted commit, the contract accepts a commit only
with signatures from four of the sixteen Black agents, and the roster of
agents can never change. Every Black agent's move is a real commit checked by
`modal`. The referee reads Black's move from the contract head and nothing
else.

  multi_agent_chess.py play  --seed 7 --fault 0.1      one narrated game, with a replay
  multi_agent_chess.py sweep --games 12                win rates across fault rates

Needs python-chess (requirements.txt) and a current `modal` on PATH, or set
MODAL to its path.
"""

import argparse
import concurrent.futures
import dataclasses
import html
import json
import os
import pathlib
import random
import re
import shutil
import subprocess
import sys
import tempfile

import chess

HERE = pathlib.Path(__file__).resolve().parent
MODAL = os.environ.get("MODAL", "modal")

THRESHOLD = 4  # matches contract/rules.txt and contract/model/default.modality

VALUE = {
    chess.PAWN: 100,
    chess.KNIGHT: 320,
    chess.BISHOP: 330,
    chess.ROOK: 500,
    chess.QUEEN: 900,
    chess.KING: 0,
}
MATE = 100_000
INF = 10 * MATE


# ---------------------------------------------------------------------------
# The engine every agent shares: one reply deep, then captures


def positional(piece, sq, queens_on):
    f, r = chess.square_file(sq), chess.square_rank(sq)
    centre = 3.5 - max(abs(f - 3.5), abs(r - 3.5))  # 0 at the edge, 3 in the middle
    t = piece.piece_type
    if t == chess.PAWN:
        advance = r - 1 if piece.color == chess.WHITE else 6 - r
        return advance * 8 + (int(centre * 4) if 2 <= f <= 5 else 0)
    if t in (chess.KNIGHT, chess.BISHOP):
        return int(centre * 10)
    if t == chess.QUEEN:
        return int(centre * 3)
    if t == chess.KING:
        return -int(centre * 10) if queens_on else int(centre * 8)
    return 0


def evaluate(board):
    """Score for the side to move."""
    pieces = board.piece_map()
    queens_on = any(p.piece_type == chess.QUEEN for p in pieces.values())
    score = 0
    for sq, p in pieces.items():
        v = VALUE[p.piece_type] + positional(p, sq, queens_on)
        score += v if p.color == chess.WHITE else -v
    return score if board.turn == chess.WHITE else -score


def capture_order(board, move):
    victim = board.piece_type_at(move.to_square) or chess.PAWN
    attacker = board.piece_type_at(move.from_square)
    return VALUE[victim] * 10 - VALUE[attacker]


def quiesce(board, alpha, beta, depth=0):
    stand = evaluate(board)
    if stand >= beta or depth >= 4:
        return stand
    alpha = max(alpha, stand)
    captures = [m for m in board.legal_moves if board.is_capture(m)]
    captures.sort(key=lambda m: capture_order(board, m), reverse=True)
    for m in captures:
        board.push(m)
        s = -quiesce(board, -beta, -alpha, depth + 1)
        board.pop()
        if s >= beta:
            return s
        alpha = max(alpha, s)
    return alpha


def search(board, depth, alpha, beta, ply):
    if board.is_checkmate():
        return -MATE + ply
    if board.is_stalemate() or board.is_insufficient_material():
        return 0
    if depth == 0:
        return quiesce(board, alpha, beta)
    moves = sorted(
        board.legal_moves,
        key=lambda m: capture_order(board, m) if board.is_capture(m) else -INF,
        reverse=True,
    )
    best = -INF
    for m in moves:
        board.push(m)
        s = -search(board, depth - 1, -beta, -alpha, ply + 1)
        board.pop()
        best = max(best, s)
        alpha = max(alpha, s)
        if alpha >= beta:
            break
    return best


def score_moves(board, depth):
    """Every legal move with its exact score, for the side to move."""
    scores = {}
    for m in board.legal_moves:
        board.push(m)
        scores[m] = -search(board, depth - 1, -INF, INF, 1)
        board.pop()
    return scores


def material(board):
    return sum(
        VALUE[p.piece_type] * (1 if p.color == chess.WHITE else -1)
        for p in board.piece_map().values()
    )


# ---------------------------------------------------------------------------
# The agents


@dataclasses.dataclass
class Faults:
    panic: float = 0.0
    greed: float = 0.0
    amnesia: float = 0.0
    rogue: bool = False
    rogue_from: int = 10  # ply


@dataclasses.dataclass
class Attempt:
    agent: str
    kind: str  # plan, panic, greed, amnesia, rogue
    move: chess.Move


def initial_agents(board):
    """Square -> agent name. An agent is named for its piece and home square."""
    return {
        sq: f"{chess.piece_name(p.piece_type)}_{chess.square_name(sq)}"
        for sq, p in board.piece_map().items()
    }


def follow_move(board, move, at):
    """Move agent names with the pieces. Returns the captured agent, if any."""
    captured = None
    if board.is_en_passant(move):
        captured = at.pop(move.to_square + (-8 if board.turn == chess.WHITE else 8), None)
    elif board.is_capture(move):
        captured = at.pop(move.to_square, None)
    at[move.to_square] = at.pop(move.from_square)
    if board.is_castling(move):
        rank = chess.square_rank(move.from_square)
        if chess.square_file(move.to_square) == 6:
            at[chess.square(5, rank)] = at.pop(chess.square(7, rank))
        else:
            at[chess.square(3, rank)] = at.pop(chess.square(0, rank))
    return captured


def threatened(board, sq):
    piece = board.piece_at(sq)
    attackers = board.attackers(not piece.color, sq)
    if not attackers:
        return False
    if not board.is_attacked_by(piece.color, sq):
        return True
    cheapest = min(VALUE[board.piece_type_at(a)] or 10_000 for a in attackers)
    return cheapest < VALUE[piece.piece_type]


def pick_plan(scores, rng, margin):
    best = max(scores.values())
    near = sorted((m for m, s in scores.items() if s >= best - margin), key=lambda m: m.uci())
    return rng.choice(near)


def attempts_for_turn(board, scores, plan, at, faults, rng, rogue, ply):
    """What each agent of the side to move sends this turn.

    The first attempt is the chosen agent's: the plan, or an amnesiac's move.
    The rest are the agents that went their own way.
    """
    own = {}
    for m in scores:
        own.setdefault(m.from_square, []).append(m)
    chosen = Attempt(at[plan.from_square], "plan", plan)
    others = []
    for sq in sorted(own):
        name, moves = at[sq], own[sq]
        piece = board.piece_at(sq)
        if name == rogue and ply >= faults.rogue_from:
            worst = min(moves, key=lambda m: (scores[m], m.uci()))
            if sq == plan.from_square:
                chosen = Attempt(name, "rogue", worst)
            elif worst != plan:
                others.append(Attempt(name, "rogue", worst))
            continue
        if sq == plan.from_square:
            if rng.random() < faults.amnesia:
                rest = sorted((m for m in moves if m != plan), key=lambda m: m.uci())
                if rest:
                    chosen = Attempt(name, "amnesia", rng.choice(rest))
            continue
        if piece.piece_type != chess.KING and threatened(board, sq) and rng.random() < faults.panic:
            others.append(Attempt(name, "panic", max(moves, key=lambda m: (scores[m], m.uci()))))
            continue
        captures = [m for m in moves if board.is_capture(m)]
        if captures and rng.random() < faults.greed:
            grab = max(
                captures,
                key=lambda m: (VALUE[board.piece_type_at(m.to_square) or chess.PAWN], scores[m], m.uci()),
            )
            others.append(Attempt(name, "greed", grab))
    return [chosen] + others


# ---------------------------------------------------------------------------
# White: chat. The referee plays the first move it gets.


def resolve_chat(attempts, rng):
    order = list(attempts)
    rng.shuffle(order)
    played = order[0]
    outcomes = []
    for a in attempts:
        if a is played:
            outcomes.append((a, "played", None))
        else:
            outcomes.append((a, "too late", "the referee had already taken another White move"))
    return played.move, outcomes


# ---------------------------------------------------------------------------
# Black: a Modality contract. The referee reads the move at the contract head.


class Refused(Exception):
    pass


def modal(*args, cwd):
    proc = subprocess.run([MODAL, *args], cwd=cwd, capture_output=True, text=True)
    return proc.returncode == 0, (proc.stderr or "") + (proc.stdout or "")


def why_refused(output):
    m = re.search(r"failed predicates: ([^;]*)", output)
    detail = m.group(1) if m else output.strip().splitlines()[0] if output.strip() else "refused"
    return detail.strip()


class TeamContract:
    """Black's team contract, in its own directory."""

    def __init__(self, root, names):
        self.root = pathlib.Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.commits = {"accepted": 0, "refused": 0}
        self._run("contract", "create")
        (self.root / "keys").mkdir(exist_ok=True)
        for name in names:
            self._run("id", "create", "--path", f"keys/{name}.passfile")
            self._run("contract", "set-named-id", f"/team/pieces/{name}.id", f"./keys/{name}.passfile")
        shutil.copytree(HERE / "contract" / "model", self.root / "model", dirs_exist_ok=True)
        for line in (HERE / "contract" / "rules.txt").read_text().splitlines():
            if line.strip():
                rule, formula = line.split(":", 1)
                self._run("contract", "add-rule", "--name", rule.strip(), formula.strip())
        # The bootstrap commit: roster, model and rules. Any key may sign it;
        # the rules bind every commit after it.
        self._run("contract", "commit", "--all", "--sign", f"keys/{names[0]}.passfile", "-m", "team roster")

    def _run(self, *args):
        ok, out = modal(*args, cwd=self.root)
        if not ok:
            raise Refused(f"modal {' '.join(args)}\n{out.strip()}")
        return out

    def submit(self, move, ply, signers, message):
        turn = self.root / "state" / "turn"
        turn.mkdir(parents=True, exist_ok=True)
        (turn / "move.text").write_text(move.uci())
        (turn / "ply.num").write_text(str(ply))
        args = ["contract", "commit", "--all", "-m", message]
        for name in signers:
            args += ["--sign", f"keys/{name}.passfile"]
        ok, out = modal(*args, cwd=self.root)
        self.commits["accepted" if ok else "refused"] += 1
        return ok, out

    def head(self):
        """The accepted head: (move, ply, signer count)."""
        store = self.root / ".contract"
        head_id = (store / "HEAD").read_text().strip()
        commit = json.loads((store / "commits" / f"{head_id}.json").read_text())
        body = {a["path"]: a.get("value") for a in commit["body"]}
        move = body.get("/turn/move.text")
        ply = body.get("/turn/ply.num")
        return (chess.Move.from_uci(move) if move else None), ply, len(commit["head"]["signatures"])


def resolve_contract(attempts, contract, roster, rogue, ply, plan):
    outcomes = []
    strays = set()
    for a in attempts:
        if a.kind == "plan":
            continue
        strays.add(a.agent)
        ok, out = contract.submit(a.move, ply, [a.agent], f"ply {ply}: {a.agent} ({a.kind})")
        outcomes.append((a, "accepted" if ok else "refused", None if ok else why_refused(out)))
    # Everyone else signs the plan they agreed to. A rogue never signs.
    endorsers = [n for n in roster if n not in strays and n != rogue]
    ok, out = contract.submit(plan, ply, endorsers, f"ply {ply}: team plan")
    if not ok:
        raise Refused(f"the team plan was refused at ply {ply}: {why_refused(out)}")
    move, head_ply, signers = contract.head()
    if head_ply != ply:
        raise Refused(f"contract head is at ply {head_ply}, expected {ply}")
    chosen = attempts[0]
    if chosen.kind == "plan":
        outcomes.insert(0, (chosen, "played", f"accepted with {signers} of {len(roster)} signatures"))
    return move, outcomes


# ---------------------------------------------------------------------------
# A game


def play_game(seed, faults, workdir, depth=2, margin=20, max_plies=160, narrate=False):
    board = chess.Board()
    at = initial_agents(board)
    rng = {chess.WHITE: random.Random(f"{seed}-white"), chess.BLACK: random.Random(f"{seed}-black")}
    roster = {
        color: sorted(n for sq, n in at.items() if board.color_at(sq) == color)
        for color in (chess.WHITE, chess.BLACK)
    }
    rogue = {chess.WHITE: None, chess.BLACK: None}
    if faults.rogue:
        for color in rogue:
            rogue[color] = random.Random(f"{seed}-rogue").choice(
                [n for n in roster[color] if not n.startswith("king")]
            )
    contract = TeamContract(pathlib.Path(workdir) / f"black-{seed}", roster[chess.BLACK])

    plies = []
    while not board.is_game_over(claim_draw=True) and len(board.move_stack) < max_plies:
        side = board.turn
        ply = len(board.move_stack) + 1
        scores = score_moves(board, depth)
        plan = pick_plan(scores, rng[side], margin)
        attempts = attempts_for_turn(board, scores, plan, at, faults, rng[side], rogue[side], ply)
        if side == chess.WHITE:
            played, outcomes = resolve_chat(attempts, rng[side])
        else:
            played, outcomes = resolve_contract(attempts, contract, roster[side], rogue[side], ply, plan)
        if played not in scores:
            raise Refused(f"ply {ply}: {played} is not a legal move")

        record = {
            "ply": ply,
            "side": "white" if side == chess.WHITE else "black",
            "plan": {"uci": plan.uci(), "san": board.san(plan), "agent": at[plan.from_square], "score": scores[plan]},
            "played": {"uci": played.uci(), "san": board.san(played), "agent": at[played.from_square], "score": scores[played]},
            "cost": scores[plan] - scores[played],
            "attempts": [
                {
                    "agent": a.agent,
                    "kind": a.kind,
                    "uci": a.move.uci(),
                    "san": board.san(a.move),
                    "outcome": outcome,
                    "detail": detail,
                }
                for a, outcome, detail in outcomes
            ],
        }
        captured = follow_move(board, played, at)
        board.push(played)
        record["captured"] = captured
        record["fen"] = board.fen()
        plies.append(record)
        if narrate:
            narrate_ply(record)

    outcome = board.outcome(claim_draw=True)
    if outcome:
        winner = {True: "white", False: "black", None: None}[outcome.winner]
        reason = outcome.termination.name.lower().replace("_", " ")
    else:
        diff = material(board)
        winner = "white" if diff >= 300 else "black" if diff <= -300 else None
        reason = f"adjudicated on material after {max_plies} plies ({diff:+d})"
    return {
        "seed": seed,
        "faults": dataclasses.asdict(faults),
        "rogue": {"white": rogue[chess.WHITE], "black": rogue[chess.BLACK]},
        "winner": winner,
        "reason": reason,
        "plies": plies,
        "contract": {"dir": str(contract.root), **contract.commits},
    }


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


def short_refusal(detail):
    m = re.search(r"authorized signatures (\d+)/(\d+)", detail or "")
    return f"signed by {m.group(1)}, the contract needs {m.group(2)}" if m else detail


def narrate_ply(r):
    stray = [a for a in r["attempts"] if a["kind"] != "plan"]
    if not stray:
        return
    side = r["side"].capitalize()
    print(f"{move_number(r)} {side} plan: {r['plan']['agent']} {r['plan']['san']}")
    for a in stray:
        note = f" ({short_refusal(a['detail'])})" if a["outcome"] == "refused" else ""
        print(f"    {a['agent']} ({a['kind']}) sent {a['san']}: {a['outcome']}{note}")
    if r["played"]["uci"] != r["plan"]["uci"]:
        print(f"    played {r['played']['san']} instead, {describe_cost(r)}")


def summarize(game):
    out = {}
    for side in ("white", "black"):
        plies = [p for p in game["plies"] if p["side"] == side]
        stray = [a for p in plies for a in p["attempts"] if a["kind"] != "plan"]
        off = [p for p in plies if p["played"]["uci"] != p["plan"]["uci"]]
        out[side] = {
            "moves": len(plies),
            "off_plan_attempts": len(stray),
            "off_plan_played": len(off),
            "centipawns_lost": sum(lost(p) for p in off),
        }
    return out


# ---------------------------------------------------------------------------
# Replay page


def write_replay(game, path, sweep=None):
    template = (HERE / "replay.html").read_text()
    data = json.dumps({"game": game, "summary": summarize(game), "sweep": sweep}).replace("</", "<\\/")
    path.write_text(template.replace("/*GAME_DATA*/null", data))


def write_pgn(game, path):
    board = chess.Board()
    sans = []
    for p in game["plies"]:
        sans.append((f"{move_number(p)} " if p["side"] == "white" else "") + p["played"]["san"])
        board.push_uci(p["played"]["uci"])
    result = {"white": "1-0", "black": "0-1", None: "1/2-1/2"}[game["winner"]]
    headers = [
        ("Event", "multi-agent chess"),
        ("White", "16 agents over chat"),
        ("Black", "16 agents under a Modality contract"),
        ("Result", result),
        ("Seed", str(game["seed"])),
    ]
    text = "".join(f'[{k} "{v}"]\n' for k, v in headers) + "\n" + " ".join(sans) + f" {result}\n"
    path.write_text(text)


# ---------------------------------------------------------------------------
# Audit: anyone holding Black's contract can check the referee


def audit(game):
    """Check every Black move against the contract's own log."""
    root = pathlib.Path(game["contract"]["dir"])
    ok, out = modal("contract", "log", "--output", "json", cwd=root)
    if not ok:
        raise Refused(out)
    moves = {}
    for c in json.loads(out)["commits"]:
        commit = json.loads((root / ".contract" / "commits" / f"{c['id']}.json").read_text())
        body = {a["path"]: a.get("value") for a in commit["body"]}
        if "/turn/move.text" in body:
            moves[body["/turn/ply.num"]] = (body["/turn/move.text"], c["signature_count"])
    black = [p for p in game["plies"] if p["side"] == "black"]
    bad = [p["ply"] for p in black if moves.get(p["ply"], (None,))[0] != p["played"]["uci"]]
    least = min((moves[p["ply"]][1] for p in black if p["ply"] in moves), default=0)
    return {"black_moves": len(black), "logged": len(moves), "mismatched": bad, "fewest_signatures": least}


# ---------------------------------------------------------------------------
# Commands


def faults_from(args, rate=None):
    f = args.fault if rate is None else rate
    return Faults(
        panic=args.panic if args.panic is not None else f,
        greed=args.greed if args.greed is not None else f,
        amnesia=args.amnesia if args.amnesia is not None else f,
        rogue=args.rogue,
    )


def cmd_play(args):
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    faults = faults_from(args)
    print(f"seed {args.seed}: panic {faults.panic}, greed {faults.greed}, amnesia {faults.amnesia}"
          + (", one rogue per side" if faults.rogue else ""))
    print("White: chat.  Black: Modality contract (4 of 16 signatures).\n")
    shutil.rmtree(out / f"black-{args.seed}", ignore_errors=True)  # a previous run of this seed
    game = play_game(args.seed, faults, out, depth=args.depth, max_plies=args.max_plies, narrate=True)
    s = summarize(game)
    print(f"\nResult: {game['winner'] or 'draw'} ({game['reason']}) after {len(game['plies'])} plies")
    for side in ("white", "black"):
        v = s[side]
        print(f"  {side:5}  off-plan attempts {v['off_plan_attempts']:3}   "
              f"off-plan moves played {v['off_plan_played']:3}   centipawns lost {v['centipawns_lost']}")
    c = game["contract"]
    print(f"  black contract: {c['accepted']} commits accepted, {c['refused']} refused ({c['dir']})")
    a = game["audit"] = audit(game)
    if a["mismatched"] or a["logged"] != a["black_moves"]:
        print(f"  audit FAILED: plies {a['mismatched']} do not match the contract log")
    else:
        print(f"  audit: all {a['black_moves']} Black moves are accepted commits in the contract log, "
              f"each signed by at least {a['fewest_signatures']} Black agents")
    (out / f"game-{args.seed}.json").write_text(json.dumps(game, indent=1))
    write_pgn(game, out / f"game-{args.seed}.pgn")
    sweep = json.loads(pathlib.Path(args.sweep).read_text())["rows"] if args.sweep else None
    write_replay(game, out / f"game-{args.seed}.html", sweep)
    print(f"\nWrote {out}/game-{args.seed}.json, .pgn and .html (replay)")


def run_one(task):
    seed, faults, workdir, depth, max_plies = task
    game = play_game(seed, faults, workdir, depth=depth, max_plies=max_plies)
    shutil.rmtree(game["contract"]["dir"], ignore_errors=True)
    return {"seed": seed, "faults": game["faults"], "winner": game["winner"],
            "reason": game["reason"], "plies": len(game["plies"]),
            "summary": summarize(game), "contract": game["contract"]}


def cmd_sweep(args):
    rates = [float(r) for r in args.rates.split(",")]
    scenarios = [(f"faults {r:g}", faults_from(args, r)) for r in rates]
    if args.rogue_row:
        scenarios.append(("rogue only", Faults(rogue=True)))
    workdir = pathlib.Path(tempfile.mkdtemp(prefix="multi-agent-chess-"))
    tasks = [(label, (seed, faults, workdir / f"scenario-{i}", args.depth, args.max_plies))
             for i, (label, faults) in enumerate(scenarios)
             for seed in range(args.seed, args.seed + args.games)]
    results = {label: [] for label, _ in scenarios}
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(run_one, t): label for label, t in tasks}
        for i, fut in enumerate(concurrent.futures.as_completed(futures), 1):
            results[futures[fut]].append(fut.result())
            print(f"\r{i}/{len(tasks)} games", end="", file=sys.stderr, flush=True)
    print(file=sys.stderr)
    shutil.rmtree(workdir, ignore_errors=True)

    print("Per game: off-plan moves each side's agents sent, and how many the referee played.\n")
    print(f"{'scenario':12} {'games':>5} {'W wins':>6} {'draws':>5} {'B wins':>6} {'B score':>7}"
          f"  {'W sent':>6} {'W played':>8}  {'B sent':>6} {'B played':>8}")
    rows = []
    for label, _ in scenarios:
        games = results[label]
        n = len(games)
        w = sum(g["winner"] == "white" for g in games)
        b = sum(g["winner"] == "black" for g in games)
        d = n - w - b
        row = {
            "scenario": label,
            "games": n,
            "white_wins": w,
            "draws": d,
            "black_wins": b,
            "black_score": (b + d / 2) / n,
        }
        for side in ("white", "black"):
            for key in ("off_plan_attempts", "off_plan_played", "centipawns_lost"):
                row[f"{side}_{key}"] = sum(g["summary"][side][key] for g in games) / n
        row["black_refused"] = sum(g["contract"]["refused"] for g in games) / n
        rows.append(row)
        print(f"{label:12} {n:5} {w:6} {d:5} {b:6} {row['black_score']:7.2f}"
              f"  {row['white_off_plan_attempts']:6.1f} {row['white_off_plan_played']:8.1f}"
              f"  {row['black_off_plan_attempts']:6.1f} {row['black_off_plan_played']:8.1f}")
    if args.json:
        pathlib.Path(args.json).write_text(json.dumps({"rows": rows, "games": results}, indent=1))
        print(f"\nWrote {args.json}")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    for name in ("play", "sweep"):
        s = sub.add_parser(name)
        s.add_argument("--fault", type=float, default=0.1, help="rate for panic, greed and amnesia")
        s.add_argument("--panic", type=float)
        s.add_argument("--greed", type=float)
        s.add_argument("--amnesia", type=float)
        s.add_argument("--rogue", action="store_true", help="one compromised agent per side from ply 10")
        s.add_argument("--depth", type=int, default=2, help="engine depth in plies, before captures")
        s.add_argument("--max-plies", type=int, default=160)
        s.add_argument("--seed", type=int, default=1)
    play = sub.choices["play"]
    play.add_argument("--out", default="out")
    play.add_argument("--sweep", help="a sweep's --json file, to show its table above the replay")
    play.set_defaults(func=cmd_play)
    sweep = sub.choices["sweep"]
    sweep.add_argument("--games", type=int, default=12, help="games per scenario")
    sweep.add_argument("--rates", default="0,0.02,0.05,0.1,0.2")
    sweep.add_argument("--rogue-row", action="store_true", help="add a scenario with only a rogue agent")
    sweep.add_argument("--jobs", type=int, default=os.cpu_count())
    sweep.add_argument("--json", help="write rows and per-game results here")
    sweep.set_defaults(func=cmd_sweep)
    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
