"""How a side's moves reach the referee.

On both sides a piece can only move itself. An agent can talk about any move,
but the only move it can send is one of its own piece's.

Chat: each agent can send its own move, and the referee plays the first it
gets. With RULES.md, agents also keep a shared file of rules that any of them
can edit and nothing checks.

Contract: a move is the /moves/<agent>.text of an accepted commit, and the
referee plays the first accepted commit for the ply. One rule, with the path
variable $k, says only a piece's own key can write its path. The preset
contract adds contract/rules.txt. A self-ruled contract starts with only that
rule; its agents add more at retros, and `modal` checks every commit against
them.
"""

import itertools
import json
import pathlib
import re
import shutil
import subprocess

import chess

import retro
from agents import Attempt

HERE = pathlib.Path(__file__).resolve().parent


class Refused(Exception):
    pass


STALL = re.compile(r"^no move was accepted \((.*)\), so ")
SENT = re.compile(r"^(\S+) \((\w+)\) (\S+): (.+)$")


def earlier_attempts(events):
    """Moves sent in rounds where nothing was accepted. A ply records only its
    last round's attempts; the earlier rounds are in its stall events."""
    out = []
    for e in events:
        m = STALL.match(e["text"]) if e["type"] == "stall" else None
        for part in (m.group(1).split("; ") if m else []):
            sent = SENT.match(part)
            if sent:
                out.append({"agent": sent[1], "kind": sent[2], "san": sent[3], "outcome": sent[4]})
    return out


def why_refused(output):
    m = re.search(r"failed predicates: ([^;]*)", output)
    if m:
        return m.group(1).strip()
    m = re.search(r"(Model violates rule[^;]*)", output)
    if m:
        return m.group(1).strip()
    lines = [l for l in output.strip().splitlines() if l.strip()]
    return lines[0].strip() if lines else "refused"


# ---------------------------------------------------------------------------
# Chat, with or without RULES.md


class Chat:
    def __init__(self, root, rules_md, llm=None, max_retros=3):
        self.root = pathlib.Path(root)
        self.rules_md = rules_md
        self.llm = llm
        self.max_retros = max_retros
        self.rules = []  # {"rule", "covers", "ply", "author"}
        self.retros = 0
        self.incidents = []
        if rules_md:
            self.root.mkdir(parents=True, exist_ok=True)
            self._write()

    @property
    def label(self):
        return "chat with RULES.md" if self.rules_md else "chat"

    def _write(self):
        lines = ["# RULES.md", ""] + [f"- {r['rule']}" for r in self.rules]
        (self.root / "RULES.md").write_text("\n".join(lines) + "\n")

    def resolve(self, ply, attempts, plan, rng, traits, rogue, roster):
        events, outcomes, sent = [], [], []
        if self.rules_md and rogue and self.rules:
            events.append({"type": "wipe", "agent": rogue,
                           "text": f"{rogue} emptied RULES.md ({len(self.rules)} rules)"})
            self.rules = []
            self._write()
        for a in attempts:
            covering = [r for r in self.rules if a.kind in r["covers"]]
            if a.kind in retro.KINDS and a.agent != rogue and covering and rng.random() < traits[a.agent].compliance:
                outcomes.append((a, "held back", f"RULES.md: {covering[0]['rule']}"))
                if a is attempts[0]:
                    sent.append(Attempt(a.agent, "plan", plan))
                continue
            sent.append(a)
        order = list(sent)
        rng.shuffle(order)
        played = order[0]
        for a in sent:
            if a is played:
                outcomes.append((a, "played", None))
            else:
                outcomes.append((a, "too late", "the referee had already taken another move"))
        return played.move, outcomes, events

    def after_ply(self, record, incident, roster, rogue=None):
        if not incident:
            return []
        self.incidents.append(incident)
        if not self.rules_md or self.retros >= self.max_retros:
            return []
        self.retros += 1
        author = record["plan"]["agent"]
        out = retro.write_white_rule(self.llm, author, incident, self.incidents[:-1],
                                     [r["rule"] for r in self.rules])
        event = {"type": "rule", "side": "white", "agent": author, "ply": record["ply"],
                 "rule": out["rule"], "covers": out["covers"], "response": out["response"]}
        if out["rule"]:
            self.rules.append({"rule": out["rule"], "covers": out["covers"], "ply": record["ply"], "author": author})
            self._write()
            event["outcome"] = "written"
            event["text"] = f"{author} added to RULES.md: {out['rule']}"
        else:
            event["outcome"] = "no rule"
            event["text"] = f"{author} wrote no rule"
        return [event]

    def report(self):
        return {"retros": self.retros, "rules": self.rules}


