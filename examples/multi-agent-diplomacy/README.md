# Multi-agent Diplomacy: promises as contracts

Seven scripted powers play standard Diplomacy, on the `diplomacy` package's
adjudicator. Every spring, neighbours sign non-aggression pacts for the year;
every turn, pact partners promise to support each other's attacks. When
breaking a promise pays — a supply centre to take, or a better use for a
promised supporter — a power breaks it with probability `--temptation`.

One game in all three worlds, with the results, is at
[www.modality.org/experiments/multi-agent-diplomacy](https://www.modality.org/experiments/multi-agent-diplomacy)
(data from `site.py`).

Nothing here plays with a language model: the powers are greedy for supply
centres, and the promises and betrayals are scripted. What changes between
the setups is what a promise is.

| `--coordination` | A promise is | A betrayal |
|---|---|---|
| `chat` | Words | Played. The betrayed power never deals with the betrayer again. |
| `contract` | A rule both powers sign in a Modality contract | Refused by `modal`; the power plays what it promised |
| `deposit` | A rule both sign, with the promiser's deposit in escrow | Played, and the victim takes the deposit, citing the betrayal |

In `deposit`, a power breaks a promise only when it values the gain (between
half a supply centre and one and a half) above the deposit, so deception stays
possible and has a price. Score is supply centres plus deposits won or lost.

## The contract

One contract per game. Each power's orders for a movement phase are a commit
under `/orders/<power>/<phase>/`, signed with its key; a move goes in
`to/<province>.text`, a support in `support/<unit>.text`, with a `done.text`
marker. The referee alone advances `/turn`. `contract.py` describes the rules
and the witness model; in short:

- Pact, strict: `always([+modifies(/orders/france/s1902m/to/mun.text)] false)`
- Support, strict: `always([+modifies(/orders/germany/s1902m) -sets(<support path>, "<order>")] false)`
- Deposit: only the victim may change `/escrow/<id>/holder.text`, only to
  itself, and only once the evidence is in the contract's state: the
  promiser's order into a protected province, or its orders for the phase
  without the promised support (`state_exists`, `text_eq`).

`check_contract.py` plays those cases by hand against a real `modal`,
including a victim posting a model that would let it take a deposit without
evidence (refused).

After each contract game, the orders the referee played are read back from
the contract's log and compared (`audit` in the results).

## Run it

```bash
pip install -r requirements.txt
# a current modal on PATH, or MODAL=/path/to/modal

python3 check_contract.py
python3 diplomacy_demo.py play --coordination chat --seed 3
python3 diplomacy_demo.py play --coordination deposit --deposit 1 --seed 3
python3 diplomacy_demo.py experiments --games 16 --jobs 8
```

A chat game takes about a second; a contract game a few minutes, most of it
in `modal`.

| File | What |
|---|---|
| `diplomacy_demo.py` | Games, experiments, audit |
| `bots.py` | The powers' orders |
| `treaties.py` | Pacts, supports, and the temptation to break them |
| `contract.py` | The table's Modality contract |
| `check_contract.py` | What the contract refuses and accepts, by hand |
| `site.py` | The data behind the experiment page |
