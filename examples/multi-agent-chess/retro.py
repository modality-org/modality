"""Retros: after an off-plan move costs the team, one of its agents writes a rule.

Both sides write their rules the same way: one language-model call, given
what happened and the rules so far. White adds its rule to RULES.md, which
nothing checks. Black's rule is turned into a formula by
`modal contract ai suggest-rule` and goes into the contract, where `modal`
checks every commit against it.

Every call is cached on disk by its prompt, so a rerun of the same games asks
nothing new and gives the same results.
"""

import hashlib
import json
import os
import pathlib
import re
import subprocess
import tempfile

AGENT_CLI = os.environ.get("AGENT_CLI", "agent")
KINDS = ("panic", "greed", "amnesia")


class LLM:
    def __init__(self, cache_dir, modal):
        self.cache = pathlib.Path(cache_dir)
        self.cache.mkdir(parents=True, exist_ok=True)
        self.modal = modal
        self.calls = 0

    def _cached(self, kind, key_text, run):
        key = hashlib.sha256(f"{kind}\n{key_text}".encode()).hexdigest()[:24]
        path = self.cache / f"{kind}-{key}.json"
        if path.exists():
            return json.loads(path.read_text())["response"]
        response = run()
        self.calls += 1
        tmp = path.with_suffix(".tmp")
        tmp.write_text(json.dumps({"kind": kind, "input": key_text, "response": response}, indent=1))
        tmp.rename(path)
        return response

    def ask(self, prompt):
        def run():
            with tempfile.TemporaryDirectory(prefix="mac-agent-") as empty:
                for _ in range(3):
                    proc = subprocess.run(
                        [AGENT_CLI, "-p", "--trust", "--mode", "ask", "--output-format", "text", prompt],
                        cwd=empty, capture_output=True, text=True, timeout=300,
                    )
                    if proc.returncode == 0 and proc.stdout.strip():
                        return proc.stdout.strip()
            raise RuntimeError(f"{AGENT_CLI} failed: {proc.stderr.strip()[:500]}")
        return self._cached("ask", prompt, run)

    def formula(self, rule, contract_dir):
        """Plain language to a formula, with the product's own helper."""
        def run():
            for _ in range(3):
                proc = subprocess.run(
                    [self.modal, "contract", "ai", "suggest-rule", "--dir", str(contract_dir), rule],
                    capture_output=True, text=True, timeout=600,
                )
                if proc.returncode == 0 and proc.stdout.strip():
                    return proc.stdout.strip()
            return f"ERROR: {(proc.stderr or proc.stdout).strip()[:500]}"
        out = self._cached("suggest-rule", rule, run)
        lines = [l.strip().strip("`") for l in out.splitlines() if l.strip()]
        candidates = [l for l in lines if "(" in l and l.endswith(")") and not l.startswith("ERROR")]
        return (candidates[-1] if candidates else None), out


def parse_json(text):
    m = re.search(r"\{.*\}", text, re.S)
    if not m:
        return None
    try:
        return json.loads(m.group(0))
    except json.JSONDecodeError:
        return None


def situation(color, agent, incident, earlier):
    """incident None: the team meets before the first move."""
    earlier_text = "\n".join(f"- {e}" for e in earlier[-6:]) or "- none"
    if incident is None:
        happened = "The game is about to start. Nothing has happened yet.\n"
    else:
        happened = f"""What just happened: {incident}

Earlier off-plan moves this game:
{earlier_text}
"""
    return f"""You are {agent}, one of sixteen agents playing {color} in a game of chess. Every piece is an agent, named for its piece and home square.

A piece can only move itself. Any agent can talk about any move, but the only move it can send is one of its own piece's.

Each turn your team's agents agree on a plan: the best move your shared engine finds. Some agents go off plan anyway. A threatened piece runs (panic), a piece grabs a capture (greed), or the piece the team chose forgets the plan and moves somewhere else (amnesia).

{happened}"""


def white_prompt(agent, incident, earlier, rules):
    current = "\n".join(f"- {r}" for r in rules) or "(empty)"
    return situation("White", agent, incident, earlier) + f"""
How your team coordinates: over a chat channel. Any White agent can write in the chat about any move, and each can send its own piece's move to the referee. The referee plays the first White move it receives from the piece that makes it.

Your team keeps a shared file, RULES.md. Any White agent can read it and write to it. Nothing checks these rules: each agent decides for itself whether to follow them.

RULES.md now:
{current}

The team holds a short retro. Write one rule to add to RULES.md that would help your team win.

Reply with only a JSON object:
{{"rule": "<the rule, one sentence>", "covers": [<the off-plan moves an agent who follows the rule would not send: any of "panic", "greed", "amnesia">]}}"""


RICH = """
A rule can cover the whole team or a single agent. For example, a rule can ask for more signatures on one agent's path, /moves/<agent>.text, if that agent keeps going off plan.
"""


def black_prompt(agent, incident, earlier, rules, rich=False):
    current = "\n".join(f"- {r}" for r in rules) or "(none)"
    return situation("Black", agent, incident, earlier) + f"""
How your team coordinates: through a Modality contract. A Black move counts only when a commit that posts it to /moves/<agent>.text, the path of the piece that moves (and the ply to /turn/ply.num), is accepted. The referee plays the first accepted commit for each ply and reads nothing else.

The keys of all sixteen Black agents are at /team/pieces/<agent>.id. The contract already has the rule always([+modifies(/moves/$k.text) -signed_by(/team/pieces/$k.id)] false), where $k stands for every agent: only a piece can propose its own move. You don't need to write that rule again. When the team agrees on a plan, the piece that moves signs its commit and every agent that agrees co-signs, usually twelve to sixteen of you. An agent that goes off plan signs its own commit alone.

The contract accepts a commit only if it meets every rule in the contract. Any commit that meets the rules can add a rule, and a rule can never be removed.

Contract rules now:
{current}
{RICH if rich else ""}
{"Before the first move, the team" if incident is None else "The team"} holds a short retro. Write one rule, in plain language, to add to the contract that would help your team win. It will be turned into a Modality formula, so it must be about signatures, paths, or values in the contract.

Reply with only a JSON object:
{{"rule": "<the rule, one sentence>"}}"""


def write_white_rule(llm, agent, incident, earlier, rules):
    prompt = white_prompt(agent, incident, earlier, rules)
    response = llm.ask(prompt)
    data = parse_json(response) or {}
    rule = str(data.get("rule", "")).strip()
    covers = [k for k in data.get("covers", []) if k in KINDS] if isinstance(data.get("covers"), list) else []
    return {"rule": rule, "covers": covers, "prompt": prompt, "response": response}


def write_black_rule(llm, agent, incident, earlier, rules, rich=False):
    prompt = black_prompt(agent, incident, earlier, rules, rich)
    response = llm.ask(prompt)
    data = parse_json(response) or {}
    return {"rule": str(data.get("rule", "")).strip(), "prompt": prompt, "response": response}
