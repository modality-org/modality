//! Order-constraint graph over `.num` paths and rational constants.
//!
//! Nodes are equivalence classes (union-find over `=`); edges are `<` or
//! `≤`. The literal set is unsatisfiable over ℚ iff some cycle contains a
//! strict edge, or a class contains two different constants, or a
//! disequality joins one class. Constants are pre-ordered among themselves.
//!
//! Negative literals flip only when every path they mention is forced to
//! hold a number (`-num_gt(x,5)` alone is "x ≤ 5, or x absent, or x not a
//! number"; case A13).
//!
//! Returns the offending literals, or `None` if consistent. Lean twin:
//! `orderDead` in `Decide.lean`.

use super::rational::Rational;
use super::sort::{Constraint, Lit, Op, Term};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub struct Ctx<'a> {
    pub lits: &'a [Lit],
    /// Paths known to hold a number (positive order literals, or state).
    pub numeric: &'a BTreeSet<String>,
}

struct Uf {
    parent: Vec<usize>,
}

impl Uf {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        let mut c = x;
        while self.parent[c] != r {
            let n = self.parent[c];
            self.parent[c] = r;
            c = n;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            // deterministic: smaller index is the root
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[hi] = lo;
        }
    }
}

/// An edge `from ≤ to` (`strict` = `<`), with the literals that produced it.
#[derive(Clone)]
struct Edge {
    from: usize,
    to: usize,
    strict: bool,
    why: Vec<Lit>,
}

fn term_numeric(t: &Term, numeric: &BTreeSet<String>) -> bool {
    match t {
        Term::Const(_) => true,
        Term::Path(p) => numeric.contains(p),
    }
}

