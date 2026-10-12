"""Promises between powers, and the temptation to break them.

Two kinds of promise:

- A pact: the promiser won't order a move into the protected power's supply
  centres during the phases it covers (this year's spring and fall).
- A support: the promiser's unit supports the beneficiary's attack this phase.

A power keeps a promise unless breaking it pays: a pacted supply centre it
could move into, or a better use for a promised supporter. Then it breaks the
promise with probability `temptation`.
"""

import dataclasses

from bots import Board, parse, province, score_move


@dataclasses.dataclass
class Promise:
    kind: str  # pact | support
    promiser: str
    to: str  # the power it is made to
    phases: list
    provinces: list = dataclasses.field(default_factory=list)  # pact: protected centres
    unit: str = ""  # support: the promiser's unit
    order: str = ""  # support: the exact order promised
    made: str = ""  # phase it was made
    escrow: str = ""  # deposit mode: the escrow holding the promiser's deposit

    def words(self):
        if self.kind == "pact":
            return (f"{self.promiser.title()} won't move into {', '.join(self.provinces)} "
                    f"({self.to.title()}'s) in {' or '.join(self.phases)}.")
        return f"{self.promiser.title()} will order {self.order} for {self.to.title()} in {self.phases[0]}."


def neighbours(game, a, b):
    """Some unit of one power stands next to a supply centre or unit of the other."""
    board = Board(game)
    near = lambda p, q: any(province(x.upper()) in q for x in board.map.loc_abut.get(p, []))
    a_units = [province(u.split()[1]) for u in game.get_power(a).units]
    b_units = [province(u.split()[1]) for u in game.get_power(b).units]
    b_places = set(game.get_power(b).centers) | set(b_units)
    a_places = set(game.get_power(a).centers) | set(a_units)
    return any(near(u, b_places) for u in a_units) or any(near(u, a_places) for u in b_units)


def pacts_for_year(game, rng, p_pact, grudges, phases):
    """Mutual non-aggression pacts between neighbours that will still deal."""
    powers = sorted(game.powers)
    out = []
    for i, a in enumerate(powers):
        for b in powers[i + 1:]:
            if game.get_power(a).is_eliminated() or game.get_power(b).is_eliminated():
                continue
            if (a, b) in grudges or (b, a) in grudges or not neighbours(game, a, b):
                continue
            if rng.random() >= p_pact:
                continue
            for x, y in ((a, b), (b, a)):
                centres = sorted(province(c) for c in game.get_power(y).centers)
                if centres:
                    out.append(Promise("pact", x, y, list(phases), provinces=centres, made=phases[0]))
    return out


def support_deals(game, rng, p_support, pacts, plans, phase):
    """Pact partners promise to support each other's attacks on supply centres."""
    possible = game.get_all_possible_orders()
    partners = {(p.promiser, p.to) for p in pacts}
    board = Board(game)
    out, used = [], set()
    for beneficiary, orders in sorted(plans.items()):
        for order in sorted(orders.values()):
            kind, _, dest = parse(order)
            if kind != "move" or dest not in board.scs or board.owner(dest) == beneficiary:
                continue
            for promiser in sorted(game.powers):
                if (promiser, beneficiary) not in partners or rng.random() >= p_support:
                    continue
                for loc in sorted(game.get_orderable_locations(promiser)):
                    support = f"{_unit_type(game, promiser, loc)} {loc} S {order}"
                    if loc not in used and support in possible[loc]:
                        used.add(loc)
                        out.append(Promise("support", promiser, beneficiary, [phase], unit=loc, order=support,
                                           made=phase))
                        break
    return out


def _unit_type(game, power, loc):
    for unit in game.get_power(power).units:
        if unit.split()[1] == loc:
            return unit.split()[0]
    return "A"


def constraints(promises, power, phase):
    """What a power has promised this phase: provinces it won't enter, orders it will give."""
    forbidden, fixed = set(), {}
    for p in promises:
        if p.promiser != power or phase not in p.phases:
            continue
        if p.kind == "pact":
            forbidden |= set(p.provinces)
        else:
            fixed[p.unit] = p.order
    return forbidden, fixed


def betrayals(game, power, orders, promises, phase, rng, temptation, price=0.0, spent=()):
    """The orders a power would give if it broke the promises it is tempted to.

    price: what breaking a promise costs it (its deposit, in supply centres). A
    tempted power breaks a promise only if what it values the gain at, between
    0.5 and 1.5 centres, is more than that.

    Returns (orders, [(promise, order)]) with one entry per promise broken.
    """
    # A promise whose deposit the victim already took costs nothing more to break.
    worth = lambda p: rng.uniform(0.5, 1.5) > (0.0 if p.escrow in spent else price)
    if temptation <= 0:
        return orders, []
    board = Board(game)
    possible = game.get_all_possible_orders()
    out, broken = dict(orders), []
    for p in promises:
        if p.promiser != power or phase not in p.phases:
            continue
        if p.kind == "pact":
            grabs = [(score_move(board, power, parse(o)[2]), loc, o)
                     for loc in sorted(out) for o in sorted(possible[loc])
                     if parse(o)[0] == "move" and parse(o)[2] in p.provinces and "VIA" not in o]
            if grabs and rng.random() < temptation and worth(p):
                _, loc, order = max(grabs)
                out[loc] = order
                broken.append((p, order))
        elif p.unit in out and out[p.unit] == p.order:
            moves = [(score_move(board, power, parse(o)[2]), o) for o in sorted(possible[p.unit])
                     if parse(o)[0] == "move" and "VIA" not in o]
            if moves and max(moves)[0] >= 10 and rng.random() < temptation and worth(p):
                out[p.unit] = max(moves)[1]
                broken.append((p, out[p.unit]))
    return out, broken
