//! Bounded search: every run from every combination of the inputs in a
//! domain, up to a depth, as one graph of (node, state) configurations.
//!
//! Inputs are posted by the first move that may write them: a domain path
//! not yet in the state, that the edge neither promises to leave alone nor
//! writes itself, takes each of its domain values. A `havoc` path may be
//! written with any of its values, or left alone, by any move that does not
//! promise to leave it alone.

use super::machine::{Machine, Outcome};
use super::value::Value;
use super::{State, Writes};
use std::collections::{BTreeMap, HashMap, VecDeque};

#[derive(Debug, Clone, Default)]
pub struct Domain {
    pub depth: usize,
    /// In the order the domain file lists them.
    pub values: Vec<(String, Vec<Value>)>,
    pub havoc: Vec<(String, Vec<Value>)>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub node: String,
    pub state: State,
    pub depth: usize,
    /// The move that first reached this configuration.
    pub via: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MoveRec {
    pub from: usize,
    pub edge: usize,
    pub writes: Writes,
    pub to: usize,
    /// Paths this move wrote of its own accord (inputs or havoc).
    pub chosen: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BadWriteRec {
    pub from: usize,
    pub edge: usize,
    pub path: String,
    pub value: Value,
}

#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub configs: Vec<Config>,
    pub moves: Vec<MoveRec>,
    pub out: Vec<Vec<usize>>,
    /// Configurations at the depth bound that have moves not explored.
    pub frontier: Vec<bool>,
    pub bad: Vec<BadWriteRec>,
    pub roots: Vec<usize>,
    /// Domain paths that some run posted as an input, in domain order,
    /// with how many values each took.
    pub inputs: Vec<(String, usize)>,
    pub depth: usize,
}

/// Every way a move on `edge` from `state` may write paths of its own
/// accord: posted inputs, then havoc.
fn choices(m: &Machine, edge: usize, state: &State, d: &Domain) -> Vec<Writes> {
    let e = &m.edges[edge];
    let mut options: Vec<(String, Vec<Option<Value>>)> = Vec::new();
    for (path, values) in &d.values {
        if !state.contains_key(path) && e.leaves_open(path) {
            options.push((path.clone(), values.iter().cloned().map(Some).collect()));
        }
    }
    for (path, values) in &d.havoc {
        if e.leaves_open(path) && !options.iter().any(|(p, _)| p == path) {
            let mut vs = vec![None];
            vs.extend(values.iter().cloned().map(Some));
            options.push((path.clone(), vs));
        }
    }
    let mut out = vec![Writes::new()];
    for (path, values) in options {
        let mut next = Vec::with_capacity(out.len() * values.len());
        for w in &out {
            for v in &values {
                let mut w = w.clone();
                if let Some(v) = v {
                    w.insert(path.clone(), v.clone());
                }
                next.push(w);
            }
        }
        out = next;
    }
    out
}

/// Explore every run of `m` over `d`, breadth first.
pub fn explore(m: &Machine, d: &Domain) -> Graph {
    let mut g = Graph {
        depth: d.depth,
        ..Graph::default()
    };
    let mut index: HashMap<(String, State), usize> = HashMap::new();
    let mut queue = VecDeque::new();
    for s in &m.start {
        let key = (s.clone(), State::new());
        if index.contains_key(&key) {
            continue;
        }
        let i = g.configs.len();
        g.configs.push(Config {
            node: s.clone(),
            state: State::new(),
            depth: 0,
            via: None,
        });
        g.out.push(Vec::new());
        g.frontier.push(false);
        g.roots.push(i);
        index.insert(key, i);
        queue.push_back(i);
    }
    let mut posted: BTreeMap<String, ()> = BTreeMap::new();
    while let Some(c) = queue.pop_front() {
        let (node, state, depth) = {
            let cfg = &g.configs[c];
            (cfg.node.clone(), cfg.state.clone(), cfg.depth)
        };
        for e in m.edges_from(&node) {
            for extra in choices(m, e.index, &state, d) {
                match m.try_move(e, &state, &extra) {
                    Outcome::Moved { writes, after } => {
                        if depth >= d.depth {
                            g.frontier[c] = true;
                            continue;
                        }
                        for p in extra.keys() {
                            if d.values.iter().any(|(dp, _)| dp == p) && !state.contains_key(p) {
                                posted.insert(p.clone(), ());
                            }
                        }
                        let key = (e.to.clone(), after.clone());
                        let mv = g.moves.len();
                        let to = match index.get(&key) {
                            Some(&t) => t,
                            None => {
                                let t = g.configs.len();
                                g.configs.push(Config {
                                    node: e.to.clone(),
                                    state: after,
                                    depth: depth + 1,
                                    via: Some(mv),
                                });
                                g.out.push(Vec::new());
                                g.frontier.push(false);
                                index.insert(key, t);
                                queue.push_back(t);
                                t
                            }
                        };
                        g.moves.push(MoveRec {
                            from: c,
                            edge: e.index,
                            writes,
                            to,
                            chosen: extra.keys().cloned().collect(),
                        });
                        g.out[c].push(mv);
                    }
                    Outcome::BadWrite { path, value } => g.bad.push(BadWriteRec {
                        from: c,
                        edge: e.index,
                        path,
                        value,
                    }),
                    Outcome::Blocked => {}
                }
            }
        }
    }
    g.inputs = d
        .values
        .iter()
        .filter(|(p, _)| posted.contains_key(p))
        .map(|(p, vs)| (p.clone(), vs.len()))
        .collect();
    g
}

impl Graph {
    /// The moves from a root to configuration `c`, in order.
    pub fn path_to(&self, c: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut at = c;
        while let Some(mv) = self.configs[at].via {
            out.push(mv);
            at = self.moves[mv].from;
        }
        out.reverse();
        out
    }

    /// How many runs: paths from a root that end where no move is possible
    /// or at the depth bound.
    pub fn runs(&self) -> u128 {
        let mut memo: HashMap<(usize, usize), u128> = HashMap::new();
        fn count(
            g: &Graph,
            c: usize,
            d: usize,
            memo: &mut HashMap<(usize, usize), u128>,
        ) -> u128 {
            if let Some(&n) = memo.get(&(c, d)) {
                return n;
            }
            let n = if d >= g.depth || g.out[c].is_empty() {
                1
            } else {
                g.out[c]
                    .iter()
                    .map(|&mv| count(g, g.moves[mv].to, d + 1, memo))
                    .fold(0u128, u128::saturating_add)
            };
            memo.insert((c, d), n);
            n
        }
        self.roots
            .iter()
            .map(|&r| count(self, r, 0, &mut memo))
            .fold(0u128, u128::saturating_add)
    }
}
