#!/usr/bin/env python3
"""Check what the table's contract guarantees, against a real `modal`.

  python3 check_contract.py

Plays a few commits by hand on two small tables, one per mode, and checks
each is accepted or refused as the contract says it must be. Exits non-zero
on the first surprise.
"""

import os
import pathlib
import sys
import tempfile

from contract import ContractTable
from treaties import Promise

MODAL = os.environ.get("MODAL", "modal")
failures = 0


def expect(what, ok, want):
    global failures
    mark = "ok  " if ok == want else "FAIL"
    failures += ok != want
    print(f"{mark} {what}: {'accepted' if ok else 'refused'}")


def strict(root):
    t = ContractTable(root, ["FRANCE", "GERMANY"], MODAL, "S1901M", "strict")
    pact = Promise("pact", "FRANCE", "GERMANY", ["S1901M"], provinces=["MUN"])
    expect("France and Germany sign a pact (France stays out of Munich)", t.promise(pact)[0], True)
    expect("France breaks it: A BUR - MUN", t.submit("FRANCE", "S1901M", ["A BUR - MUN"])[0], False)
    expect("Germany writes France's orders", t._commit_as("germany", "/orders/france/s1901m/to/bur.text",
                                                          "A PAR - BUR"), False)
    support = Promise("support", "GERMANY", "FRANCE", ["S1901M"], unit="MUN", order="A MUN S A PAR - BUR")
    expect("Germany promises a support", t.promise(support)[0], True)
    expect("Germany leaves the support out", t.submit("GERMANY", "S1901M", ["A MUN - RUH"])[0], False)
    expect("Germany gives the support", t.submit("GERMANY", "S1901M", ["A MUN S A PAR - BUR"])[0], True)
    expect("France keeps the pact", t.submit("FRANCE", "S1901M", ["A PAR - BUR"])[0], True)
    expect("France moves the turn on", t._commit_as("france", "/turn/phase.text", "f1901m"), False)


def deposit(root):
    t = ContractTable(root, ["FRANCE", "GERMANY"], MODAL, "S1901M", "deposit")
    pact = Promise("pact", "FRANCE", "GERMANY", ["S1901M"], provinces=["MUN"])
    expect("France and Germany sign a pact, with France's deposit in escrow", t.promise(pact, 1.0)[0], True)
    e = t.escrows[0]
    loose = (f"    p0 --> p0: +signed_by(/powers/germany.id) +sets(/escrow/{e['id']}/holder.text, \"germany\") "
             f"-modifies(/orders) -modifies(/turn)")
    expect("Germany posts a model that lets it take the deposit without evidence",
           t._commit_with_model("germany", f"/escrow/{e['id']}/holder.text", "germany", loose), False)
    expect("Germany takes the deposit before any betrayal", t.claim(e["id"], "A BUR - MUN", "S1901M")[0], False)
    expect("France breaks the pact: A BUR - MUN", t.submit("FRANCE", "S1901M", ["A BUR - MUN"])[0], True)
    expect("Germany takes the deposit, citing the betrayal", t.claim(e["id"], "A BUR - MUN", "S1901M")[0], True)
    expect("France takes it back alone", t._commit_as("france", f"/escrow/{e['id']}/holder.text", "france"), False)
    expect("Germany hands it to Italy", t._commit_as("germany", f"/escrow/{e['id']}/holder.text", "italy"), False)


def _commit_as(self, signer, path, value):
    file = self._write(path, value)
    ok, _ = self._commit([signer], "probe")
    if not ok:
        file.unlink()
    return ok


def _commit_with_model(self, signer, path, value, extra_edge):
    file = self._write(path, value)
    model = self.root / "model" / "default.modality"
    saved = model.read_text()
    lines = saved.splitlines()
    model.write_text("\n".join(lines[:-2] + [extra_edge] + lines[-2:]) + "\n")
    ok, _ = self._commit([signer], "probe")
    if not ok:
        file.unlink()
        model.write_text(saved)
    return ok


ContractTable._commit_as = _commit_as
ContractTable._commit_with_model = _commit_with_model

if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as tmp:
        print("strict")
        strict(pathlib.Path(tmp) / "strict")
        print("deposit")
        deposit(pathlib.Path(tmp) / "deposit")
    print("all as expected" if not failures else f"{failures} unexpected")
    sys.exit(1 if failures else 0)
