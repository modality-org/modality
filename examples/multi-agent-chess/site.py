#!/usr/bin/env python3
"""Write the data behind www.modality.org/demos/multi-agent-chess.

  python3 site.py                       plays the page's games from results/llm-cache
  python3 site.py --out /some/dir       writes there instead of the site's static dir
  python3 site.py --run out/experiments also writes eval.json and elo.json from that run's games

Plays the three games the page replays, from the cached language-model
answers, and writes each one trimmed to what the page shows, together with
results/results.json. A cache miss fails rather than asking a model, so the
page shows the games the docs page counts.
"""

import argparse
import json
import math
import pathlib
import random
import shutil
import tempfile

from multi_agent_chess import EXPERIMENTS, MODAL, Setup, audit, play_game, row_for, summarize
from retro import LLM

HERE = pathlib.Path(__file__).resolve().parent
STATIC = HERE.parent.parent / "sites" / "www.modality.org" / "static" / "data" / "multi-agent-chess"

GAMES = [
    ("chat", Setup("chat-vs-chat", black="chat"), 12),
    ("rules-md-chat", Setup("rules-md-vs-chat", white="rules-md", black="chat"), 1),
    ("baseline", Setup("baseline"), 7),
    ("rule-first", Setup("rogue-self-ruled", white="rules-md", black="self-ruled", rogue=True), 7),
    ("rogue-first", Setup("rogue-self-ruled", white="rules-md", black="self-ruled", rogue=True), 3),
]


def pick(d, *keys):
    return {k: d[k] for k in keys if d.get(k) is not None}


def trim(game):
    plies = []
    for p in game["plies"]:
        plies.append({
            **pick(p, "ply", "side", "cost", "captured", "fen"),
            "plan": pick(p["plan"], "san", "agent"),
            "played": pick(p["played"], "uci", "san", "agent"),
            "attempts": [pick(a, "agent", "kind", "san", "outcome", "detail") for a in p["attempts"]],
            "events": [pick(e, "type", "side", "agent", "text", "formula", "outcome", "detail") for e in p["events"]],
        })
    return {
        **pick(game, "seed", "labels", "rogue", "winner", "reason", "audit"),
        "setup": pick(game["setup"], "white", "black", "rogue", "rogue_from"),
        "plies": plies,
        "forfeit_events": [pick(e, "type", "agent", "text", "formula", "outcome") for e in game["forfeit_events"]],
        "rules": {
            "black": [pick(r, "rule", "formula", "ply", "author", "outcome", "detail") for r in game["contract"]["rules"]],
            "white": [pick(r, "rule", "ply", "author") for r in game["white_team"]["rules"]],
        },
        "contract": pick(game["contract"], "accepted", "refused"),
        **pick(game["contract"], "owns_rule"),
        "summary": summarize(game),
    }


class CacheOnly(LLM):
    """Answers only from the cache: the page must show the games the docs count."""

    def _cached(self, kind, key_text, run):
        def miss():
            raise SystemExit(f"{kind} is not in {self.cache}: {key_text[:120]}...")
        return super()._cached(kind, key_text, miss)


# Each comparison changes one side's way of coordinating and keeps the other on
# plain chat, so what changes is that side's method. Games under a stronger
# method end sooner, so counts go per move or as shares, not per game.
def per_game(side, key):
    return lambda rows: sum(r[side][key] for r in rows) / len(rows)


def per_moves(side, key, n=10):
    return lambda rows: n * sum(r[side][key] for r in rows) / max(1, sum(r[side]["moves"] for r in rows))


def share(side, key, of="off_plan_attempts"):
    return lambda rows: sum(r[side][key] for r in rows) / max(1, sum(r[side][of] for r in rows))


