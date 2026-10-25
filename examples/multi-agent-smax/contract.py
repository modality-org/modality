"""The squad's Modality contract.

Every half second, the referee commits what it sees to /world (health, who
is alive, who is closest to whom), signed with its key. Then each unit's
agent commits its order to /orders/<unit>.text, signed with its own key.
Only accepted orders are played; a refused unit keeps its last accepted
order.

One rule comes with the contract: only a unit can write its own order. After
a battle the squad does not win, one agent writes a rule in plain language,
and `modal contract ai suggest-rule` turns it into a formula. Each battle's
contract starts with all the squad's rules and a witness model for them, and
`modal` checks every commit against them.
A rule can test the facts the referee committed (they are accepted state by
the time a unit commits) and the order a commit sets.

  /world/step.num                 half seconds since the battle began
  /world/allies_alive.num         our living units
  /world/enemies_alive.num        their living units
  /world/focus.text               the living enemy closest to our squad's centre
  /world/<unit>/hp.num            every unit's health (A0.. ours, E0.. theirs)
  /world/<unit>/alive.bool
  /world/<A>/closest.text         the living enemy closest to our unit A
  /world/<A>/in_range.num         how many enemies our unit A can shoot now
  /orders/<A>.text                our unit A's order, e.g. "attack E1"
"""

import itertools
import json
import math
import pathlib
import re
import shutil
import subprocess


class Refused(Exception):
    pass


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
# Witness models
#
# Most rules are boxes, always([L] false): no commit may meet every label of
# L. The most permissive witness is one state whose steps each falsify one
# label of L, and a set of rules takes one such label from each. A rule of
# any other shape gets a model from `modal model synthesize`, run alongside.

EDGE = re.compile(r"^\s*(\w+)\s*-->\s*(\w+)\s*(?::\s*(.*))?$")
BOX = re.compile(r"^\[(.*)\]\s*false$")


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


ATOM = re.compile(r"^([+-])(\w+)\((.*)\)$")
BOUNDS = {"num_lt": ("<", False), "num_lte": ("<=", False), "num_gt": (">", False), "num_gte": (">=", False),
          "num_eq": ("=", False)}


def args_of(text):
    out, depth, cur, quoted = [], 0, "", False
    for ch in text:
        quoted ^= ch == '"'
        if not quoted:
            depth += ch == "("
            depth -= ch == ")"
        if ch == "," and depth == 0 and not quoted:
            out.append(cur.strip())
            cur = ""
            continue
        cur += ch
    out.append(cur.strip())
    return out


def literal(text):
    t = text.strip()
    if t.startswith('"') and t.endswith('"'):
        return t[1:-1]
    try:
        return float(t)
    except ValueError:
        return None  # another path


def possible(labels):
    """False when the labels plainly cannot hold together: two values for one
    path, or numeric bounds on a path with nothing between them. `modal` checks
    the rest, and refuses a model with a step no commit can take."""
    eq, lo, hi = {}, {}, {}
    for label in labels:
        m = ATOM.match(label)
        if not m:
            continue
        sign, name, args = m.groups()
        a = args_of(args)
        if name in ("text_eq", "sets", "post_to") and len(a) == 2 and sign == "+":
            value = literal(a[1])
            if value is None:
                continue
            key = (name if name == "text_eq" else "sets", a[0])
            if eq.setdefault(key, value) != value:
                return False
        if name in BOUNDS and len(a) == 2:
            value = literal(a[1])
            if not isinstance(value, float):
                continue
            op = BOUNDS[name][0]
            if sign == "-":  # the comparison is false
                op = {"<": ">=", "<=": ">", ">": "<=", ">=": "<", "=": None}[op]
                if op is None:
                    continue
            path = a[0]
            if op in ("<", "<=", "="):
                bound = (value, op == "<")
                if path not in hi or bound < hi[path]:
                    hi[path] = bound
            if op in (">", ">=", "="):
                bound = (value, op == ">")
                if path not in lo or (bound[0], bound[1]) > lo[path]:
                    lo[path] = bound
            if path in lo and path in hi:
                (l, lstrict), (h, hstrict) = lo[path], hi[path]
                if l > h or (l == h and (lstrict or hstrict)):
                    return False
    return True


def negate(label):
    return ("-" if label[0] == "+" else "+") + label[1:]


def box_part(labels):
    """The witness for always([labels] false): one state, a step per label
    that falsifies it."""
    return {"initial": "q", "edges": {"q": [("q", [negate(l)]) for l in labels]}}


