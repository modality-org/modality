"""A SMAX battle where every unit takes orders, and a small controller plays them.

SMAX (JaxMARL) is an open-source rewrite of the StarCraft Multi-Agent
Challenge: two small armies on a 32 by 32 map, one environment step every
half second of game time. Each unit is its own agent. An agent does not pick
raw actions; it gives its unit an order, and `act` turns the order into the
unit's action on every step until the next order:

  attack E2      shoot enemy E2 when in range, otherwise close in on it
  move 12 18     walk to a point, without shooting
  hold           stand still and shoot the closest enemy in range
  retreat        walk away from the closest enemy, without shooting

The other side is SMAX's own heuristic: each unit shoots the closest enemy in
range and keeps shooting it until it dies, and otherwise walks at the enemy.
"""

import dataclasses
import math
import re

import jax
import jax.numpy as jnp
import numpy as np
from jaxmarl.environments.smax import SMAX, map_name_to_scenario
from jaxmarl.environments.smax.heuristic_enemy import (
    create_heuristic_policy,
    get_heuristic_policy_initial_state,
)

NORTH, EAST, SOUTH, WEST, STOP = range(5)
MOVES = 5
STEP_SECONDS = 0.5  # 8 world steps of 1/16 s


@dataclasses.dataclass
class Unit:
    name: str      # A0.. for our side, E0.. for theirs
    kind: str
    x: float
    y: float
    health: float
    max_health: float
    alive: bool
    attack_range: float
    sight_range: float
    cooldown: float


ORDER = re.compile(r"^\s*(attack\s+E\d+|move\s+-?\d+(\.\d+)?\s*,?\s+-?\d+(\.\d+)?|hold|retreat)\s*$", re.I)


def parse_order(text):
    """An order in canonical form, or None."""
    if not text or not ORDER.match(text):
        return None
    words = text.replace(",", " ").split()
    head = words[0].lower()
    if head == "attack":
        return f"attack {words[1].upper()}"
    if head == "move":
        return f"move {float(words[1]):g} {float(words[2]):g}"
    return head


