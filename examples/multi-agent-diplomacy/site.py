#!/usr/bin/env python3
"""Write the data behind www.modality.org/experiments/multi-agent-diplomacy.

  python3 site.py                         replays the page's seed in three worlds
  python3 site.py --run out/experiments   also copies that run's results

Replays one seed in chat, under a strict contract, and with one-centre
deposits, saving the map before each movement phase is adjudicated (the
orders drawn on it) and what was promised, broken, refused and claimed. Then,
in sites/www.modality.org:

  node scripts/highlight-modality.mjs static/data/multi-agent-diplomacy/rules.json
"""

import argparse
import json
import pathlib
import re
import shutil
import tempfile

from diplomacy.engine.renderer import Renderer

from diplomacy_demo import Setup, play_game, summarize

HERE = pathlib.Path(__file__).resolve().parent
STATIC = HERE.parent.parent / "sites" / "www.modality.org" / "static" / "data" / "multi-agent-diplomacy"

SEED = 9
WORLDS = {
    "chat": Setup("chat", "chat", note="Promises over chat"),
    "contract": Setup("contract", "contract", note="Promises as contract rules"),
    "deposit": Setup("deposit-1", "deposit", deposit=1.0, note="A one-centre deposit on every promise"),
}


def minify(svg):
    svg = re.sub(r"<\?xml[^>]*>", "", svg)
    svg = re.sub(r"<!DOCTYPE[^>]*>", "", svg)
    svg = re.sub(r"<!--.*?-->", "", svg, flags=re.S)
    return re.sub(r">\s+<", "><", svg).strip()


def trim(game, timeline):
    phases = []
    for r in game["phases"]:
        if not r["phase"].endswith("M"):
            continue
        phases.append({
            "phase": r["phase"],
            "promises": [{"kind": p["kind"], "promiser": p["promiser"], "to": p["to"],
                          "provinces": p.get("provinces", []), "order": p.get("order", "")} for p in r["promises"]],
            "broken": [{k: b[k] for k in ("kind", "by", "against", "order", "promise") if k in b}
                       | ({"claim": b["claim"]} if "claim" in b else {}) for b in r["broken"]],
            "refused": [{"promise": x["promise"], "order": x["order"]} for x in r["refused"]],
            **timeline[r["phase"]],
        })
    return {"seed": game["seed"], "setup": game["setup"], "phases": phases, "centres": game["centres"],
            "tokens": game["tokens"], "score": game["score"], "winner": game["winner"],
            "summary": summarize(game), "audit": game.get("audit")}


def replay(name, setup, out):
    frames = out / "frames" / name
    shutil.rmtree(frames, ignore_errors=True)
    frames.mkdir(parents=True)
    timeline = {}

    def on_phase(game, record, tokens):
        phase = game.get_current_phase()
        (frames / f"{phase}.svg").write_text(minify(Renderer(game).render(incl_orders=True)))
        timeline[phase] = {"centres": {p: len(game.get_power(p).centers) for p in game.powers},
                           "tokens": dict(tokens)}

    with tempfile.TemporaryDirectory() as work:
        game = play_game(setup, SEED, work, on_phase=on_phase)
        rules = game.get("contract", {}).get("rules", [])
        escrows = game.get("contract", {}).get("escrows", [])
    print(f"{name}: winner {game['winner']}, {summarize(game)['broken']} broken, {summarize(game)['refused']} refused")
    return trim(game, timeline), rules, escrows


def rule_examples(worlds):
    """One rule of each kind, as the games committed them."""
    out = []
    strict = worlds["contract"][1]
    pact = next(r for r in strict if "won't move" in r["promise"])
    support = next((r for r in strict if "will order" in r["promise"]), None)
    out.append({"key": "pact", "words": pact["promise"], "formula": pact["formula"]})
    if support:
        out.append({"key": "support", "words": support["promise"], "formula": support["formula"]})
    deposit_rule = next(r for r in worlds["deposit"][1] if "won't move" in r["promise"])
    out.append({"key": "deposit", "words": deposit_rule["promise"], "formula": deposit_rule["formula"]})
    return {"rules": out}


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--run", help="an experiments run, to copy its results.json")
    p.add_argument("--out", default=str(STATIC))
    args = p.parse_args()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    worlds = {name: replay(name, setup, out) for name, setup in WORLDS.items()}
    (out / "worlds.json").write_text(json.dumps({name: w[0] for name, w in worlds.items()}, separators=(",", ":")))
    (out / "rules.json").write_text(json.dumps(rule_examples(worlds), indent=1))
    if args.run:
        shutil.copyfile(pathlib.Path(args.run) / "results.json", out / "results.json")
    print(f"Wrote {out}")


if __name__ == "__main__":
    main()