def product_model(parts):
    """All parts run side by side after a free bootstrap step."""
    start = tuple(p["initial"] for p in parts)
    names = {start: "s1"}
    edges, queue = [], [start]
    while queue:
        state = queue.pop(0)
        choices = [parts[i]["edges"].get(q, []) for i, q in enumerate(state)]
        for combo in itertools.product(*choices):
            merged = merge([l for _, ls in combo for l in ls])
            if merged is None or not possible(merged):
                continue
            target = tuple(d for d, _ in combo)
            if target not in names:
                names[target] = f"s{len(names) + 1}"
                queue.append(target)
            edges.append((names[state], names[target], merged))
    lines = ["model Contract {", "  part flow {", "    s0 --> s1"]
    lines += [f"    {a} --> {b}" + (f": {' '.join(ls)}" if ls else "") for a, b, ls in edges]
    return "\n".join(lines + ["  }", "}"]) + "\n"


OPEN_MODEL = "model Contract {\n  part flow {\n    s0 --> s1\n    s1 --> s1\n  }\n}\n"

# Only a unit can write its own order: for every k, a commit that writes
# /orders/k.text must be signed by k. `$k` is a path variable.
OWNS = "[+modifies(/orders/$k.text) -signed_by(/team/units/$k.id)] false"
OWNS_RULE = f"always({OWNS})"


def with_ownership(model):
    """Each step after the bootstrap is the referee's world commit, or one
    unit k's order, signed by k, writing no other order and no world fact.

    `-modifies(/orders/!$k)` is a hole: no write under any other unit's path.
    """
    movers = [["+signed_by(/team/referee.id)", "-modifies(/orders)"],
              ["+signed_by(/team/units/$k.id)", "-modifies(/orders/!$k)", "-modifies(/world)"]]
    lines, first = [], None
    for line in model.splitlines():
        m = EDGE.match(line)
        if not m:
            lines.append(line)
            continue
        src, dst, labels = m.groups()
        first = first or src
        if src == first:  # the bootstrap commit sets the contract up
            lines.append(line)
            continue
        for extra in movers:
            ls = merge(split_labels(labels) + extra)
            if ls is not None:
                lines.append(f"    {src} --> {dst}: {' '.join(ls)}")
    return "\n".join(lines) + "\n"


def balanced(text):
    depth = 0
    for ch in text:
        depth += ch in "(["
        depth -= ch in ")]"
        if depth < 0:
            return False
    return depth == 0


def split_and(text):
    """Top-level `&` terms of text, each without wrapping parentheses."""
    parts, depth, cur = [], 0, ""
    for ch in text:
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


def conjuncts(formula):
    """always(A & B) and always(A) & always(B) both give [A, B]; any other
    formula is one part."""
    out = []
    for term in split_and(formula.strip()):
        body = term[len("always("):-1] if term.startswith("always(") and term.endswith(")") else None
        if body is None or not balanced(body):
            return [formula.strip()]
        out += split_and(body)
    return out


def implied_by_owns(conjunct):
    """True for [L] false when L ties a write to /orders/$k to k's own key: the
    ownership edges every model carries satisfy it already."""
    c = re.sub(r"\$\w+", "$k", " ".join(conjunct.split()))
    if not (c.startswith("[") and c.endswith("] false")):
        return False
    labels = set(split_labels(c[1:c.rindex("]")]))
    writes_own = any(re.fullmatch(r"\+modifies\(/orders/\$k(\.text)?\)", l) for l in labels)
    return writes_own and "-signed_by(/team/units/$k.id)" in labels


# ---------------------------------------------------------------------------
# The squad's contract


def world_facts(battle):
    units = battle.units()
    ours = [u for u in units[: battle.allies] if u.alive]
    theirs = [u for u in units[battle.allies:] if u.alive]
    facts = {
        "/world/step.num": battle.step_count,
        "/world/allies_alive.num": len(ours),
        "/world/enemies_alive.num": len(theirs),
    }
    if ours and theirs:
        cx = sum(u.x for u in ours) / len(ours)
        cy = sum(u.y for u in ours) / len(ours)
        facts["/world/focus.text"] = min(theirs, key=lambda e: math.hypot(e.x - cx, e.y - cy)).name
    for u in units:
        facts[f"/world/{u.name}/hp.num"] = round(u.health)
        facts[f"/world/{u.name}/alive.bool"] = u.alive
    for u in units[: battle.allies]:
        if not u.alive or not theirs:
            continue
        facts[f"/world/{u.name}/closest.text"] = min(theirs, key=lambda e: math.hypot(e.x - u.x, e.y - u.y)).name
        facts[f"/world/{u.name}/in_range.num"] = sum(math.hypot(e.x - u.x, e.y - u.y) < u.attack_range
                                                    for e in theirs)
    return facts


