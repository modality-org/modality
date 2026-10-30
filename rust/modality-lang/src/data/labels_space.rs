//! The model's own graph as a space, for checking by labels alone.
//!
//! A configuration is a model node; a move is an edge, standing for every
//! commit its labels allow. A box sees an edge when some such commit could
//! carry the box's labels; a diamond is met when every one does. The theory
//! decides labels without terms. A label with a term, and an edge whose
//! guards read data, leave the answer to data: unknown, never a guess.

use super::formula::Space;
use super::label::{property_text, Kind, Label, OFFLINE_PREDICATES};
use super::machine::Machine;
use super::Tri;
use crate::ast::Property;
use crate::theory::{standard, NoState, Theory, Tri as TTri, TheoryVersion};

pub struct LabelSpace<'a> {
    pub machine: &'a Machine,
    pub version: TheoryVersion,
    nodes: Vec<String>,
    /// Edges out of each reachable node: `(edge index, target config)`.
    out: Vec<Vec<(usize, usize)>>,
    roots: Vec<usize>,
}

/// Whether the theory cannot read the label: a term that is not a literal
/// (`len(/in/s.text)`), or a predicate no network evaluates.
pub fn data_dependent(l: &Label) -> bool {
    let computed = l.terms().iter().any(|t| !matches!(t, super::Term::Lit(_) | super::Term::Path(_)));
    let term_args = l.property.has_terms()
        && !matches!(&l.kind, Kind::Set { value: super::Term::Lit(_), .. });
    computed || term_args || OFFLINE_PREDICATES.contains(&l.property.name.as_str())
}

impl<'a> LabelSpace<'a> {
    pub fn new(machine: &'a Machine, version: TheoryVersion) -> Self {
        Self::without(machine, version, &[])
    }

    /// The space without the edges in `dead` (by index): edges no run takes,
    /// from a known starting state.
    pub fn without(machine: &'a Machine, version: TheoryVersion, dead: &[usize]) -> Self {
        let nodes = machine.reachable();
        let idx = |n: &str| nodes.iter().position(|x| x == n);
        let out = nodes
            .iter()
            .map(|n| {
                machine
                    .edges_from(n)
                    .filter(|e| !dead.contains(&e.index))
                    .filter_map(|e| idx(&e.to).map(|t| (e.index, t)))
                    .collect()
            })
            .collect();
        let roots = machine.start.iter().filter_map(|s| idx(s)).collect();
        LabelSpace {
            machine,
            version,
            nodes,
            out,
            roots,
        }
    }

    pub fn nodes(&self) -> &[String] {
        &self.nodes
    }

    fn theory(&self) -> Theory<'static> {
        Theory::new(self.version, standard(), &NoState)
    }

    /// Whether moves on the edge depend on data: a guard, or a computed
    /// write that may fail.
    pub fn edge_reads_data(&self, edge: usize) -> bool {
        self.machine.edges[edge].labels.iter().any(|l| {
            matches!(l.kind, Kind::Pred { .. } | Kind::Never) || data_dependent(l)
        })
    }

    fn plain(labels: &[Label]) -> Vec<Property> {
        labels
            .iter()
            .filter(|l| !data_dependent(l))
            .map(|l| l.property.clone())
            .collect()
    }

    /// Whether the edge itself carries `l` (`Some(true)`), carries its
    /// opposite (`Some(false)`), or says nothing of it.
    fn carried(&self, edge: usize, l: &Label) -> Option<bool> {
        let text = property_text(&l.property);
        let neg = property_text(&l.property.negated());
        let e = &self.machine.edges[edge];
        if e.labels.iter().any(|x| property_text(&x.property) == text) {
            Some(true)
        } else if e.labels.iter().any(|x| property_text(&x.property) == neg) {
            Some(false)
        } else {
            None
        }
    }

    /// Some commit on the edge could carry every label in `ls`.
    pub fn consistent(&self, ls: &[Label], edge: usize) -> Tri {
        let e = &self.machine.edges[edge];
        let mut lits = Self::plain(&e.labels);
        lits.extend(Self::plain(ls));
        let v = self.theory().consistent(&lits);
        if v.tri == TTri::False {
            return Tri::False;
        }
        let mut out = match v.tri {
            TTri::True => Tri::True,
            _ => Tri::Unknown,
        };
        for l in ls.iter().filter(|l| data_dependent(l)) {
            match self.carried(edge, l) {
                Some(false) => return Tri::False,
                Some(true) => {}
                None => out = out.and(Tri::Unknown),
            }
        }
        if out == Tri::True && self.edge_reads_data(edge) {
            out = Tri::Unknown;
        }
        out
    }

    /// Every commit on the edge carries every label in `ls`.
    pub fn entails(&self, ls: &[Label], edge: usize) -> Tri {
        let e = &self.machine.edges[edge];
        let edge_lits = Self::plain(&e.labels);
        let mut out = Tri::True;
        for l in ls {
            if let Some(c) = self.carried(edge, l) {
                if !c {
                    return Tri::False;
                }
                continue;
            }
            if data_dependent(l) {
                out = out.and(Tri::Unknown);
                continue;
            }
            // A diamond is met only through a label the edge entails; one the
            // theory cannot show is not met, as on a network.
            let mut with_neg = edge_lits.clone();
            with_neg.push(l.property.negated());
            if self.theory().consistent(&with_neg).tri != TTri::False {
                return Tri::False;
            }
        }
        if out == Tri::True && self.edge_reads_data(edge) {
            out = Tri::Unknown;
        }
        out
    }

    /// The edges, breadth first, from a start node to `edge`'s source.
    pub fn path_to(&self, edge: usize) -> Vec<usize> {
        let target = &self.machine.edges[edge].from;
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; self.nodes.len()];
        let mut seen = vec![false; self.nodes.len()];
        let mut queue: std::collections::VecDeque<usize> = self.roots.iter().copied().collect();
        for &r in &self.roots {
            seen[r] = true;
        }
        while let Some(c) = queue.pop_front() {
            if &self.nodes[c] == target {
                let mut path = vec![edge];
                let mut at = c;
                while let Some((e, p)) = prev[at] {
                    path.push(e);
                    at = p;
                }
                path.reverse();
                return path;
            }
            for &(e, t) in &self.out[c] {
                if !seen[t] {
                    seen[t] = true;
                    prev[t] = Some((e, c));
                    queue.push_back(t);
                }
            }
        }
        vec![edge]
    }
}

impl Space for LabelSpace<'_> {
    fn size(&self) -> usize {
        self.nodes.len()
    }
    fn roots(&self) -> Vec<usize> {
        self.roots.clone()
    }
    fn node(&self, c: usize) -> &str {
        &self.nodes[c]
    }
    fn frontier(&self, _c: usize) -> bool {
        false
    }
    fn moves(&self, c: usize) -> Vec<(usize, usize)> {
        self.out[c].clone()
    }
    fn move_from(&self, mv: usize) -> &str {
        &self.machine.edges[mv].from
    }
    fn box_match(&self, ls: &[Label], mv: usize) -> Tri {
        self.consistent(ls, mv)
    }
    fn diamond_match(&self, ls: &[Label], mv: usize) -> Tri {
        self.entails(ls, mv)
    }
}