# ---------------------------------------------------------------------------
# Witness models for rules added one at a time
#
# A model posted with a new rule must replay the whole history, and the rule
# must hold from the state the rule's commit reaches. The model below has one
# epoch per rule. Epoch 0 accepts anything. A commit that adds rule k moves
# from epoch k-1 to epoch k, and epoch k's edges carry the labels of rules 1
# to k, taken from `modal model synthesize` for each rule and run side by side.


EDGE = re.compile(r"^\s*(\w+)\s*-->\s*(\w+)\s*(?::\s*(.*))?$")


def split_labels(text):
    out, depth, cur = [], 0, ""
    for ch in text or "":
        depth += ch == "("
        depth -= ch == ")"
        if ch.isspace() and depth == 0:
            if cur:
                out.append(cur)
            cur = ""
            continue
        cur += ch
    if cur:
        out.append(cur)
    return out


def parse_parts(text):
    parts, current = [], None
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("part "):
            current = {"initial": None, "edges": {}}
            parts.append(current)
            continue
        m = re.match(r"^initial\s+(\w+)", s)
        if m and current is not None:
            current["initial"] = m.group(1)
            continue
        m = EDGE.match(line)
        if m:
            if current is None:
                current = {"initial": None, "edges": {}}
                parts.append(current)
            src, dst, labels = m.groups()
            current["initial"] = current["initial"] or src
            current["edges"].setdefault(src, []).append((dst, split_labels(labels)))
    return parts


def merge(labels):
    """Union of labels, or None when an edge would need X and not X."""
    seen = {}
    for l in labels:
        sign, atom = l[0], l[1:]
        if seen.get(atom, sign) != sign:
            return None
        seen[atom] = sign
    return [f"{sign}{atom}" for atom, sign in seen.items()]


def epoch_model(rule_parts):
    comps = [(k, p) for k, parts in enumerate(rule_parts, 1) for p in parts]
    n = len(rule_parts)

    def active(k):
        return [i for i, (r, _) in enumerate(comps) if r <= k]

    start = (0, ())
    names = {start: "s1"}
    edges = []
    queue = [start]
    while queue:
        state = queue.pop(0)
        k, cs = state
        act = active(k)
        choices = [comps[i][1]["edges"].get(cs[j], []) for j, i in enumerate(act)]
        for combo in itertools.product(*choices):
            labels = [l for _, ls in combo for l in ls]
            dsts = tuple(d for d, _ in combo)
            targets = [((k, dsts), labels + ["-RULE"])]
            if k < n:
                fresh = tuple(comps[i][1]["initial"] for i in active(k + 1) if comps[i][0] == k + 1)
                targets.append(((k + 1, dsts + fresh), labels + ["+RULE"]))
            for target, ls in targets:
                merged = merge(ls)
                if merged is None:
                    continue
                if target not in names:
                    names[target] = f"s{len(names) + 1}"
                    queue.append(target)
                edges.append((names[state], names[target], merged))
    lines = ["model Contract {", "  part flow {", "    s0 --> s1"]
    for src, dst, labels in edges:
        lines.append(f"    {src} --> {dst}: {' '.join(labels)}")
    lines += ["  }", "}"]
    return "\n".join(lines) + "\n"


OPEN_MODEL = "model Contract {\n  part flow {\n    s0 --> s1\n    s1 --> s1\n  }\n}\n"


def move_path(agent):
    return f"/moves/{agent}.text"


