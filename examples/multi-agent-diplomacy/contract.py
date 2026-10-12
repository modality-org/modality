"""The table's Modality contract: orders as commits, promises as rules.

Every power commits its orders for a movement phase under its own path,
/orders/<power>/<phase>/..., signed with its key, with a done.text marker. A
move goes in to/<province>.text (all of a power's moves into that province,
joined by "; "), a support in support/<unit>.text. The referee alone advances
/turn. A promise is a rule both powers sign, in one of two modes.

strict: a betrayal is a commit no rule allows, and `modal` refuses it.
  pact     always([+modifies(/orders/france/s1902m/to/mun.text)] false)
  support  always([+modifies(/orders/germany/s1902m) -sets(<support path>, "<order>")] false)

deposit: the promiser puts a deposit in escrow, /escrow/<id>/holder.text. A
betrayal is accepted, and is evidence: the victim alone may then take the
escrow, and only with the evidence in the contract's state.
  always([+modifies(/escrow/e1) -signed_by(/powers/<victim>.id)] false)
  always([+sets(/escrow/e1/holder.text, "<victim>") -state_exists(<evidence>) ...] false)
  always([+modifies(/escrow/e1/holder.text) -sets(... "<promiser>") -sets(... "<victim>")] false)

The witness model has a state per movement phase. The referee's edge moves
to the next phase. Each power has an edge per phase, signed by it alone,
writing only its own orders for that phase; in strict mode it carries that
phase's pacts and supports. In deposit mode an escrow's creation and its claim
each get an edge at the phase they happen in. A commit is checked against the
model it posts, and that model must keep every rule, so no posted model can
let anyone take an escrow without the evidence.
"""

import json
import pathlib
import shutil
import subprocess

from bots import parse

# The powers are known when the table opens, so ownership is one concrete rule
# per power, with no path variable to ground on every check.
def own_orders(power):
    return f"always([+modifies(/orders/{power}) -signed_by(/powers/{power}.id)] false)"

REFEREE_TURN = "always([+modifies(/turn) -signed_by(/referee.id)] false)"


def slug(text):
    return text.lower().replace("/", "_")


def order_path(power, phase, order):
    kind, loc, dest = parse(order)
    base = f"/orders/{power}/{phase.lower()}"
    if kind == "move":
        return f"{base}/to/{slug(dest)}.text"
    return f"{base}/{kind}/{slug(loc)}.text"


def done_path(power, phase):
    return f"/orders/{power}/{phase.lower()}/done.text"


class Refused(Exception):
    pass


