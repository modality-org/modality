//! State flow (`V2`): what is known about accepted state on arrival at each
//! node of a model, and the edges that rules out.
//!
//! A commit changes accepted state only at the paths its actions name. An
//! edge with `-modifies(q)` therefore leaves every path under `q` as it
//! was, and a literal that reads only such paths holds after the commit
//! if it held before. The facts at a node are the literals that hold
//! however the contract arrived there: at an initial node what the seed
//! says (nothing, or what accepted state says there), and otherwise what
//! every edge into it carries. An edge whose labels
//! contradict the facts at its node is never taken once the contract is
//! under way. That is a lint, never a refusal: contracts may end.
//!
//! Lean: `Flow.lean`. `flow_sound` proves that facts closed under
//! [`post`] hold on every run; `dead_after_sound` that no commit on a run
//! takes an edge [`flow`] reports.

use super::dead;
use super::sort::{under, Constraint, Lit, Term};
use std::collections::BTreeMap;

/// The paths a literal reads, when it reads accepted state and nothing
/// else. Lean: `Atom.reads`.
pub fn state_reads(l: &Lit) -> Option<Vec<&str>> {
    fn term(t: &Term) -> Option<&str> {
        match t {
            Term::Path(p) => Some(p),
            Term::Const(_) => None,
        }
    }
    match &l.c {
        Constraint::Order { lhs, rhs, .. } => {
            Some(term(lhs).into_iter().chain(term(rhs)).collect())
        }
        Constraint::Eq { path, .. }
        | Constraint::Text { path, .. }
        | Constraint::Is { path, .. }
        | Constraint::Exists { path } => Some(vec![path.as_str()]),
        Constraint::Eq2 { a, b } => Some(vec![a.as_str(), b.as_str()]),
        _ => None,
    }
}

/// The edge forbids every write at `path`: it has `-modifies` of `path`
/// or of a path above it. Lean: `frames`.
pub fn frames(edge: &[Lit], path: &str) -> bool {
    edge.iter()
        .any(|l| !l.positive && matches!(&l.c, Constraint::Writes { path: q } if under(path, q)))
}

/// A commit that takes the edge leaves the literal as it was. Lean:
/// `carries`.
pub fn carries(edge: &[Lit], l: &Lit) -> bool {
    state_reads(l).is_some_and(|paths| paths.iter().all(|p| frames(edge, p)))
}

/// What is still known after a commit takes the edge from a node where
/// `facts` hold. Sorted. Lean: `post`.
pub fn post(facts: &[Lit], edge: &[Lit]) -> Vec<Lit> {
    let mut out: Vec<Lit> = facts
        .iter()
        .chain(edge)
        .filter(|l| carries(edge, l))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// One edge. `lits` is `None` when the labels cannot be read as literals
/// (they hold variables): such an edge carries nothing and is never
/// reported.
#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub lits: Option<Vec<Lit>>,
}

/// The result of [`flow`].
#[derive(Debug, Clone, Default)]
pub struct Flow {
    /// The facts at every node a run can reach. A node missing here is
    /// reached by no run.
    pub facts: BTreeMap<String, Vec<Lit>>,
    /// Edges (by index) whose labels contradict the facts at their node,
    /// with the literals that do. Edges dead on their own are not listed.
    pub dead_after: Vec<(usize, Vec<Lit>)>,
}

/// Facts on arrival at every node from `initial`, knowing nothing there.
pub fn flow(edges: &[FlowEdge], initial: &[String]) -> Flow {
    flow_seeded(edges, initial, &[])
}

/// Facts on arrival at every node from `initial`, where every run starts
/// with `seed` true, and the edges they rule out. An edge ruled out, or
/// dead on its own, is never taken, so it carries nothing into its target;
/// the facts are recomputed until no more edges fall. Lean: `Closed` with
/// `seed`; from accepted state, `stateFacts_seed`.
pub fn flow_seeded(edges: &[FlowEdge], initial: &[String], seed: &[Lit]) -> Flow {
    let mut seed = seed.to_vec();
    seed.sort();
    seed.dedup();
    let mut taken: Vec<bool> = edges
        .iter()
        .map(|e| e.lits.as_ref().is_none_or(|l| dead(l).is_none()))
        .collect();
    let live = taken.clone();
    loop {
        let facts = fixpoint(edges, initial, &seed, &taken);
        let falls = falling(edges, &facts, &taken);
        if falls.is_empty() {
            // Report against the final facts: an edge out of a node that a
            // later fall cut off is not reported, the edge in is.
            let dead_after = falling(edges, &facts, &live);
            return Flow { facts, dead_after };
        }
        for (i, _) in falls {
            taken[i] = false;
        }
    }
}

/// Edges among `among` whose labels contradict the facts at their node.
/// Facts that cannot hold together mean no run gets there, and are
/// skipped.
fn falling(
    edges: &[FlowEdge],
    facts: &BTreeMap<String, Vec<Lit>>,
    among: &[bool],
) -> Vec<(usize, Vec<Lit>)> {
    let mut out = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        let (Some(lits), Some(at)) = (&e.lits, facts.get(&e.from)) else {
            continue;
        };
        if !among[i] || at.is_empty() || dead(at).is_some() {
            continue;
        }
        let mut all = at.clone();
        all.extend(lits.iter().cloned());
        all.sort();
        all.dedup();
        if let Some(why) = dead(&all) {
            out.push((i, why));
        }
    }
    out
}

/// The greatest facts closed under [`post`] over the edges still taken:
/// each node's set starts at the seed (initial nodes) or at what its first
/// edge in carries, and only shrinks, so this ends.
fn fixpoint(
    edges: &[FlowEdge],
    initial: &[String],
    seed: &[Lit],
    taken: &[bool],
) -> BTreeMap<String, Vec<Lit>> {
    let mut facts: BTreeMap<String, Vec<Lit>> =
        initial.iter().map(|n| (n.clone(), seed.to_vec())).collect();
    loop {
        let mut changed = false;
        for (e, _) in edges.iter().zip(taken).filter(|(_, t)| **t) {
            let Some(at) = facts.get(&e.from) else {
                continue;
            };
            let carried = match &e.lits {
                Some(lits) => post(at, lits),
                None => Vec::new(),
            };
            match facts.get_mut(&e.to) {
                None => {
                    facts.insert(e.to.clone(), carried);
                    changed = true;
                }
                Some(known) => {
                    let before = known.len();
                    known.retain(|l| carried.binary_search(l).is_ok());
                    changed |= known.len() != before;
                }
            }
        }
        if !changed {
            return facts;
        }
    }
}