# Only a piece can propose its own move: for every k, a commit that writes
# /moves/k.text must be signed by k. `$k` is a path variable.
OWNS = "[+modifies(/moves/$k.text) -signed_by(/team/pieces/$k.id)] false"
OWNS_RULE = f"always({OWNS})"


def with_ownership(model):
    """Each step after the bootstrap moves no piece, or one piece k signed by k.

    `-modifies(/moves/!$k)` is a hole: no write under any other piece's path.
    """
    movers = [["-modifies(/moves)"], ["+signed_by(/team/pieces/$k.id)", "-modifies(/moves/!$k)"]]
    lines, first = [], None
    for line in model.splitlines():
        m = EDGE.match(line)
        if not m:
            lines.append(line)
            continue
        src, dst, labels = m.groups()
        first = first or src
        if src == first:  # the bootstrap commit sets the rules up
            lines.append(line)
            continue
        for extra in movers:
            ls = merge(split_labels(labels) + extra)
            if ls is not None:
                lines.append(f"    {src} --> {dst}: {' '.join(ls)}")
    return "\n".join(lines) + "\n"


def conjuncts(formula):
    """always(A & B & ...) as [A, B, ...]; any other formula as itself."""
    f = formula.strip()
    if not (f.startswith("always(") and f.endswith(")")):
        return [f]
    body, parts, depth, cur = f[len("always("):-1].strip(), [], 0, ""
    for i, ch in enumerate(body):
        depth += ch in "(["
        depth -= ch in ")]"
        if ch == "&" and depth == 0:
            parts.append(cur)
            cur = ""
            continue
        cur += ch
    parts.append(cur)
    out = []
    for part in (x.strip() for x in parts):
        while part.startswith("(") and part.endswith(")") and balanced(part[1:-1]):
            part = part[1:-1].strip()
        out.append(part)
    return out


def implied_by_owns(conjunct):
    """True for [L] false when L holds both of OWNS's labels: OWNS already forbids
    every commit such a box forbids, whatever else L adds."""
    c = " ".join(conjunct.split())
    if not (c.startswith("[") and c.endswith("] false")):
        return False
    labels = set(split_labels(c[1:c.rindex("]")]))
    return {"+modifies(/moves/$k.text)", "-signed_by(/team/pieces/$k.id)"} <= labels


def balanced(text):
    depth = 0
    for ch in text:
        depth += ch in "(["
        depth -= ch in ")]"
        if depth < 0:
            return False
    return depth == 0


# ---------------------------------------------------------------------------
# A team contract