EVALS = {
    "rules-md": {
        "pair": ("chat-vs-chat", "rules-md-vs-chat"),
        "metrics": [
            ("sent", "Off-plan moves White's agents sent, per 10 White moves", "num1", per_moves("white", "off_plan_attempts")),
            ("held", "Share of them RULES.md held back", "pct", share("white", "held_back")),
            ("played", "Share of them that reached the board", "pct", share("white", "off_plan_played")),
            ("cp", "Centipawns White lost to them, per game", "num0", per_game("white", "centipawns_lost")),
            ("score", "White score (win 1, draw ½)", "num2", per_game("white", "score")),
            ("control", "Black, which only chats: share of its off-plan moves that reached the board", "pct",
             share("black", "off_plan_played")),
        ],
    },
    "contract": {
        "pair": ("chat-vs-chat", "baseline"),
        "metrics": [
            ("sent", "Off-plan moves Black's agents sent, per 10 Black moves", "num1", per_moves("black", "off_plan_attempts")),
            ("played", "Share of them that reached the board", "pct", share("black", "off_plan_played")),
            ("cp", "Centipawns Black lost to them, per game", "num0", per_game("black", "centipawns_lost")),
            ("replan", "Centipawns given up replanning around a refused piece, per game", "num0",
             per_game("black", "replan_centipawns")),
            ("score", "Black score (win 1, draw ½)", "num2", per_game("black", "score")),
            ("control", "White, which only chats: share of its off-plan moves that reached the board", "pct",
             share("white", "off_plan_played")),
        ],
    },
}


def interval(rows, value, rng, n=2000):
    """The value over all games, and a 95% interval from resampling games."""
    boots = sorted(value([rng.choice(rows) for _ in rows]) for _ in range(n))
    return {"mean": value(rows), "lo": boots[int(0.025 * n)], "hi": boots[int(0.975 * n) - 1]}


def eval_stats(run):
    rng = random.Random(0)
    per = {}
    for name in sorted({n for e in EVALS.values() for n in e["pair"]}):
        rows = []
        for f in sorted((run / name).glob("game-*.json")):
            g = json.loads(f.read_text())
            s = summarize(g)
            for side in ("white", "black"):
                s[side]["score"] = {side: 1, None: 0.5}.get(g["winner"], 0)
            rows.append(s)
        per[name] = rows
    out = {}
    for key, e in EVALS.items():
        a, b = e["pair"]
        out[key] = {
            "pair": [a, b],
            "games": {a: len(per[a]), b: len(per[b])},
            "metrics": [{"key": k, "label": label, "format": fmt,
                         "a": interval(per[a], value, rng), "b": interval(per[b], value, rng)}
                        for k, label, fmt, value in e["metrics"]],
        }
    return out


# Elo for each way of coordinating. Each experiment is a match between White's
# method and Black's. Only the experiments with the default faults, no rogue
# and uniform pawns share one setting, so only they go on one scale.
# Rated: the three ways of coordinating the page compares, each against the
# other two. RULES.md is followed half the time, as everywhere on the page; the
# self-ruled contract writes a rule before the first move and more after
# incidents. The other experiments are in the table, not rated.
ELO_ANCHOR, ELO_BASE = "chat", 1500
ELO_PLAYERS = {
    "chat": "Plain chat",
    "rules-md": "Chat + RULES.md",
    "self-ruled-start": "Self-ruled Modality contract",
}
ELO_MATCHES = {  # experiment: (White's method, Black's method)
    "chat-vs-chat": ("chat", "chat"),
    "rules-md-vs-chat": ("rules-md", "chat"),
    "self-ruled-start": ("chat", "self-ruled-start"),
    "rules-md-vs-self-ruled-start": ("rules-md", "self-ruled-start"),
}
K = math.log(10) / 400
PRIOR_SD = 400  # a light pull toward the anchor, so a sweep in a resample stays finite


def solve(a, b):
    """Gaussian elimination for a small dense system."""
    n = len(b)
    m = [row[:] + [b[i]] for i, row in enumerate(a)]
    for c in range(n):
        piv = max(range(c, n), key=lambda r: abs(m[r][c]))
        m[c], m[piv] = m[piv], m[c]
        for r in range(n):
            if r != c:
                f = m[r][c] / m[c][c]
                m[r] = [x - f * y for x, y in zip(m[r], m[c])]
    return [m[i][n] / m[i][i] for i in range(n)]


