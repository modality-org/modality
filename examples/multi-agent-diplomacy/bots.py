"""Scripted powers: greedy for supply centres, with no negotiation of their own.

A power scores each unit's possible orders, takes the best moves without
bouncing its own units, and has idle units support its own attacks. Promises
(treaties.py) constrain the orders it may give; a betrayal swaps an order out.
"""

import collections


def province(loc):
    """'STP/SC' -> 'STP'."""
    return loc.split("/")[0]


def parse(order):
    """'A PAR - BUR' -> ('move', 'PAR', 'BUR'); supports, holds and the rest likewise."""
    parts = order.split()
    unit_loc = parts[1] if len(parts) > 1 else ""
    if len(parts) >= 4 and parts[2] == "-":
        return "move", unit_loc, province(parts[3])
    if len(parts) >= 3 and parts[2] == "S":
        target = province(parts[-1]) if "-" in parts else province(parts[4] if len(parts) > 4 else parts[-1])
        return "support", unit_loc, target
    if len(parts) >= 3 and parts[2] == "C":
        return "convoy", unit_loc, province(parts[-1])
    if len(parts) >= 3 and parts[2] == "H":
        return "hold", unit_loc, province(unit_loc)
    return "other", unit_loc, ""


class Board:
    """What the bots read off the map: supply centres, owners, distances."""

    def __init__(self, game):
        self.game = game
        self.map = game.map
        self.scs = {province(s) for s in self.map.scs}
        self._dist = {}

    def owner(self, prov):
        for name, power in self.game.powers.items():
            if prov in power.centers:
                return name
        return None

    def units_at(self):
        out = {}
        for name, power in self.game.powers.items():
            for unit in power.units:
                out[province(unit.split()[1])] = name
        return out

    def distance(self, start, goals):
        """Steps from start to the nearest of goals, ignoring unit types."""
        key = (start, frozenset(goals))
        if key in self._dist:
            return self._dist[key]
        seen, queue = {start}, collections.deque([(start, 0)])
        found = 99
        while queue:
            loc, d = queue.popleft()
            if loc in goals:
                found = d
                break
            for nxt in self.map.loc_abut.get(loc, []):
                p = province(nxt.upper())
                if p not in seen:
                    seen.add(p)
                    queue.append((p, d + 1))
        self._dist[key] = found
        return found


def score_move(board, power, dest):
    owner = board.owner(dest)
    targets = {s for s in board.scs if board.owner(s) != power}
    s = 0.0
    if dest in board.scs and owner != power:
        s += 10 + (2 if owner is None else 0)
    elif dest in board.scs:
        s += 1
    if targets:
        s += 3 / (1 + board.distance(dest, targets))
    return s


def movement_orders(game, power, rng, forbidden=(), fixed=None):
    """Greedy orders for a movement phase.

    forbidden: provinces this power may not move into (its pacts).
    fixed: {unit_loc: order} it has promised (its supports).
    """
    board = Board(game)
    possible = game.get_all_possible_orders()
    locs = sorted(game.get_orderable_locations(power.upper()))
    fixed = dict(fixed or {})
    enemies = {p for p, owner in board.units_at().items() if owner != power.upper()}
    options = []
    for loc in locs:
        if loc in fixed:
            continue
        for order in sorted(possible[loc]):
            kind, unit_loc, dest = parse(order)
            if kind == "move" and "VIA" not in order and dest not in forbidden:
                options.append((score_move(board, power.upper(), dest) + rng.random() * 0.01, loc, order, dest))
            elif kind == "hold":
                mine = province(loc) in board.scs and board.owner(province(loc)) == power.upper()
                threatened = any(province(a.upper()) in enemies for a in board.map.loc_abut.get(province(loc), []))
                options.append(((6 if mine and threatened else 1) + rng.random() * 0.01, loc, order, province(loc)))
    orders, taken = dict(fixed), {parse(o)[2] for o in fixed.values() if parse(o)[0] == "move"}
    for score, loc, order, dest in sorted(options, reverse=True):
        if loc in orders:
            continue
        kind = parse(order)[0]
        if kind == "move" and dest in taken:
            continue
        orders[loc] = order
        if kind == "move":
            taken.add(dest)
    # Units left holding support an attack of their own on a supply centre.
    attacks = [o for o in orders.values() if parse(o)[0] == "move" and parse(o)[2] in board.scs]
    for loc, order in list(orders.items()):
        if parse(order)[0] != "hold" or loc in fixed:
            continue
        for attack in attacks:
            support = f"{order.split()[0]} {loc} S {attack}"
            if support in possible[loc]:
                orders[loc] = support
                break
    return orders


def other_orders(game, power, rng):
    """Retreats and builds: no promises cover them."""
    possible = game.get_all_possible_orders()
    locs = game.get_orderable_locations(power.upper())
    phase = game.get_current_phase()
    if phase.endswith("A"):
        p = game.get_power(power.upper())
        delta = len(p.centers) - len(p.units)
        orders = []
        for loc in sorted(locs):
            if delta > 0 and len(orders) < delta:
                builds = [o for o in possible[loc] if o.endswith(" B")]
                if builds:
                    orders.append(rng.choice(sorted(builds)))
            elif delta < 0 and len(orders) < -delta:
                disbands = [o for o in possible[loc] if o.endswith(" D")]
                if disbands:
                    orders.append(disbands[0])
        return orders
    return [rng.choice(sorted(possible[loc])) for loc in sorted(locs) if possible[loc]]
