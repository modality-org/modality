"""Claude agents, one per unit, talking over team chat.

Every half second each living unit's agent is asked for its own order. It
sees what its unit sees, its team's orders, and what its teammates said last
time, and may say one short thing back. Messages arrive on the next step.

Every call goes through the Claude Code CLI and is cached on disk by its
prompt, so a rerun of the same battle asks nothing new.
"""

import concurrent.futures
import hashlib
import json
import os
import pathlib
import re
import subprocess
import tempfile

from battle import parse_order

# Every answer comes from one pinned model, and the model is part of each
# cache key, so answers from another model are never reused.
CLAUDE_CLI = os.environ.get("CLAUDE_CLI", "claude")
LLM_MODEL = os.environ.get("LLM_MODEL", "claude-sonnet-5-5")


class LLM:
    def __init__(self, cache_dir):
        self.cache = pathlib.Path(cache_dir)
        self.cache.mkdir(parents=True, exist_ok=True)
        self.calls = 0

    def ask(self, prompt):
        key = hashlib.sha256(f"ask\n{LLM_MODEL}\n{prompt}".encode()).hexdigest()[:24]
        path = self.cache / f"ask-{key}.json"
        if path.exists():
            return json.loads(path.read_text())["response"]
        with tempfile.TemporaryDirectory(prefix="smax-agent-") as empty:
            error = ""
            for _ in range(4):
                try:
                    proc = subprocess.run(
                        [CLAUDE_CLI, "-p", "--output-format", "text", "--model", LLM_MODEL, "--tools", "",
                         "--strict-mcp-config", "--setting-sources", "", "--no-session-persistence", "--", prompt],
                        cwd=empty, capture_output=True, text=True, timeout=180,
                    )
                except subprocess.TimeoutExpired:
                    error = "timed out"
                    continue
                if proc.returncode == 0 and proc.stdout.strip():
                    break
                error = proc.stderr.strip()[:500]
            else:
                raise RuntimeError(f"{CLAUDE_CLI} failed: {error}")
        response = proc.stdout.strip()
        self.calls += 1
        tmp = path.with_suffix(".tmp")
        tmp.write_text(json.dumps({"model": LLM_MODEL, "input": prompt, "response": response}, indent=1))
        tmp.rename(path)
        return response


def parse_json(text):
    m = re.search(r"\{.*\}", text, re.S)
    if not m:
        return None
    try:
        return json.loads(m.group(0))
    except json.JSONDecodeError:
        return None


BRIEF = """\
You are one unit in a squad of {allies} fighting {enemies} enemy units on a 32 by 32 map (x grows east, y grows
north). Every unit in your squad is a separate agent, like you: you control only your own unit, and the others
decide for themselves. You cannot see inside their heads; you only see their current orders and what they say
on team chat. Every half second of game time you give your unit one order:

  attack E<n>    shoot enemy E<n> when it is in range, otherwise walk toward it
  move <x> <y>   walk to a point without shooting
  hold           stand still and shoot the closest enemy in range
  retreat        walk away from the closest enemy without shooting

A unit dies at 0 health. The battle is won when every enemy is dead.
The enemy units each shoot the closest of you in range and keep shooting it until it dies."""

TURN = """\
{brief}
{rules}
Time: {seconds:.1f} seconds into the battle.
{view}
{chat}
Reply with JSON only: {{"order": "<your order>", "say": "<optional: at most 15 words to your team>"}}"""


def chat_lines(messages):
    if not messages:
        return "Team chat: nothing said yet."
    return "Team chat (most recent last):\n" + "\n".join(f"  {m['from']} ({m['t']:.1f}s): {m['text']}"
                                                         for m in messages[-8:])


def rules_text(rules):
    if not rules:
        return ""
    return ("\nYour squad's RULES.md, written by your squad after earlier battles:\n"
            + "\n".join(f"  {i}. {r}" for i, r in enumerate(rules, 1)) + "\n")


class ChatTeam:
    """Our side: one Claude agent per unit, coordinating over team chat.

    With `rules`, every prompt also shows the squad's RULES.md. Nothing checks
    that an order follows them."""

    def __init__(self, llm, rules=()):
        self.llm = llm
        self.rules = list(rules)
        self.messages = []

    def decide(self, battle):
        brief = BRIEF.format(allies=battle.allies, enemies=battle.enemies)
        alive = [u.name for u in battle.units()[: battle.allies] if u.alive]
        seconds = battle.step_count * 0.5
        heard = list(self.messages)
        rules = rules_text(self.rules)
        prompts = {name: TURN.format(brief=brief, rules=rules, seconds=seconds, view=battle.view(name),
                                     chat=chat_lines(heard))
                   for name in alive}
        with concurrent.futures.ThreadPoolExecutor(max_workers=len(prompts) or 1) as pool:
            answers = dict(zip(prompts, pool.map(self.llm.ask, prompts.values())))
        decisions = {}
        for name in sorted(answers):
            reply = parse_json(answers[name]) or {}
            order = parse_order(str(reply.get("order", "")))
            said = str(reply.get("say", "") or "").strip()[:120]
            decisions[name] = {"order": order or battle.orders[name], "raw": reply.get("order"),
                               "valid": order is not None, "say": said}
            if order:
                battle.orders[name] = order
            if said:
                self.messages.append({"from": name, "t": seconds, "text": said})
        return decisions


RETRO = """\
{brief}

Your squad just fought a battle on map {scenario} and {outcome}. Here is how it went, one line per half second:
our units' orders and health, the enemies' health, and what was said on team chat.

{log}

{rules_now}
Write ONE new rule for your squad's RULES.md that would have helped you win this battle. Every unit in your
squad will read RULES.md before every order in later battles. Make it a concrete rule about what a unit should
do, in one sentence, and do not repeat a rule already there.
Reply with JSON only: {{"rule": "<the rule>"}}"""


def battle_log(record):
    allies = sum(1 for u in record["frames"][0]["units"] if u["name"].startswith("A"))
    lines = []
    for f in record["frames"]:
        ours = "  ".join(f"{u['name']} {f['orders'].get(u['name'], '-')} ({u['health']:.0f}hp)" if u["alive"]
                         else f"{u['name']} dead" for u in f["units"][:allies])
        theirs = "  ".join(f"{u['name']} {u['health']:.0f}hp" if u["alive"] else f"{u['name']} dead"
                           for u in f["units"][allies:])
        lines.append(f"{f['step'] * 0.5:4.1f}s  {ours}  |  {theirs}")
        for name, d in sorted(f.get("decisions", {}).items()):
            if d.get("say"):
                lines.append(f"        {name} said: {d['say']}")
    return "\n".join(lines)


def write_rule(llm, record, rules):
    allies = sum(1 for u in record["frames"][0]["units"] if u["name"].startswith("A"))
    enemies = len(record["frames"][0]["units"]) - allies
    rules_now = ("RULES.md now:\n" + "\n".join(f"  {i}. {r}" for i, r in enumerate(rules, 1))
                 if rules else "RULES.md is empty.")
    prompt = RETRO.format(brief=BRIEF.format(allies=allies, enemies=enemies), scenario=record["scenario"],
                          outcome="lost" if record["outcome"] == "lost" else "did not win",
                          log=battle_log(record), rules_now=rules_now)
    reply = parse_json(llm.ask(prompt)) or {}
    rule = str(reply.get("rule", "")).strip()
    return rule or None