class SquadContract:
    def __init__(self, root, units, modal, rules=()):
        self.root = pathlib.Path(root)
        shutil.rmtree(self.root, ignore_errors=True)
        self.root.mkdir(parents=True)
        self.modal = modal
        self.units = list(units)
        self.commits = {"accepted": 0, "refused": 0}
        self.orders = {}  # each unit's last accepted order
        self._run("contract", "create")
        (self.root / "keys").mkdir()
        for name in ["referee", *self.units]:
            self._run("id", "create", "--path", f"keys/{name}.passfile")
            path = "/team/referee.id" if name == "referee" else f"/team/units/{name}.id"
            self._run("contract", "set-named-id", path, f"./keys/{name}.passfile")
        (self.root / "model").mkdir(exist_ok=True)
        self.rules = list(rules)
        model, why = self.model_for([r["formula"] for r in self.rules], modal, self.root)
        if model is None:
            raise Refused(f"no witness model for the squad's rules: {why}")
        (self.root / "model" / "default.modality").write_text(model)
        self._run("contract", "add-rule", "--name", "owns_orders", OWNS_RULE)
        for i, r in enumerate(self.rules, 1):
            self._run("contract", "add-rule", "--name", f"rule_{i}", r["formula"])
        # The bootstrap commit: roster, rules and model. The rules bind every
        # commit after it.
        ok, out = self._commit(["referee", *self.units], "squad roster and rules")
        if not ok:
            raise Refused(f"the squad's rules were refused: {why_refused(out)}")

    # -- plumbing

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

    def _file(self, path):
        return self.root / "state" / path.lstrip("/")

    def _write(self, path, value):
        f = self._file(path)
        f.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(value, bool):
            value = "true" if value else "false"
        f.write_text(str(value))
        return f

    # -- each step

    def observe(self, battle):
        """The referee's commit of what it sees. Facts of dead units are kept."""
        facts = world_facts(battle)
        for path, value in facts.items():
            self._write(path, value)
        for stale in ("closest.text", "in_range.num"):
            for u in self.units:
                path = f"/world/{u}/{stale}"
                if path not in facts:
                    self._file(path).unlink(missing_ok=True)
        if "/world/focus.text" not in facts:
            self._file("/world/focus.text").unlink(missing_ok=True)
        ok, out = self._commit(["referee"], f"step {battle.step_count}: world")
        if not ok:
            raise Refused(f"the referee's world commit was refused: {out.strip()[:500]}")
        return facts

    def propose(self, unit, order, step):
        """A unit's order, as its own commit. (accepted, why refused)."""
        if self.orders.get(unit) == order:
            return True, None
        f = self._write(f"/orders/{unit}.text", order)
        ok, out = self._commit([unit], f"step {step}: {unit} {order}")
        if ok:
            self.orders[unit] = order
            return True, None
        if unit in self.orders:
            f.write_text(self.orders[unit])
        else:
            f.unlink()
        return False, why_refused(out)

    # -- rules

    @staticmethod
    def model_for(formulas, modal, scratch):
        """A witness model for all the formulas, or (None, why)."""
        parts = []
        for f in formulas:
            for c in conjuncts(f):
                if implied_by_owns(c):
                    continue
                box = BOX.match(" ".join(c.split()))
                if box:
                    parts.append(box_part(split_labels(box.group(1))))
                    continue
                out_path = pathlib.Path(scratch) / ".synth.modality"
                proc = subprocess.run([modal, "model", "synthesize", "--formulas", f"always({c})",
                                       "-o", str(out_path)], capture_output=True, text=True)
                if proc.returncode != 0 or not out_path.exists():
                    output = (proc.stderr or "") + (proc.stdout or "")
                    if "parse failed" in output:
                        return None, f"the formula does not parse: {c}"
                    return None, why_refused(output)
                synthesized = parse_parts(out_path.read_text())
                out_path.unlink()
                if not synthesized:
                    return None, "no witness model"
                parts += synthesized
        return with_ownership(product_model(parts) if parts else OPEN_MODEL), None

    def log(self):
        """Accepted commits, oldest first, with what each wrote."""
        store = self.root / ".contract"
        out = []
        head = (store / "HEAD").read_text().strip()
        while head:
            c = json.loads((store / "commits" / f"{head}.json").read_text())
            out.append({a["path"]: a.get("value") for a in c["body"] if a.get("path")})
            parents = c["head"].get("parent") or c["head"].get("parents")
            head = parents[0] if isinstance(parents, list) and parents else parents
        return list(reversed(out))


def try_rule(formula, rules, units, modal, scratch):
    """Would a contract with the squad's rules and this one be accepted?
    Returns None, or why not."""
    try:
        SquadContract(scratch, units, modal, [*rules, {"formula": formula}])
    except Refused as e:
        return str(e).splitlines()[-1][:300]
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    return None
