//! Text equality classes over paths and string literals.
//!
//! Positive literals give edges "hold the same string": `(= p "s")` joins
//! `p` and the literal `"s"`, `(= a b)` joins two paths, and a substring
//! test or `(signed p)` says `p` holds a string. Classes are the
//! transitive closure, so two paths equal to the same literal are in one
//! class (case K1). A path is *texted* when some positive literal forces
//! it to hold a string. Lean twin: `textOf`, `same`, `textPaths` in
//! `Decide.lean`.

use super::sort::{Constraint, Lit};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Path(String),
    Lit(String),
}

pub struct TextClasses {
    index: BTreeMap<Node, usize>,
    parent: Vec<usize>,
}

impl TextClasses {
    pub fn new(lits: &[Lit]) -> Self {
        let mut edges: Vec<(Node, Node)> = Vec::new();
        for lit in lits.iter().filter(|l| l.positive) {
            match &lit.c {
                Constraint::Eq { path, lit } => {
                    edges.push((Node::Path(path.clone()), Node::Lit(lit.clone())))
                }
                Constraint::Eq2 { a, b } => {
                    edges.push((Node::Path(a.clone()), Node::Path(b.clone())))
                }
                Constraint::Text { path, .. } | Constraint::Signer { id: path } => {
                    edges.push((Node::Path(path.clone()), Node::Path(path.clone())))
                }
                _ => {}
            }
        }
        let nodes: BTreeSet<Node> = edges
            .iter()
            .flat_map(|(a, b)| [a.clone(), b.clone()])
            .collect();
        let index: BTreeMap<Node, usize> =
            nodes.into_iter().enumerate().map(|(i, n)| (n, i)).collect();
        let mut classes = Self {
            parent: (0..index.len()).collect(),
            index,
        };
        for (a, b) in &edges {
            let (ia, ib) = (classes.index[a], classes.index[b]);
            classes.union(ia, ib);
        }
        classes
    }

    fn find(&self, mut x: usize) -> usize {
        while self.parent[x] != x {
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[hi] = lo;
        }
    }

    fn class(&self, n: &Node) -> Option<usize> {
        self.index.get(n).map(|&i| self.find(i))
    }

    /// Some positive literal forces `p` to hold a string.
    pub fn texted(&self, p: &str) -> bool {
        self.index.contains_key(&Node::Path(p.to_string()))
    }

    /// `a` and `b` are forced to hold the same string (for `a == b`: `a`
    /// holds one).
    pub fn same_paths(&self, a: &str, b: &str) -> bool {
        match (
            self.class(&Node::Path(a.to_string())),
            self.class(&Node::Path(b.to_string())),
        ) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    /// `p` is forced to hold exactly `s`.
    pub fn same_lit(&self, p: &str, s: &str) -> bool {
        match (
            self.class(&Node::Path(p.to_string())),
            self.class(&Node::Lit(s.to_string())),
        ) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    /// The literals in `p`'s class (more than one is a contradiction).
    pub fn class_lits(&self, p: &str) -> Vec<&str> {
        let Some(c) = self.class(&Node::Path(p.to_string())) else {
            return Vec::new();
        };
        self.index
            .iter()
            .filter(|(n, &i)| matches!(n, Node::Lit(_)) && self.find(i) == c)
            .map(|(n, _)| match n {
                Node::Lit(s) => s.as_str(),
                Node::Path(_) => unreachable!(),
            })
            .collect()
    }

    /// Paths forced to hold a string, sorted.
    pub fn texted_paths(&self) -> impl Iterator<Item = &str> {
        self.index.keys().filter_map(|n| match n {
            Node::Path(p) => Some(p.as_str()),
            Node::Lit(_) => None,
        })
    }

    /// Two different literals in one class.
    pub fn two_lits(&self) -> Option<(&str, &str)> {
        let mut first: BTreeMap<usize, &str> = BTreeMap::new();
        for (n, &i) in &self.index {
            if let Node::Lit(s) = n {
                let c = self.find(i);
                match first.get(&c) {
                    Some(t) if *t != s.as_str() => return Some((t, s.as_str())),
                    Some(_) => {}
                    None => {
                        first.insert(c, s.as_str());
                    }
                }
            }
        }
        None
    }

    /// Index of `p`'s class among texted paths' classes, for fresh names:
    /// the position of the first texted path in the same class.
    pub fn rep(&self, p: &str) -> usize {
        self.texted_paths()
            .position(|q| self.same_paths(q, p))
            .unwrap_or(0)
    }
}