class Battle:
    """One battle: our units (team 0) under orders, theirs under the heuristic."""

    def __init__(self, scenario="3m", seed=0, max_steps=100):
        self.env = SMAX(scenario=map_name_to_scenario(scenario), max_steps=max_steps)
        self.scenario = scenario
        self.key = jax.random.PRNGKey(seed)
        self.key, k = jax.random.split(self.key)
        _, self.state = self.env.reset(k)
        self.allies = self.env.num_allies
        self.enemies = self.env.num_enemies
        self.enemy_agents = [f"enemy_{i}" for i in range(self.enemies)]
        policy = create_heuristic_policy(self.env, 1, shoot=True, attack_mode="closest")
        self._enemy_policy = jax.jit(jax.vmap(policy))
        self._enemy_state = jax.vmap(lambda _: get_heuristic_policy_initial_state())(jnp.arange(self.enemies))
        self.orders = {f"A{i}": "hold" for i in range(self.allies)}
        self.step_count = 0

    # The board, in plain numbers.

    def units(self):
        s = self.state
        pos = np.asarray(s.unit_positions)
        health = np.asarray(s.unit_health)
        alive = np.asarray(s.unit_alive)
        types = np.asarray(s.unit_types)
        cool = np.asarray(s.unit_weapon_cooldowns)
        out = []
        for i in range(self.allies + self.enemies):
            t = int(types[i])
            name = f"A{i}" if i < self.allies else f"E{i - self.allies}"
            out.append(Unit(name, self.env.unit_type_names[t], float(pos[i, 0]), float(pos[i, 1]),
                            float(health[i]), float(self.env.unit_type_health[t]), bool(alive[i]),
                            float(self.env.unit_type_attack_ranges[t]),
                            float(self.env.unit_type_sight_ranges[t]), float(cool[i])))
        return out

    @property
    def done(self):
        return bool(self.state.done)

    def outcome(self):
        us = self.units()
        ours = sum(u.alive for u in us[: self.allies])
        theirs = sum(u.alive for u in us[self.allies:])
        if theirs == 0 and ours > 0:
            return "won"
        if ours == 0 and theirs > 0:
            return "lost"
        return "draw"

    # From orders to actions.

    def act(self, unit, order, units):
        if not unit.alive:
            return STOP
        enemies = [u for u in units[self.allies:] if u.alive]
        def dist(u):
            return math.hypot(u.x - unit.x, u.y - unit.y)
        in_range = [u for u in enemies if dist(u) < unit.attack_range]
        def shoot(u):
            return MOVES + int(u.name[1:])
        if order.startswith("attack"):
            target = next((u for u in enemies if u.name == order.split()[1]), None)
            if target is None:  # dead: hold instead
                return shoot(min(in_range, key=dist)) if in_range else STOP
            if dist(target) < unit.attack_range:
                return shoot(target)
            return toward(unit, target.x, target.y)
        if order.startswith("move"):
            _, x, y = order.split()
            x, y = float(x), float(y)
            if math.hypot(x - unit.x, y - unit.y) < 0.5:
                return STOP
            return toward(unit, x, y)
        if order == "retreat":
            if not enemies:
                return STOP
            near = min(enemies, key=dist)
            return toward(unit, 2 * unit.x - near.x, 2 * unit.y - near.y)
        return shoot(min(in_range, key=dist)) if in_range else STOP  # hold

    def step(self):
        """One environment step (half a second) under the current orders."""
        units = self.units()
        actions = {f"ally_{i}": jnp.int32(self.act(units[i], self.orders[f"A{i}"], units))
                   for i in range(self.allies)}
        obs = self.env.get_obs_unit_list(self.state)
        enemy_obs = jnp.stack([obs[a] for a in self.enemy_agents])
        self.key, k_enemy, k_step = jax.random.split(self.key, 3)
        enemy_actions, self._enemy_state = self._enemy_policy(
            jax.random.split(k_enemy, self.enemies), self._enemy_state, enemy_obs)
        for i, a in enumerate(self.enemy_agents):
            actions[a] = enemy_actions[i]
        _, self.state, _, _, _ = self.env.step_env(k_step, self.state, actions)
        self.step_count += 1
        return units

    # What a unit sees.

    def view(self, name):
        units = self.units()
        me = next(u for u in units if u.name == name)
        def seen(u):
            return u.alive and math.hypot(u.x - me.x, u.y - me.y) <= me.sight_range
        lines = [f"You are {me.name}, a {me.kind} at ({me.x:.1f}, {me.y:.1f}) with "
                 f"{me.health:.0f}/{me.max_health:.0f} health; you shoot up to {me.attack_range:g} away "
                 f"and see up to {me.sight_range:g} away. Your order now: {self.orders[name]}."]
        lines.append("Your team:")
        for u in units[: self.allies]:
            if u.name == name:
                continue
            if not u.alive:
                lines.append(f"  {u.name}: dead")
            else:
                lines.append(f"  {u.name} {u.kind} at ({u.x:.1f}, {u.y:.1f}), {u.health:.0f}/{u.max_health:.0f} "
                             f"health, order: {self.orders[u.name]}")
        enemies = [u for u in units[self.allies:] if seen(u)]
        if enemies:
            lines.append("Enemies you can see:")
            for u in enemies:
                d = math.hypot(u.x - me.x, u.y - me.y)
                lines.append(f"  {u.name} {u.kind} at ({u.x:.1f}, {u.y:.1f}), {u.health:.0f}/{u.max_health:.0f} "
                             f"health, {d:.1f} away{' (in range)' if d < me.attack_range else ''}")
        else:
            lines.append("You see no enemies.")
        lost = sum(not u.alive for u in units[self.allies:])
        if lost:
            lines.append(f"Enemies killed so far: {lost} of {self.enemies}.")
        return "\n".join(lines)

    def frame(self):
        """The board for a replay."""
        return {
            "step": self.step_count,
            "units": [{"name": u.name, "kind": u.kind, "x": round(u.x, 2), "y": round(u.y, 2),
                       "health": round(u.health, 1), "max": u.max_health, "alive": u.alive}
                      for u in self.units()],
            "orders": dict(self.orders),
        }


def toward(unit, x, y):
    dx, dy = x - unit.x, y - unit.y
    if abs(dx) < 1e-6 and abs(dy) < 1e-6:
        return STOP
    if abs(dx) >= abs(dy):
        return EAST if dx > 0 else WEST
    return NORTH if dy > 0 else SOUTH