def fit_elo(games):
    """Ratings and White's first-move edge, by Newton's method on the Elo likelihood.

    games: (white_method, black_method, white_score). Draws count as half a win.
    """
    free = [p for p in ELO_PLAYERS if p != ELO_ANCHOR]
    idx = {p: i for i, p in enumerate(free)}
    n = len(free) + 1  # the last parameter is White's edge
    theta = [0.0] * n

    def rating(p):
        return 0.0 if p == ELO_ANCHOR else theta[idx[p]]

    for _ in range(50):
        g = [-t / PRIOR_SD ** 2 for t in theta]
        h = [[(-1 / PRIOR_SD ** 2 if i == j else 0.0) for j in range(n)] for i in range(n)]
        for w, b, s in games:
            e = 1 / (1 + math.exp(-K * (rating(w) + theta[-1] - rating(b))))
            x = [0.0] * n
            x[-1] += K
            if w != ELO_ANCHOR:
                x[idx[w]] += K
            if b != ELO_ANCHOR:
                x[idx[b]] -= K
            for i in range(n):
                g[i] += (s - e) * x[i]
                for j in range(n):
                    h[i][j] -= e * (1 - e) * x[i] * x[j]
        step = solve(h, g)
        theta = [t - d for t, d in zip(theta, step)]
        if max(abs(d) for d in step) < 1e-6:
            break
    return {**{p: ELO_BASE + rating(p) for p in ELO_PLAYERS}, "white_edge": theta[-1]}


def elo(run):
    by_exp = {}
    for name, (w, b) in ELO_MATCHES.items():
        files = sorted((run / name).glob("game-*.json"))
        if len(files) < 16:  # an experiment still running counts once it is done
            continue
        by_exp[name] = [(w, b, {"white": 1, None: 0.5}.get(g["winner"], 0))
                        for g in (json.loads(f.read_text()) for f in files)]
    point = fit_elo([x for gs in by_exp.values() for x in gs])
    rng = random.Random(0)
    boots = []
    for _ in range(1000):  # resample games within each experiment
        boots.append(fit_elo([rng.choice(gs) for gs in by_exp.values() for _ in gs]))

    def est(key):
        xs = sorted(bt[key] for bt in boots)
        return {"mean": point[key], "lo": xs[25], "hi": xs[974]}

    games = {p: sum(len(gs) for n, gs in by_exp.items() if p in ELO_MATCHES[n]) for p in ELO_PLAYERS}
    players = [{"key": p, "label": label, "games": games[p], "rating": est(p)}
               for p, label in ELO_PLAYERS.items() if games[p]]
    players.sort(key=lambda x: -x["rating"]["mean"])
    return {"anchor": ELO_ANCHOR, "base": ELO_BASE, "white_edge": est("white_edge"),
            "matches": {n: len(gs) for n, gs in by_exp.items()}, "players": players}


def results_rows(run, games_per=16):
    """A row per experiment from the run's games. An experiment the run has not
    finished keeps its row from results/results.json, marked pending."""
    previous = {r["experiment"]: r for r in json.loads((HERE / "results" / "results.json").read_text())["rows"]}
    rows = []
    for e in EXPERIMENTS:
        files = sorted((run / e.name).glob("game-*.json"))
        if len(files) >= games_per:
            games = []
            for f in files:
                g = json.loads(f.read_text())
                games.append({"seed": g["seed"], "winner": g["winner"], "reason": g["reason"],
                              "summary": summarize(g), "audit": g["audit"]})
            rows.append(row_for(e, games))
        elif e.name in previous:
            rows.append({**previous[e.name], "pending": True})
    return rows


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--cache", default=str(HERE / "results" / "llm-cache"))
    p.add_argument("--out", default=str(STATIC))
    p.add_argument("--only", help="comma-separated game names, e.g. chat")
    p.add_argument("--run", help="an experiments run, to write eval.json from its games")
    args = p.parse_args()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    if args.run:
        (out / "eval.json").write_text(json.dumps(eval_stats(pathlib.Path(args.run)), indent=1))
        (out / "elo.json").write_text(json.dumps(elo(pathlib.Path(args.run)), indent=1))
        print(f"Wrote {out / 'eval.json'} and {out / 'elo.json'}")
    for name, setup, seed in GAMES:
        if args.only and name not in args.only.split(","):
            continue
        with tempfile.TemporaryDirectory() as work:
            game = play_game(setup, seed, work, CacheOnly(args.cache, MODAL))
            game["audit"] = audit(game)
        (out / f"{name}.json").write_text(json.dumps(trim(game), separators=(",", ":")))
        print(f"{name}: seed {seed}, {game['winner'] or 'draw'} ({game['reason']}), {len(game['plies'])} plies")
    if args.run:
        rows = results_rows(pathlib.Path(args.run))
        (out / "results.json").write_text(json.dumps({"rows": rows}, indent=1))
    else:
        shutil.copyfile(HERE / "results" / "results.json", out / "results.json")
    print(f"Wrote {out}")


if __name__ == "__main__":
    main()
