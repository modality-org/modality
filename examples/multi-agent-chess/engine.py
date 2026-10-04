"""The engine every agent shares: one reply deep, then captures."""

import chess

VALUE = {
    chess.PAWN: 100,
    chess.KNIGHT: 320,
    chess.BISHOP: 330,
    chess.ROOK: 500,
    chess.QUEEN: 900,
    chess.KING: 0,
}
MATE = 100_000
INF = 10 * MATE


def positional(piece, sq, queens_on):
    f, r = chess.square_file(sq), chess.square_rank(sq)
    centre = 3.5 - max(abs(f - 3.5), abs(r - 3.5))  # 0 at the edge, 3 in the middle
    t = piece.piece_type
    if t == chess.PAWN:
        advance = r - 1 if piece.color == chess.WHITE else 6 - r
        return advance * 8 + (int(centre * 4) if 2 <= f <= 5 else 0)
    if t in (chess.KNIGHT, chess.BISHOP):
        return int(centre * 10)
    if t == chess.QUEEN:
        return int(centre * 3)
    if t == chess.KING:
        return -int(centre * 10) if queens_on else int(centre * 8)
    return 0


def evaluate(board):
    """Score for the side to move."""
    pieces = board.piece_map()
    queens_on = any(p.piece_type == chess.QUEEN for p in pieces.values())
    score = 0
    for sq, p in pieces.items():
        v = VALUE[p.piece_type] + positional(p, sq, queens_on)
        score += v if p.color == chess.WHITE else -v
    return score if board.turn == chess.WHITE else -score


def capture_order(board, move):
    victim = board.piece_type_at(move.to_square) or chess.PAWN
    attacker = board.piece_type_at(move.from_square)
    return VALUE[victim] * 10 - VALUE[attacker]


def quiesce(board, alpha, beta, depth=0):
    stand = evaluate(board)
    if stand >= beta or depth >= 4:
        return stand
    alpha = max(alpha, stand)
    captures = [m for m in board.legal_moves if board.is_capture(m)]
    captures.sort(key=lambda m: capture_order(board, m), reverse=True)
    for m in captures:
        board.push(m)
        s = -quiesce(board, -beta, -alpha, depth + 1)
        board.pop()
        if s >= beta:
            return s
        alpha = max(alpha, s)
    return alpha


def search(board, depth, alpha, beta, ply):
    if board.is_checkmate():
        return -MATE + ply
    if board.is_stalemate() or board.is_insufficient_material():
        return 0
    if depth == 0:
        return quiesce(board, alpha, beta)
    moves = sorted(
        board.legal_moves,
        key=lambda m: capture_order(board, m) if board.is_capture(m) else -INF,
        reverse=True,
    )
    best = -INF
    for m in moves:
        board.push(m)
        s = -search(board, depth - 1, -beta, -alpha, ply + 1)
        board.pop()
        best = max(best, s)
        alpha = max(alpha, s)
        if alpha >= beta:
            break
    return best


def score_moves(board, depth):
    """Every legal move with its exact score, for the side to move."""
    scores = {}
    for m in board.legal_moves:
        board.push(m)
        scores[m] = -search(board, depth - 1, -INF, INF, 1)
        board.pop()
    return scores


def material(board):
    return sum(
        VALUE[p.piece_type] * (1 if p.color == chess.WHITE else -1)
        for p in board.piece_map().values()
    )