class ContractTeam:
    def __init__(self, root, names, modal, preset=True, llm=None, max_retros=3, pregame=False, rich=False):
        self.root = pathlib.Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.modal = modal
        self.preset = preset
        self.llm = llm
        self.max_retros = max_retros
        self.commits = {"accepted": 0, "refused": 0}
        self.rules = []  # {"rule", "formula", "ply", "author", "outcome", "detail"}
        self.retros = 0
        self.incidents = []
        self.turn = None  # the last accepted (agent, uci, ply)
        self.moves = {}  # each piece's last accepted move
        self.names = list(names)
        self.hijack_tried = False
        self._run("contract", "create")
        (self.root / "keys").mkdir(exist_ok=True)
        for name in names:
            self._run("id", "create", "--path", f"keys/{name}.passfile")
            self._run("contract", "set-named-id", f"/team/pieces/{name}.id", f"./keys/{name}.passfile")
        (self.root / "model").mkdir(exist_ok=True)
        self._run("contract", "add-rule", "--name", "owns_moves", OWNS_RULE)
        model = self.root / "model" / "default.modality"
        if preset:
            model.write_text(with_ownership((HERE / "contract" / "model" / "default.modality").read_text()))
            for line in (HERE / "contract" / "rules.txt").read_text().splitlines():
                if line.strip():
                    rule, formula = line.split(":", 1)
                    self._run("contract", "add-rule", "--name", rule.strip(), formula.strip())
                    self.rules.append({"rule": rule.strip(), "formula": formula.strip(), "ply": 0,
                                       "author": "preset", "outcome": "accepted", "detail": None})
        else:
            model.write_text(with_ownership(OPEN_MODEL))
        # The bootstrap commit: roster and model, and the preset rules. The
        # rules bind every commit after it.
        self._run("contract", "commit", "--all", "--sign", f"keys/{names[0]}.passfile", "-m", "team roster")
        self.pregame = pregame
        self.rich = rich  # retros after refused moves too, and rules aimed at one agent
        self.pregame_events = self._pregame_retro(names) if pregame else []

    @property
    def label(self):
        if self.preset:
            return "Modality contract"
        if self.rich:
            return "self-ruled Modality contract, rules from the start, richer rules"
        return "self-ruled Modality contract, rules from the start" if self.pregame else "self-ruled Modality contract"

    def _pregame_retro(self, names):
        """Before the first move, the team writes a rule from the briefing alone."""
        author = "king_e8" if "king_e8" in names else names[0]
        out = retro.write_black_rule(self.llm, author, None, [], [], rich=self.rich)
        event = {"type": "rule", "side": "black", "agent": author, "ply": 0,
                 "rule": out["rule"], "response": out["response"]}
        if not out["rule"]:
            event.update(outcome="no rule", text=f"{author} wrote no rule")
            return [event]
        formula, raw = self.llm.formula(out["rule"], self.root)
        event["formula"] = formula
        if not formula:
            event.update(outcome="refused", detail=f"no formula: {raw[:200]}",
                         text=f"{author} proposed: {out['rule']} (no formula)")
            self.rules.append({"rule": out["rule"], "formula": None, "ply": 0, "author": author,
                               "outcome": "refused", "detail": "no formula"})
            return [event]
        entry = self.add_rule(formula, list(names), author, 0, out["rule"])
        event.update(outcome=entry["outcome"], detail=entry["detail"],
                     text=f"before the game, {author} {'added' if entry['outcome'] == 'accepted' else 'proposed, refused:'} {formula}")
        return [event]

    def _modal(self, *args):
        proc = subprocess.run([self.modal, *args], cwd=self.root, capture_output=True, text=True)
        return proc.returncode == 0, (proc.stderr or "") + (proc.stdout or "")

    def _run(self, *args):
        ok, out = self._modal(*args)
        if not ok:
            raise Refused(f"modal {' '.join(args)}\n{out.strip()}")
        return out

    def _commit(self, signers, message):
        args = ["contract", "commit", "--all", "-m", message]
        for name in signers:
            args += ["--sign", f"keys/{name}.passfile"]
        ok, out = self._modal(*args)
        self.commits["accepted" if ok else "refused"] += 1
        return ok, out

    def _write_turn(self, agent, uci, ply):
        moves = self.root / "state" / "moves"
        moves.mkdir(parents=True, exist_ok=True)
        (moves / f"{agent}.text").write_text(uci)
        turn = self.root / "state" / "turn"
        turn.mkdir(parents=True, exist_ok=True)
        (turn / "ply.num").write_text(str(ply))

    def _restore_turn(self):
        """Put back the accepted state, so a refused move does not ride along."""
        moves = self.root / "state" / "moves"
        shutil.rmtree(moves, ignore_errors=True)
        shutil.rmtree(self.root / "state" / "turn", ignore_errors=True)
        if self.turn:
            moves.mkdir(parents=True)
            for agent, uci in self.moves.items():
                (moves / f"{agent}.text").write_text(uci)
            self._write_turn(*self.turn)

    def head(self):
        store = self.root / ".contract"
        head_id = (store / "HEAD").read_text().strip()
        commit = json.loads((store / "commits" / f"{head_id}.json").read_text())
        body = {a["path"]: a.get("value") for a in commit["body"] if a.get("path")}
        moved = [(p[len("/moves/"):-len(".text")], v) for p, v in body.items() if p.startswith("/moves/")]
        agent, uci = moved[0] if len(moved) == 1 else (None, None)
        return agent, uci, body.get("/turn/ply.num"), len(commit["head"]["signatures"])

    # -- moves

    def resolve(self, ply, attempts, plan, rng, traits, rogue, roster):
        events = []
        if rogue and not self.hijack_tried:
            events.append(self._hijack(rogue, ply))
        strays = {a.agent for a in attempts if a.kind != "plan"}
        endorsers = [n for n in roster if n not in strays and n != rogue]
        order = list(attempts)
        rng.shuffle(order)
        played, results = None, {}
        for a in order:
            if played is not None:
                results[id(a)] = (a, "too late", "the contract had already accepted another move")
                continue
            self._restore_turn()
            self._write_turn(a.agent, a.move.uci(), ply)
            # The piece proposes its own move; on the plan, every agent that agrees co-signs.
            signers = sorted({a.agent, *endorsers}) if a.kind == "plan" else [a.agent]
            ok, out = self._commit(signers, f"ply {ply}: {'team plan' if a.kind == 'plan' else a.agent + ' (' + a.kind + ')'}")
            if not ok:
                results[id(a)] = (a, "refused", why_refused(out))
                continue
            agent, uci, head_ply, count = self.head()
            if (agent, uci, head_ply) != (a.agent, a.move.uci(), ply):
                raise Refused(f"contract head is {agent} {uci} at ply {head_ply}, "
                              f"expected {a.agent} {a.move.uci()} at {ply}")
            self.turn = (agent, uci, ply)
            self.moves[agent] = uci
            played = a.move
            results[id(a)] = (a, "played", f"accepted with {count} of {len(roster)} signatures")
        self._restore_turn()
        return played, [results[id(a)] for a in attempts], events

    # -- rules

    def _model_for(self, formulas):
        """One epoch per rule. A rule's parts are synthesized one at a time and run
        side by side; a part that restates the per-piece rule is left to the
        ownership edges every model carries (with_ownership)."""
        rule_parts = []
        for f in formulas:
            parts = []
            for c in conjuncts(f):
                if implied_by_owns(c):
                    continue
                proc = subprocess.run(
                    [self.modal, "model", "synthesize", "--formulas", f"always({c})",
                     "-o", str(self.root / ".synth.modality")],
                    capture_output=True, text=True,
                )
                path = self.root / ".synth.modality"
                if proc.returncode != 0 or not path.exists():
                    return None, why_refused((proc.stderr or "") + (proc.stdout or ""))
                synthesized = parse_parts(path.read_text())
                path.unlink()
                if not synthesized:
                    return None, "no witness model"
                parts += synthesized
            rule_parts.append(parts)
        return epoch_model(rule_parts), None

    def _preset_model_with(self, current, formula):
        """The preset model with a one-state rule's labels on its steady loops."""
        proc = subprocess.run(
            [self.modal, "model", "synthesize", "--formulas", formula, "-o", str(self.root / ".synth.modality")],
            capture_output=True, text=True,
        )
        path = self.root / ".synth.modality"
        if proc.returncode != 0 or not path.exists():
            return None
        parts = parse_parts(path.read_text())
        path.unlink()
        if len(parts) != 1 or set(d for es in parts[0]["edges"].values() for d, _ in es) - {parts[0]["initial"]}:
            return None
        extra = [ls for es in parts[0]["edges"].values() for _, ls in es]
        lines = ["model Contract {", "  part flow {"]
        for part in parse_parts(current):
            for src, edges in part["edges"].items():
                for dst, labels in edges:
                    combos = [merge(labels + e) for e in extra] if src == dst else [labels]
                    for ls in combos:
                        if ls is not None:
                            lines.append(f"    {src} --> {dst}" + (f": {' '.join(ls)}" if ls else ""))
        return "\n".join(lines + ["  }", "}"]) + "\n"

    def add_rule(self, formula, signers, author, ply, text):
        entry = {"rule": text, "formula": formula, "ply": ply, "author": author}
        name = f"rule_{len(self.rules) + 1}"
        model_path = self.root / "model" / "default.modality"
        previous = model_path.read_text()
        if self.preset:
            model = self._preset_model_with(previous, formula)
        else:
            accepted = [r["formula"] for r in self.rules if r["outcome"] == "accepted"]
            model, why = self._model_for(accepted + [formula])
            if model is None:
                entry.update(outcome="refused", detail=f"no witness model: {why}")
                self.rules.append(entry)
                return entry
        if model:
            model_path.write_text(model if self.preset else with_ownership(model))
        ok, out = self._modal("contract", "add-rule", "--name", name, formula)
        rule_file = self.root / "rules" / f"{name}.modality"
        if ok:
            self._restore_turn()
            ok, out = self._commit(signers, f"ply {ply}: rule from {author}")
        if ok:
            entry.update(outcome="accepted", detail=None)
        else:
            entry.update(outcome="refused", detail=why_refused(out))
            rule_file.unlink(missing_ok=True)
            model_path.write_text(previous)
        self.rules.append(entry)
        return entry

    def _hijack(self, rogue, ply):
        self.hijack_tried = True
        formula = f"always([-signed_by(/team/pieces/{rogue}.id)] false)"
        entry = self.add_rule(formula, [rogue], rogue, ply, f"every commit must be signed by {rogue}")
        verb = "took over the contract" if entry["outcome"] == "accepted" else "tried to take over the contract"
        return {"type": "hijack", "side": "black", "agent": rogue, "ply": ply, "formula": formula,
                "outcome": entry["outcome"], "detail": entry["detail"],
                "text": f"{rogue} {verb} with a rule only it can sign"}

    def contained(self, record):
        """What a refused off-plan move did, for a retro on a rich contract."""
        refused = [a for a in record["attempts"] if a["kind"] != "plan" and a["outcome"] == "refused"]
        refused += [a for a in earlier_attempts(record.get("events", [])) if a["kind"] != "plan"]
        if not refused:
            return None
        n = (record["ply"] + 1) // 2
        who = ", ".join(f"{a['agent']} ({a['kind']}) sent {a['san']}" for a in refused)
        return (f"On move {n}... the team planned {record['plan']['san']} ({record['plan']['agent']}), "
                f"and {who} instead. The contract refused {'it' if len(refused) == 1 else 'them'}, "
                f"and {record['played']['san']} was played.")

    def after_ply(self, record, incident, roster, rogue=None):
        if not incident and self.rich and not self.preset:
            incident = self.contained(record)
        if not incident:
            return []
        self.incidents.append(incident)
        if self.preset or self.retros >= self.max_retros:
            return []
        self.retros += 1
        author = record["plan"]["agent"]
        accepted = [r for r in self.rules if r["outcome"] == "accepted"]
        out = retro.write_black_rule(self.llm, author, incident, self.incidents[:-1],
                                     [r["rule"] for r in accepted], rich=self.rich)
        event = {"type": "rule", "side": "black", "agent": author, "ply": record["ply"],
                 "rule": out["rule"], "response": out["response"]}
        if not out["rule"]:
            event.update(outcome="no rule", text=f"{author} wrote no rule")
            return [event]
        formula, raw = self.llm.formula(out["rule"], self.root)
        event["formula"] = formula
        if not formula:
            event.update(outcome="refused", detail=f"no formula: {raw[:200]}",
                         text=f"{author} proposed: {out['rule']} (no formula)")
            self.rules.append({"rule": out["rule"], "formula": None, "ply": record["ply"], "author": author,
                               "outcome": "refused", "detail": "no formula"})
            return [event]
        signers = [n for n in roster if n != rogue]
        entry = self.add_rule(formula, signers, author, record["ply"], out["rule"])
        event.update(outcome=entry["outcome"], detail=entry["detail"])
        verb = "added" if entry["outcome"] == "accepted" else "proposed, refused:"
        event["text"] = f"{author} {verb} {formula}"
        return [event]

    def report(self):
        return {"retros": self.retros, "rules": self.rules, "owns_rule": OWNS_RULE,
                "pregame_events": self.pregame_events,
                "dir": str(self.root), **self.commits}