/// `None` consistent; `Some(lits)` inconsistent.
#[allow(clippy::needless_range_loop)]
pub fn check(ctx: &Ctx) -> Option<Vec<Lit>> {
    // Collect terms.
    let mut terms: BTreeSet<Term> = BTreeSet::new();
    let mut used: Vec<(&Lit, &Term, Op, &Term, bool)> = Vec::new(); // (lit, lhs, op, rhs, negated)
    for lit in ctx.lits {
        if let Constraint::Order { lhs, op, rhs } = &lit.c {
            if !(lit.positive || (term_numeric(lhs, ctx.numeric) && term_numeric(rhs, ctx.numeric)))
            {
                continue; // may be satisfied by absence
            }
            terms.insert(lhs.clone());
            terms.insert(rhs.clone());
            used.push((lit, lhs, *op, rhs, !lit.positive));
        }
    }
    if used.is_empty() {
        return None;
    }
    let terms: Vec<Term> = terms.into_iter().collect();
    let index: BTreeMap<&Term, usize> = terms.iter().enumerate().map(|(i, t)| (t, i)).collect();
    let n = terms.len();
    let mut uf = Uf::new(n);

    let mut edges: Vec<Edge> = Vec::new();
    let mut diseq: Vec<(usize, usize, Lit)> = Vec::new();

    // Constants order themselves.
    let consts: Vec<(usize, Rational)> = terms
        .iter()
        .enumerate()
        .filter_map(|(i, t)| match t {
            Term::Const(c) => Some((i, *c)),
            _ => None,
        })
        .collect();
    for (i, (ia, a)) in consts.iter().enumerate() {
        for (ib, b) in consts.iter().skip(i + 1) {
            match a.cmp_exact(b) {
                Ordering::Less => edges.push(Edge {
                    from: *ia,
                    to: *ib,
                    strict: true,
                    why: vec![],
                }),
                Ordering::Greater => edges.push(Edge {
                    from: *ib,
                    to: *ia,
                    strict: true,
                    why: vec![],
                }),
                Ordering::Equal => {} // normalised rationals: cannot happen for distinct terms
            }
        }
    }

    for (lit, lhs, op, rhs, negated) in &used {
        let (l, r) = (index[lhs], index[rhs]);
        let why = vec![(*lit).clone()];
        match (op, negated) {
            (Op::Lt, false) => edges.push(Edge {
                from: l,
                to: r,
                strict: true,
                why,
            }),
            (Op::Le, false) => edges.push(Edge {
                from: l,
                to: r,
                strict: false,
                why,
            }),
            (Op::Eq, false) => uf.union(l, r),
            // ¬(l < r) ⇔ r ≤ l ; ¬(l ≤ r) ⇔ r < l ; ¬(l = r) ⇔ l ≠ r
            (Op::Lt, true) => edges.push(Edge {
                from: r,
                to: l,
                strict: false,
                why,
            }),
            (Op::Le, true) => edges.push(Edge {
                from: r,
                to: l,
                strict: true,
                why,
            }),
            (Op::Eq, true) => diseq.push((l, r, (*lit).clone())),
        }
    }

    // Two different constants in one class.
    for (i, (ia, a)) in consts.iter().enumerate() {
        for (ib, b) in consts.iter().skip(i + 1) {
            if uf.find(*ia) == uf.find(*ib) && a != b {
                return Some(eq_lits(ctx));
            }
        }
    }

    // Reachability closure over class representatives, tracking strictness.
    // reach[a][b] = Some(strict) if a ≤ b derivable (strict if some < on a path).
    let mut reach: Vec<Vec<Option<bool>>> = vec![vec![None; n]; n];
    for i in 0..n {
        reach[i][i] = Some(false);
    }
    for e in &edges {
        let (a, b) = (uf.find(e.from), uf.find(e.to));
        let cur = reach[a][b];
        reach[a][b] = Some(cur.unwrap_or(false) || e.strict);
    }
    for k in 0..n {
        for i in 0..n {
            let Some(ik) = reach[i][k] else { continue };
            for j in 0..n {
                let Some(kj) = reach[k][j] else { continue };
                let s = ik || kj;
                reach[i][j] = Some(reach[i][j].unwrap_or(false) || s);
            }
        }
    }

    // A strict edge inside a cycle: reach[b][a] exists for an edge a<b, or
    // any strict self-reach.
    for e in &edges {
        let (a, b) = (uf.find(e.from), uf.find(e.to));
        if e.strict && (a == b || reach[b][a].is_some()) {
            return Some(cycle_lits(ctx, e, &edges, &mut uf, &reach));
        }
    }
    for i in 0..n {
        if reach[i][i] == Some(true) {
            return Some(order_lits(ctx));
        }
    }

    // Disequality inside one class, or across mutually ≤-reachable classes.
    for (l, r, lit) in &diseq {
        let (a, b) = (uf.find(*l), uf.find(*r));
        if a == b || (reach[a][b].is_some() && reach[b][a].is_some()) {
            let mut why = order_lits(ctx);
            if !why.contains(lit) {
                why.push(lit.clone());
            }
            why.sort();
            return Some(why);
        }
    }

    None
}

fn order_lits(ctx: &Ctx) -> Vec<Lit> {
    let mut v: Vec<Lit> = ctx
        .lits
        .iter()
        .filter(|l| matches!(l.c, Constraint::Order { .. }))
        .cloned()
        .collect();
    v.sort();
    v
}

fn eq_lits(ctx: &Ctx) -> Vec<Lit> {
    let mut v: Vec<Lit> = ctx
        .lits
        .iter()
        .filter(|l| matches!(l.c, Constraint::Order { op: Op::Eq, .. }) && l.positive)
        .cloned()
        .collect();
    v.sort();
    v
}

/// Best-effort minimal explanation: the strict edge plus the edges that
/// close the cycle back to it. Falls back to all order literals.
fn cycle_lits(
    ctx: &Ctx,
    strict: &Edge,
    edges: &[Edge],
    uf: &mut Uf,
    reach: &[Vec<Option<bool>>],
) -> Vec<Lit> {
    let (a, b) = (uf.find(strict.from), uf.find(strict.to));
    let mut why: Vec<Lit> = strict.why.clone();
    // edges that lie on some path from b back to a
    for e in edges {
        let (x, y) = (uf.find(e.from), uf.find(e.to));
        let on_path = (x == b || reach[b][x].is_some()) && (y == a || reach[y][a].is_some());
        if on_path {
            why.extend(e.why.iter().cloned());
        }
    }
    // equalities that merged classes are part of the story too
    why.extend(eq_lits(ctx));
    why.sort();
    why.dedup();
    if why.is_empty() {
        order_lits(ctx)
    } else {
        why
    }
}
