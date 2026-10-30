//! A formula on the graph a bounded search explored, in three values.
//!
//! Each subformula gets two sets of configurations: where it surely holds
//! (`must`) and where it may (`may`). A configuration at the depth bound
//! with moves not explored is undecided for every box and diamond, and so
//! is a label the checker cannot read on data (`signed_by`, static labels)
//! unless the edge itself says it. A formula holds when it surely holds at
//! every root, is violated when it cannot hold at some root, and is
//! otherwise undecided.

use super::label::Label;
use super::machine::Machine;
use super::search::Graph;
use super::Tri;
use crate::ast::{FormulaExpr, Property};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
struct Sets {
    must: Vec<bool>,
    may: Vec<bool>,
}

/// Where a formula is evaluated: configurations, and the moves between
/// them, each matched against a box's or diamond's labels.
pub trait Space {
    fn size(&self) -> usize;
    fn roots(&self) -> Vec<usize>;
    /// The model node of a configuration.
    fn node(&self, c: usize) -> &str;
    /// A configuration whose moves were not all explored.
    fn frontier(&self, c: usize) -> bool;
    /// `(move, target)` for each move out of `c`.
    fn moves(&self, c: usize) -> Vec<(usize, usize)>;
    /// The node a move leaves from.
    fn move_from(&self, mv: usize) -> &str;
    /// Whether a box over `ls` sees the move: true when some commit the
    /// move stands for carries every label.
    fn box_match(&self, ls: &[Label], mv: usize) -> Tri;
    /// Whether a diamond over `ls` is met by the move: true when every
    /// commit it stands for carries every label.
    fn diamond_match(&self, ls: &[Label], mv: usize) -> Tri;
}

pub struct Evaluator<'a> {
    pub space: &'a dyn Space,
    /// When set, boxes and diamonds see only moves on edges from these
    /// nodes (an invariant's `on q -> *`).
    pub anchor: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphVerdict {
    Holds,
    Violated,
    Undecided,
}

/// A bounded search's graph as a space: one move is one commit.
pub struct DataSpace<'a> {
    pub machine: &'a Machine,
    pub graph: &'a Graph,
}

impl DataSpace<'_> {
    pub fn matches(&self, ls: &[Label], mv: usize) -> Tri {
        let m = &self.graph.moves[mv];
        let pre = &self.graph.configs[m.from].state;
        match_move(ls, pre, &m.writes, &self.machine.edges[m.edge])
    }
}

/// Whether every label in `ls` is true of one move on `edge` from `pre`.
pub fn match_move(
    ls: &[Label],
    pre: &super::State,
    writes: &super::Writes,
    edge: &super::machine::Edge,
) -> Tri {
    let mut out = Tri::True;
    for l in ls {
        let t = match l.holds(pre, writes) {
            Tri::Unknown => opaque(&l.property, &edge.transition.properties),
            t => t,
        };
        out = out.and(t);
    }
    out
}

impl Space for DataSpace<'_> {
    fn size(&self) -> usize {
        self.graph.configs.len()
    }
    fn roots(&self) -> Vec<usize> {
        self.graph.roots.clone()
    }
    fn node(&self, c: usize) -> &str {
        &self.graph.configs[c].node
    }
    fn frontier(&self, c: usize) -> bool {
        self.graph.frontier[c]
    }
    fn moves(&self, c: usize) -> Vec<(usize, usize)> {
        self.graph.out[c]
            .iter()
            .map(|&mv| (mv, self.graph.moves[mv].to))
            .collect()
    }
    fn move_from(&self, mv: usize) -> &str {
        &self.machine.edges[self.graph.moves[mv].edge].from
    }
    fn box_match(&self, ls: &[Label], mv: usize) -> Tri {
        self.matches(ls, mv)
    }
    fn diamond_match(&self, ls: &[Label], mv: usize) -> Tri {
        self.matches(ls, mv)
    }
}

impl<'a> Evaluator<'a> {
    pub fn new(space: &'a dyn Space) -> Self {
        Evaluator {
            space,
            anchor: None,
        }
    }

    pub fn verdict(&self, f: &FormulaExpr) -> Result<GraphVerdict, String> {
        let labels = LabelCache::new(f)?;
        let s = self.eval(f, &HashMap::new(), &labels);
        let roots = self.space.roots();
        Ok(if roots.iter().all(|&r| s.must[r]) {
            GraphVerdict::Holds
        } else if roots.iter().any(|&r| !s.may[r]) {
            GraphVerdict::Violated
        } else {
            GraphVerdict::Undecided
        })
    }

    fn sees(&self, mv: usize) -> bool {
        match &self.anchor {
            None => true,
            Some(nodes) => nodes.iter().any(|n| n == self.space.move_from(mv)),
        }
    }

    fn anchor_includes(&self, c: usize) -> bool {
        match &self.anchor {
            None => true,
            Some(nodes) => nodes.iter().any(|n| n == self.space.node(c)),
        }
    }

