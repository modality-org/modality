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


# ---------------------------------------------------------------------------
# The contract world

CONTRACT_BRIEF = """
Your squad coordinates through a Modality contract. Every half second the referee commits what it sees to
the contract, and then each unit's order counts only if a commit that writes it to /orders/<unit>.text,
signed with that unit's own key, is accepted. The contract refuses any commit that breaks one of its rules;
a refused unit keeps its last accepted order.

Contract rules (the squad wrote them after earlier battles; `modal` checks every order against them):
{rules}"""

REFUSED = """
The contract refused your order "{order}": {why}
Give a different order that the contract rules allow."""

CONTRACT_PATHS = """\
The contract's state, which rules can test (the referee commits /world before the units order):
  /world/step.num                 half seconds since the battle began
  /world/allies_alive.num         our living units
  /world/enemies_alive.num        their living units
  /world/focus.text               the living enemy closest to our squad's centre, e.g. E1
  /world/<unit>/hp.num            every unit's health (A0.. ours, E0.. theirs)
  /world/<unit>/alive.bool        true or false
  /world/<A>/closest.text         the living enemy closest to our unit A
  /world/<A>/in_range.num         how many enemies our unit A can shoot now
  /orders/<A>.text                our unit A's order, e.g. "attack E1", "hold", "retreat", "move 10 12"
Each of our units' keys is /team/units/<A>.id; a commit is one unit's order, signed by that unit."""

CONTRACT_RETRO = """\
{brief}

Your squad coordinates through a Modality contract: each unit's order is a commit, and the contract refuses
any order that breaks one of its rules. It already has the rule that only a unit can write its own order.
{paths}

Your squad just fought a battle on map {scenario} and {outcome}. Here is how it went, one line per half second:
our units' orders and health, the enemies' health, what was said on team chat, and orders the contract refused.

{log}

{rules_now}
Write ONE new rule for the contract that would have helped you win this battle. It will be turned into a
Modality formula and checked against every order in later battles, so it must be about the orders units may
commit and the facts above, naming the paths. Make it one sentence, and do not repeat a rule already there.
Reply with JSON only: {{"rule": "<the rule>"}}"""


def contract_rules_text(rules):
    if not rules:
        return "  (none yet, besides: only a unit can write its own order)"
    return "\n".join(f"  {i}. {r['rule']}\n     {r['formula']}" for i, r in enumerate(rules, 1))


class ContractTeam(ChatTeam):
    """Our side: one Claude agent per unit, chat as before, and every order a
    commit to the squad's contract."""

    def __init__(self, llm, root, modal, rules=()):
        from contract import SquadContract
        super().__init__(llm)
        self.contract_rules = list(rules)
        self.contract = None
        self.root, self.modal = root, modal
        self._make = SquadContract

    def decide(self, battle):
        if self.contract is None:
            units = [f"A{i}" for i in range(battle.allies)]
            self.contract = self._make(self.root, units, self.modal, self.contract_rules)
        self.contract.observe(battle)
        brief = BRIEF.format(allies=battle.allies, enemies=battle.enemies)
        alive = [u.name for u in battle.units()[: battle.allies] if u.alive]
        seconds = battle.step_count * 0.5
        heard = list(self.messages)
        rules = CONTRACT_BRIEF.format(rules=contract_rules_text(self.contract_rules))
        base = {name: TURN.format(brief=brief, rules=rules, seconds=seconds, view=battle.view(name),
                                  chat=chat_lines(heard)) for name in alive}

        def ask_all(prompts):
            with concurrent.futures.ThreadPoolExecutor(max_workers=len(prompts) or 1) as pool:
                return dict(zip(prompts, pool.map(self.llm.ask, prompts.values())))

        decisions, said = {}, {}
        pending = base
        for attempt in range(2):
            answers = ask_all(pending)
            retry = {}
            for name in sorted(answers):
                reply = parse_json(answers[name]) or {}
                order = parse_order(str(reply.get("order", "")))
                if reply.get("say"):
                    said[name] = str(reply["say"]).strip()[:120]
                d = decisions.setdefault(name, {"refused": []})
                d.update(raw=reply.get("order"), valid=order is not None)
                if order is None:
                    continue
                ok, why = self.contract.propose(name, order, battle.step_count)
                if ok:
                    battle.orders[name] = order
                    d["accepted"] = order
                else:
                    d["refused"].append({"order": order, "why": why})
                    retry[name] = base[name] + REFUSED.format(order=order, why=why)
            if not retry:
                break
            pending = retry
        for name in sorted(decisions):
            d = decisions[name]
            d["order"] = battle.orders[name]
            d["say"] = said.get(name, "")
            if d["say"]:
                self.messages.append({"from": name, "t": seconds, "text": d["say"]})
        return decisions