class ContractTable:
    def __init__(self, root, powers, modal, first_phase, mode="strict"):
        self.root = pathlib.Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.modal = modal
        self.mode = mode
        self.powers = [p.lower() for p in powers]
        self.phases = []
        self.forbid = {p: [] for p in self.powers}  # strict: order paths no commit may write
        self.oblige = {}  # strict: (power, phase) -> [(path, order)]
        self.escrows = []  # deposit: {"id", "promiser", "victim", "evidence", "at", "claimed_at", "claim_label"}
        self.commits = {"accepted": 0, "refused": 0}
        self.rules = []  # {"promise", "formula", "outcome", "detail"}
        self._run("contract", "create")
        (self.root / "keys").mkdir(exist_ok=True)
        for name in ["referee"] + self.powers:
            self._run("id", "create", "--path", f"keys/{name}.passfile")
            path = "/referee.id" if name == "referee" else f"/powers/{name}.id"
            self._run("contract", "set-named-id", path, f"./keys/{name}.passfile")
        for power in self.powers:
            self._run("contract", "add-rule", "--name", f"own_{power}", own_orders(power))
        self._run("contract", "add-rule", "--name", "referee_turn", REFEREE_TURN)
        self.phases.append(first_phase.lower())
        self._write_turn()
        self._write_model()
        ok, out = self._commit(["referee"], "table opens")
        if not ok:
            raise Refused(out)

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
        before = self._count()
        args = ["contract", "commit", "--all", "-m", message]
        for name in signers:
            args += ["--sign", f"keys/{name}.passfile"]
        _, out = self._modal(*args)
        ok = self._count() > before
        self.commits["accepted" if ok else "refused"] += 1
        return ok, out

    def _count(self):
        commits = self.root / ".contract" / "commits"
        return len(list(commits.glob("*.json"))) if commits.exists() else 0

    def _write(self, path, value):
        file = self.root / "state" / path.lstrip("/")
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(value)
        return file

    def _write_turn(self):
        self._write("/turn/phase.text", self.phases[-1])

    def _write_model(self):
        escrow = " -modifies(/escrow)" if self.mode == "deposit" else ""
        lines = ["model Contract {", "  part flow {", "    s0 --> p0"]
        for i, phase in enumerate(self.phases):
            here = f"p{i}"
            lines.append(f"    {here} --> p{i + 1}: +signed_by(/referee.id) -modifies(/orders){escrow}")
            lines.append(f"    {here} --> {here}: -modifies(/orders) -modifies(/turn){escrow}")
            for power in self.powers:
                labels = [f"+signed_by(/powers/{power}.id)"]
                labels += [f"-signed_by(/powers/{o}.id)" for o in self.powers if o != power]
                labels += [f"-modifies(/orders/{o})" for o in self.powers if o != power]
                labels.append("-modifies(/turn)")
                if self.mode == "deposit":
                    labels.append("-modifies(/escrow)")
                # In this phase the power writes only this phase's orders: one label per other
                # phase that a promise names, then this phase's own promises.
                named = {path.split("/")[3] for path in self.forbid[power]}
                named |= {when for who, when in self.oblige if who == power}
                labels += [f"-modifies(/orders/{power}/{when})" for when in sorted(named - {phase})]
                labels += [f"-modifies({p})" for p in self.forbid[power] if p.split("/")[3] == phase]
                labels += [f'+sets({p}, "{o}")' for p, o in self.oblige.get((power, phase), [])]
                lines.append(f"    {here} --> {here}: " + " ".join(labels))
            for e in self.escrows:
                others = [f"-modifies(/escrow/{x['id']})" for x in self.escrows if x["id"] != e["id"]]
                holder = f"/escrow/{e['id']}/holder.text"
                if e["at"] == i:
                    lines.append(
                        f"    {here} --> {here}: +signed_by(/powers/{e['promiser']}.id) "
                        f"+signed_by(/powers/{e['victim']}.id) +sets({holder}, \"{e['promiser']}\") "
                        f"-sets({holder}, \"{e['victim']}\") -modifies(/orders) -modifies(/turn) " + " ".join(others))
                if e.get("claimed_at") == i:
                    lines.append(
                        f"    {here} --> {here}: +signed_by(/powers/{e['victim']}.id) "
                        f"+sets({holder}, \"{e['victim']}\") {e['claim_label']} "
                        f"-modifies(/orders) -modifies(/turn) " + " ".join(others))
        n = len(self.phases)
        lines.append(f"    p{n} --> p{n}: -modifies(/orders) -modifies(/turn){escrow}")
        lines += ["  }", "}"]
        (self.root / "model").mkdir(exist_ok=True)
        (self.root / "model" / "default.modality").write_text("\n".join(lines) + "\n")

    # -- the table

    def advance(self, phase):
        self.phases.append(phase.lower())
        self._write_turn()
        self._write_model()
        ok, out = self._commit(["referee"], f"phase {phase}")
        if not ok:
            raise Refused(out)

    def promise(self, p, deposit=0.0):
        """Commit a promise as a rule both powers sign. Returns (accepted, formula)."""
        promiser, to = p.promiser.lower(), p.to.lower()
        here = len(self.phases) - 1
        written = []
        if self.mode == "deposit":
            eid = f"e{len(self.escrows) + 1}"
            holder = f"/escrow/{eid}/holder.text"
            if p.kind == "pact":
                evidence = [f"/orders/{promiser}/{ph.lower()}/to/{slug(prov)}.text"
                            for ph in p.phases for prov in p.provinces]
                proof = f"always([+sets({holder}, \"{to}\") " + " ".join(f"-state_exists({e})" for e in evidence) + "] false)"
            else:
                phase = p.phases[0].lower()
                spath = order_path(promiser, phase, p.order)
                evidence = [done_path(promiser, phase), spath]
                proof = (f"always([+sets({holder}, \"{to}\") -state_exists({evidence[0]})] false) & "
                         f"always([+sets({holder}, \"{to}\") +text_eq({spath}, \"{p.order}\")] false)")
            formula = (f"always([+modifies(/escrow/{eid}) -signed_by(/powers/{to}.id)] false) & {proof} & "
                       f"always([+modifies({holder}) -sets({holder}, \"{promiser}\") -sets({holder}, \"{to}\")] false)")
            entry_escrow = {"id": eid, "promiser": promiser, "victim": to, "kind": p.kind, "evidence": evidence,
                            "order": p.order, "at": here, "deposit": deposit}
            self.escrows.append(entry_escrow)
            written.append(self._write(holder, promiser))
            p.escrow = eid
        elif p.kind == "pact":
            paths = [f"/orders/{promiser}/{ph.lower()}/to/{slug(prov)}.text" for ph in p.phases for prov in p.provinces]
            formula = " & ".join(f"always([+modifies({path})] false)" for path in paths)
            saved = list(self.forbid[promiser])
            self.forbid[promiser] += paths
        else:
            phase = p.phases[0].lower()
            path = order_path(promiser, phase, p.order)
            formula = f'always([+modifies(/orders/{promiser}/{phase}) -sets({path}, "{p.order}")] false)'
            saved = dict(self.oblige)
            self.oblige.setdefault((promiser, phase), []).append((path, p.order))
        self._write_model()
        name = f"promise_{len(self.rules) + 1}"
        self._run("contract", "add-rule", "--name", name, formula)
        ok, out = self._commit([promiser, to], p.words())
        entry = {"promise": p.words(), "formula": formula, "outcome": "accepted" if ok else "refused"}
        if not ok:  # put the model, the rule and the escrow back as they were
            (self.root / "rules" / f"{name}.modality").unlink(missing_ok=True)
            for f in written:
                f.unlink(missing_ok=True)
            if self.mode == "deposit":
                self.escrows.pop()
            elif p.kind == "pact":
                self.forbid[promiser] = saved
            else:
                self.oblige = saved
            self._write_model()
            entry["detail"] = out.strip()[-400:]
        self.rules.append(entry)
        return ok, formula

    def claim(self, eid, betrayal_order=None, phase=None):
        """The victim takes an escrow, citing the betrayal. Returns (accepted, detail)."""
        e = next(x for x in self.escrows if x["id"] == eid)
        if "claimed_at" in e:
            return False, "already taken"
        if e["kind"] == "pact":
            path = order_path(e["promiser"], phase, betrayal_order)
            e["claim_label"] = f"+state_exists({path})"
        else:
            done, spath = e["evidence"]
            e["claim_label"] = f'+state_exists({done}) -text_eq({spath}, "{e["order"]}")'
        e["claimed_at"] = len(self.phases) - 1
        holder = self._write(f"/escrow/{eid}/holder.text", e["victim"])
        self._write_model()
        ok, out = self._commit([e["victim"]], f"{e['victim']} claims {eid}")
        if not ok:
            holder.write_text(e["promiser"])
            e.pop("claimed_at")
            self._write_model()
        return ok, ("" if ok else why(out))

    def submit(self, power, phase, orders):
        """Commit a power's orders. Returns (accepted, detail); a refused set leaves no trace."""
        power = power.lower()
        files = {}
        for order in orders:
            files.setdefault(order_path(power, phase, order), []).append(order)
        files[done_path(power, phase)] = ["done"]
        written = [self._write(path, "; ".join(sorted(vals))) for path, vals in files.items()]
        ok, out = self._commit([power], f"{power} orders {phase}")
        if not ok:
            for path in written:
                path.unlink(missing_ok=True)
            base = self.root / "state" / "orders" / power / phase.lower()
            if base.exists() and not any(base.rglob("*.text")):
                shutil.rmtree(base)
        return ok, ("" if ok else why(out))

    def logged_orders(self):
        """(power, phase) -> sorted orders, read back from the contract's own log."""
        proc = subprocess.run([self.modal, "contract", "log", "--output", "json"], cwd=self.root,
                              capture_output=True, text=True)
        out = {}
        for c in json.loads(proc.stdout)["commits"]:
            commit = json.loads((self.root / ".contract" / "commits" / f"{c['id']}.json").read_text())
            for action in commit["body"]:
                path = action.get("path") or ""
                if path.startswith("/orders/") and not path.endswith("/done.text"):
                    _, _, power, phase = path.split("/")[:4]
                    out.setdefault((power, phase), []).extend(str(action.get("value")).split("; "))
        return {k: sorted(v) for k, v in out.items()}

    def report(self):
        return {"rules": self.rules, "escrows": self.escrows, "dir": str(self.root), **self.commits}


def why(output):
    for line in output.splitlines():
        if "No valid transition" in line or "violates" in line:
            return line.strip()[:240]
    return output.strip().splitlines()[-1][:240] if output.strip() else ""