    fn all(&self, v: bool) -> Vec<bool> {
        vec![v; self.space.size()]
    }

    fn eval(&self, f: &FormulaExpr, env: &HashMap<String, Sets>, lc: &LabelCache) -> Sets {
        let n = self.space.size();
        match f {
            FormulaExpr::True => Sets {
                must: self.all(true),
                may: self.all(true),
            },
            FormulaExpr::False => Sets {
                must: self.all(false),
                may: self.all(false),
            },
            FormulaExpr::Prop(name) | FormulaExpr::Var(name) => match env.get(name) {
                Some(s) => s.clone(),
                None => {
                    let at: Vec<bool> = (0..n).map(|c| self.space.node(c) == name).collect();
                    Sets {
                        must: at.clone(),
                        may: at,
                    }
                }
            },
            FormulaExpr::Paren(e) => self.eval(e, env, lc),
            FormulaExpr::Not(e) => {
                let s = self.eval(e, env, lc);
                Sets {
                    must: s.may.iter().map(|b| !b).collect(),
                    may: s.must.iter().map(|b| !b).collect(),
                }
            }
            FormulaExpr::And(a, b) => {
                let (a, b) = (self.eval(a, env, lc), self.eval(b, env, lc));
                Sets {
                    must: (0..n).map(|i| a.must[i] && b.must[i]).collect(),
                    may: (0..n).map(|i| a.may[i] && b.may[i]).collect(),
                }
            }
            FormulaExpr::Or(a, b) => {
                let (a, b) = (self.eval(a, env, lc), self.eval(b, env, lc));
                Sets {
                    must: (0..n).map(|i| a.must[i] || b.must[i]).collect(),
                    may: (0..n).map(|i| a.may[i] || b.may[i]).collect(),
                }
            }
            FormulaExpr::Implies(a, b) => self.eval(
                &FormulaExpr::Or(Box::new(FormulaExpr::Not(a.clone())), b.clone()),
                env,
                lc,
            ),
            FormulaExpr::Box(ls, e) => {
                let s = self.eval(e, env, lc);
                let ls = lc.get(ls);
                let mut must = self.all(true);
                let mut may = self.all(true);
                for c in 0..n {
                    if self.space.frontier(c) && self.anchor_includes(c) {
                        must[c] = false;
                    }
                    for (mv, to) in self.space.moves(c) {
                        if !self.sees(mv) {
                            continue;
                        }
                        match self.space.box_match(ls, mv) {
                            Tri::True => {
                                must[c] &= s.must[to];
                                may[c] &= s.may[to];
                            }
                            Tri::Unknown => must[c] &= s.must[to],
                            Tri::False => {}
                        }
                    }
                }
                Sets { must, may }
            }
            FormulaExpr::Diamond(ls, e) => {
                let s = self.eval(e, env, lc);
                let ls = lc.get(ls);
                let mut must = self.all(false);
                let mut may = self.all(false);
                for c in 0..n {
                    if self.space.frontier(c) && self.anchor_includes(c) {
                        may[c] = true;
                    }
                    for (mv, to) in self.space.moves(c) {
                        if !self.sees(mv) {
                            continue;
                        }
                        match self.space.diamond_match(ls, mv) {
                            Tri::True => {
                                must[c] |= s.must[to];
                                may[c] |= s.may[to];
                            }
                            Tri::Unknown => may[c] |= s.may[to],
                            Tri::False => {}
                        }
                    }
                }
                Sets { must, may }
            }
            FormulaExpr::DiamondBox(..) => self.eval(&f.expand_diamond_box(), env, lc),
            FormulaExpr::Next(e) => self.eval(&FormulaExpr::Diamond(Vec::new(), e.clone()), env, lc),
            FormulaExpr::Always(e) => {
                let x = fresh(env);
                let body = FormulaExpr::And(
                    e.clone(),
                    Box::new(FormulaExpr::Box(Vec::new(), Box::new(FormulaExpr::Var(x.clone())))),
                );
                self.fix(&x, &body, true, env, lc)
            }
            FormulaExpr::Eventually(e) => {
                let x = fresh(env);
                let body = FormulaExpr::Or(
                    e.clone(),
                    Box::new(FormulaExpr::Diamond(
                        Vec::new(),
                        Box::new(FormulaExpr::Var(x.clone())),
                    )),
                );
                self.fix(&x, &body, false, env, lc)
            }
            FormulaExpr::Until(p, q) => {
                let x = fresh(env);
                let body = FormulaExpr::Or(
                    q.clone(),
                    Box::new(FormulaExpr::And(
                        p.clone(),
                        Box::new(FormulaExpr::Diamond(
                            Vec::new(),
                            Box::new(FormulaExpr::Var(x.clone())),
                        )),
                    )),
                );
                self.fix(&x, &body, false, env, lc)
            }
            FormulaExpr::Lfp(x, body) => self.fix(x, body, false, env, lc),
            FormulaExpr::Gfp(x, body) => self.fix(x, body, true, env, lc),
        }
    }