def contract_battle_log(record):
    lines = battle_log(record).splitlines()
    out = []
    by_time = {}
    for f in record["frames"]:
        refused = [f"{n}'s order \"{r['order']}\" ({r['why']})" for n, d in sorted(f.get("decisions", {}).items())
                   for r in d.get("refused", [])]
        if refused:
            by_time[f"{f['step'] * 0.5:4.1f}s"] = refused
    for line in lines:
        out.append(line)
        key = line[:5]
        if key in by_time:
            out += [f"        the contract refused {r}" for r in by_time.pop(key)]
    return "\n".join(out)


def write_contract_rule(llm, record, rules):
    allies = sum(1 for u in record["frames"][0]["units"] if u["name"].startswith("A"))
    enemies = len(record["frames"][0]["units"]) - allies
    rules_now = ("Contract rules now:\n" + contract_rules_text(rules)) if rules else "The contract has no rules yet."
    prompt = CONTRACT_RETRO.format(brief=BRIEF.format(allies=allies, enemies=enemies), paths=CONTRACT_PATHS,
                                   scenario=record["scenario"],
                                   outcome="lost" if record["outcome"] == "lost" else "did not win",
                                   log=contract_battle_log(record), rules_now=rules_now)
    reply = parse_json(llm.ask(prompt)) or {}
    rule = str(reply.get("rule", "")).strip()
    return rule or None


# What `modal ai eval` tells its rule writer and `suggest-rule` does not yet:
# without it, the helper tests a unit's new order with text_eq, which reads the
# order from before the commit.
VALUES_NOTE = (" (Writing the formula: num_*, bool_* and text_* predicates read the contract's accepted state, from"
               " before the commit being checked, so they never see the value a commit writes. To test the order a"
               " commit writes, use +sets(/orders/<unit>.text, \"<order>\"). Paths: " + " ".join(
                   l.split()[0] for l in CONTRACT_PATHS.splitlines()[1:] if l.strip().startswith("/")) + ")")


def to_formula(llm, rule, contract_dir, modal):
    """Plain language to a formula, with the product's own helper (cached)."""
    rule = rule + VALUES_NOTE
    key = hashlib.sha256(f"suggest-rule\n{LLM_MODEL}\n{rule}".encode()).hexdigest()[:24]
    path = llm.cache / f"suggest-rule-{key}.json"
    if path.exists():
        out = json.loads(path.read_text())["response"]
    else:
        out = ""
        for _ in range(3):
            proc = subprocess.run([modal, "contract", "ai", "suggest-rule", "--dir", str(contract_dir), rule],
                                  capture_output=True, text=True, timeout=600)
            out = (proc.stdout or "").strip()
            if proc.returncode == 0 and out:
                break
            out = "ERROR: " + ((proc.stderr or proc.stdout) or "").strip()[:500]
        path.write_text(json.dumps({"model": LLM_MODEL, "input": rule, "response": out}, indent=1))
    lines = [l.strip().strip("`") for l in out.splitlines() if l.strip()]
    candidates = [l for l in lines if "(" in l and l.endswith(")") and not l.startswith("ERROR")]
    return (candidates[-1] if candidates else None), out
