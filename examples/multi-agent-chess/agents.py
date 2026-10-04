"""The agents: one per piece, each with its own traits."""

import dataclasses
import random

import chess

from engine import VALUE


@dataclasses.dataclass
class Traits:
    panic: float = 0.0  # a threatened piece runs, whatever the plan says
    greed: float = 0.0  # a piece grabs a capture, whatever the plan says
    amnesia: float = 0.0  # the piece the team chose forgets the plan
    compliance: float = 0.5  # how often a written rule holds the agent back


@dataclasses.dataclass
class Attempt:
    agent: str
    kind: str  # plan, panic, greed, amnesia, rogue
    move: chess.Move


# A pawn's personality multiplies each fault rate by one of these, and sets
# its compliance to one of those. Both average to the uniform agent's.
PERSONALITY_SCALE = (0.0, 0.5, 1.0, 1.5, 2.0)
PERSONALITY_COMPLIANCE = (0.0, 0.25, 0.5, 0.75, 1.0)


def initial_agents(board):
    """Square -> agent name. An agent is named for its piece and home square."""
    return {
        sq: f"{chess.piece_name(p.piece_type)}_{chess.square_name(sq)}"
        for sq, p in board.piece_map().items()
    }


def make_traits(names, rate, compliance, pawns, seed):
    """Every agent's traits. With pawns="personalities", each pawn differs.

    The pawns on a file share a personality on both sides, so neither side
    draws the better pawns.
    """
    traits = {n: Traits(rate, rate, rate, compliance) for n in names}
    if pawns == "personalities":
        rng = random.Random(f"{seed}-pawns")
        for f in "abcdefgh":
            scale = [rng.choice(PERSONALITY_SCALE) for _ in range(3)]
            t = Traits(
                min(0.9, rate * scale[0]),
                min(0.9, rate * scale[1]),
                min(0.9, rate * scale[2]),
                rng.choice(PERSONALITY_COMPLIANCE),
            )
            for rank in "27":
                traits[f"pawn_{f}{rank}"] = dataclasses.replace(t)
    return traits


def choose_rogue(seed, roster):
    return random.Random(f"{seed}-rogue").choice([n for n in roster if not n.startswith("king")])


def follow_move(board, move, at):
    """Move agent names with the pieces. Returns the captured agent, if any."""
    captured = None
    if board.is_en_passant(move):
        captured = at.pop(move.to_square + (-8 if board.turn == chess.WHITE else 8), None)
    elif board.is_capture(move):
        captured = at.pop(move.to_square, None)
    at[move.to_square] = at.pop(move.from_square)
    if board.is_castling(move):
        rank = chess.square_rank(move.from_square)
        if chess.square_file(move.to_square) == 6:
            at[chess.square(5, rank)] = at.pop(chess.square(7, rank))
        else:
            at[chess.square(3, rank)] = at.pop(chess.square(0, rank))
    return captured


def threatened(board, sq):
    piece = board.piece_at(sq)
    attackers = board.attackers(not piece.color, sq)
    if not attackers:
        return False
    if not board.is_attacked_by(piece.color, sq):
        return True
    cheapest = min(VALUE[board.piece_type_at(a)] or 10_000 for a in attackers)
    return cheapest < VALUE[piece.piece_type]


def pick_plan(scores, rng, margin):
    best = max(scores.values())
    near = sorted((m for m, s in scores.items() if s >= best - margin), key=lambda m: m.uci())
    return rng.choice(near)


def attempts_for_turn(board, scores, plan, at, traits, rng, rogue):
    """What each agent of the side to move sends this turn.

    The first attempt is the chosen agent's: the plan, or an amnesiac's move.
    The rest are the agents that went their own way. `rogue` is the
    compromised agent, once it is active.
    """
    own = {}
    for m in scores:
        own.setdefault(m.from_square, []).append(m)
    chosen = Attempt(at[plan.from_square], "plan", plan)
    others = []
    for sq in sorted(own):
        name, moves = at[sq], own[sq]
        piece = board.piece_at(sq)
        t = traits[name]
        if name == rogue:
            worst = min(moves, key=lambda m: (scores[m], m.uci()))
            if sq == plan.from_square:
                if worst != plan:
                    chosen = Attempt(name, "rogue", worst)
            elif worst != plan:
                others.append(Attempt(name, "rogue", worst))
            continue
        if sq == plan.from_square:
            if rng.random() < t.amnesia:
                rest = sorted((m for m in moves if m != plan), key=lambda m: m.uci())
                if rest:
                    chosen = Attempt(name, "amnesia", rng.choice(rest))
            continue
        if piece.piece_type != chess.KING and threatened(board, sq) and rng.random() < t.panic:
            others.append(Attempt(name, "panic", max(moves, key=lambda m: (scores[m], m.uci()))))
            continue
        captures = [m for m in moves if board.is_capture(m)]
        if captures and rng.random() < t.greed:
            grab = max(
                captures,
                key=lambda m: (VALUE[board.piece_type_at(m.to_square) or chess.PAWN], scores[m], m.uci()),
            )
            others.append(Attempt(name, "greed", grab))
    return [chosen] + others