    fn fix(
        &self,
        x: &str,
        body: &FormulaExpr,
        greatest: bool,
        env: &HashMap<String, Sets>,
        lc: &LabelCache,
    ) -> Sets {
        let mut cur = Sets {
            must: self.all(greatest),
            may: self.all(greatest),
        };
        let limit = 2 * self.space.size() + 4;
        for _ in 0..limit {
            let mut env2 = env.clone();
            env2.insert(x.to_string(), cur.clone());
            let next = self.eval(body, &env2, lc);
            if next == cur {
                break;
            }
            cur = next;
        }
        cur
    }
}

fn fresh(env: &HashMap<String, Sets>) -> String {
    let mut i = env.len();
    loop {
        let x = format!("__x{i}");
        if !env.contains_key(&x) {
            return x;
        }
        i += 1;
    }
}

/// A label not read on data matches a move only by what the edge says.
fn opaque(label: &Property, edge: &[Property]) -> Tri {
    let same = |p: &Property| p.name == label.name && p.source == label.source;
    if edge.iter().any(|p| same(p) && p.sign == label.sign) {
        Tri::True
    } else if edge.iter().any(|p| same(p) && p.sign != label.sign) {
        Tri::False
    } else {
        Tri::Unknown
    }
}

/// Labels of every box and diamond, read once.
struct LabelCache {
    by_key: HashMap<String, Vec<Label>>,
}

impl LabelCache {
    fn new(f: &FormulaExpr) -> Result<Self, String> {
        let mut by_key = HashMap::new();
        fn walk(f: &FormulaExpr, out: &mut HashMap<String, Vec<Label>>) -> Result<(), String> {
            match f {
                FormulaExpr::Box(ls, e) | FormulaExpr::Diamond(ls, e) | FormulaExpr::DiamondBox(ls, e) => {
                    if let std::collections::hash_map::Entry::Vacant(slot) = out.entry(key(ls)) {
                        slot.insert(
                            ls.iter()
                                .map(Label::from_property)
                                .collect::<Result<Vec<_>, _>>()?,
                        );
                    }
                    if let FormulaExpr::DiamondBox(..) = f {
                        walk(&f.expand_diamond_box(), out)?;
                    }
                    walk(e, out)
                }
                FormulaExpr::And(a, b)
                | FormulaExpr::Or(a, b)
                | FormulaExpr::Implies(a, b)
                | FormulaExpr::Until(a, b) => {
                    walk(a, out)?;
                    walk(b, out)
                }
                FormulaExpr::Not(e)
                | FormulaExpr::Paren(e)
                | FormulaExpr::Eventually(e)
                | FormulaExpr::Always(e)
                | FormulaExpr::Next(e)
                | FormulaExpr::Lfp(_, e)
                | FormulaExpr::Gfp(_, e) => walk(e, out),
                FormulaExpr::True | FormulaExpr::False | FormulaExpr::Prop(_) | FormulaExpr::Var(_) => {
                    Ok(())
                }
            }
        }
        walk(f, &mut by_key)?;
        by_key.insert(String::new(), Vec::new());
        Ok(LabelCache { by_key })
    }

    fn get(&self, ls: &[Property]) -> &[Label] {
        self.by_key.get(&key(ls)).map(Vec::as_slice).unwrap_or(&[])
    }
}

fn key(ls: &[Property]) -> String {
    serde_json::to_string(ls).unwrap_or_default()
}

/// `always([L] false)`, possibly in parentheses: the labels `L`.
pub fn safety_labels(f: &FormulaExpr) -> Option<&[Property]> {
    if let FormulaExpr::Always(e) = strip(f) {
        if let FormulaExpr::Box(ls, b) = strip(e) {
            if is_false(b) {
                return Some(ls.as_slice());
            }
        }
    }
    None
}

pub fn strip(f: &FormulaExpr) -> &FormulaExpr {
    match f {
        FormulaExpr::Paren(e) => strip(e),
        _ => f,
    }
}

fn is_false(f: &FormulaExpr) -> bool {
    match f {
        FormulaExpr::False => true,
        FormulaExpr::Paren(e) => is_false(e),
        _ => false,
    }
}

/// The first move, breadth first, that a safety formula's box surely
/// matches, among moves on edges from `anchor` when given: a counterexample.
pub fn first_match(
    space: &DataSpace,
    ls: &[Property],
    anchor: Option<&[String]>,
) -> Result<Option<usize>, String> {
    let labels = ls
        .iter()
        .map(Label::from_property)
        .collect::<Result<Vec<_>, _>>()?;
    let mut best: Option<(usize, usize)> = None;
    for (i, m) in space.graph.moves.iter().enumerate() {
        if let Some(a) = anchor {
            if !a.contains(&space.machine.edges[m.edge].from) {
                continue;
            }
        }
        if space.matches(&labels, i) != Tri::True {
            continue;
        }
        let d = space.graph.configs[m.from].depth;
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    Ok(best.map(|(_, i)| i))
}
