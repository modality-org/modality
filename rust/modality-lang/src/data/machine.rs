//! A model read as a machine over data: each edge is a guard on the state a
//! move starts from, the writes it must make, and the paths it promises to
//! leave alone. A move may write anything else; who writes it (the inputs a
//! run posts, or havoc in a search) is the caller's choice.

use super::label::{under, Kind, Label};
use super::term::Term;
use super::value::{PathType, Value};
use super::{State, Tri, Writes};
use crate::ast::{Model, Transition};

#[derive(Debug, Clone)]
pub struct Edge {
    pub index: usize,
    pub from: String,
    pub to: String,
    pub labels: Vec<Label>,
    pub transition: Transition,
}

impl Edge {
    pub fn name(&self) -> String {
        format!("{} -> {}", self.from, self.to)
    }

    /// Paths a move on this edge promises not to write.
    pub fn frames(&self) -> impl Iterator<Item = &str> {
        self.labels.iter().filter_map(Label::frame)
    }

    pub fn frames_path(&self, path: &str) -> bool {
        self.frames().any(|f| under(path, f))
    }

    /// Writes the edge requires, by path, as terms.
    pub fn required(&self) -> Vec<(String, Term)> {
        self.labels.iter().filter_map(Label::required_write).collect()
    }

    pub fn sets_path(&self, path: &str) -> bool {
        self.required().iter().any(|(p, _)| p == path)
    }

    /// Whether a move on this edge may write `path` of its own accord: not
    /// promised away, and not already a required write.
    pub fn leaves_open(&self, path: &str) -> bool {
        !self.frames_path(path) && !self.sets_path(path)
    }

    /// The paths a move wrote, in the order the edge's labels list its
    /// required writes, then the rest.
    pub fn write_order(&self, writes: &Writes) -> Vec<String> {
        let mut out: Vec<String> = self
            .required()
            .into_iter()
            .map(|(p, _)| p)
            .filter(|p| writes.contains_key(p))
            .collect();
        for k in writes.keys() {
            if !out.contains(k) {
                out.push(k.clone());
            }
        }
        out
    }

    pub fn guards(&self) -> impl Iterator<Item = &Label> {
        self.labels
            .iter()
            .filter(|l| matches!(l.kind, Kind::Pred { .. } | Kind::Never))
    }
}

/// What happens when a move is tried.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The move is made: what it wrote, and the state after.
    Moved { writes: Writes, after: State },
    /// A label of the edge is false for this move.
    Blocked,
    /// Everything holds, but a write does not fit its path's type, so no
    /// such move can be made.
    BadWrite { path: String, value: Value },
}

#[derive(Debug, Clone)]
pub struct Machine {
    pub name: String,
    pub start: Vec<String>,
    pub edges: Vec<Edge>,
}

impl Machine {
    pub fn new(model: &Model) -> Result<Machine, String> {
        let transitions: Vec<Transition> = model
            .transitions
            .iter()
            .chain(model.parts.iter().flat_map(|p| p.transitions.iter()))
            .cloned()
            .collect();
        let mut edges = Vec::new();
        for (index, t) in transitions.into_iter().enumerate() {
            let labels = t
                .properties
                .iter()
                .map(Label::from_property)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("{} -> {}: {e}", t.from, t.to))?;
            edges.push(Edge {
                index,
                from: t.from.clone(),
                to: t.to.clone(),
                labels,
                transition: t,
            });
        }
        Ok(Machine {
            name: model.name.clone(),
            start: crate::model_checker::start_nodes(model),
            edges,
        })
    }

    pub fn edges_from<'a>(&'a self, node: &'a str) -> impl Iterator<Item = &'a Edge> + 'a {
        self.edges.iter().filter(move |e| e.from == node)
    }

    pub fn nodes(&self) -> Vec<String> {
        let mut out: Vec<String> = self.start.clone();
        for e in &self.edges {
            for n in [&e.from, &e.to] {
                if !out.contains(n) {
                    out.push(n.clone());
                }
            }
        }
        out
    }

    /// Nodes some path of edges reaches from a start node.
    pub fn reachable(&self) -> Vec<String> {
        let mut seen: Vec<String> = self.start.clone();
        let mut i = 0;
        while i < seen.len() {
            let n = seen[i].clone();
            for e in self.edges_from(&n) {
                if !seen.contains(&e.to) {
                    seen.push(e.to.clone());
                }
            }
            i += 1;
        }
        seen
    }

    /// Try a move on `edge` from `pre`, with `extra` written of the
    /// caller's accord (inputs, havoc). The edge's required writes win.
    pub fn try_move(&self, edge: &Edge, pre: &State, extra: &Writes) -> Outcome {
        let mut writes = extra.clone();
        for (path, term) in edge.required() {
            match term.eval(pre) {
                Some(v) => {
                    writes.insert(path, v);
                }
                None => return Outcome::Blocked,
            }
        }
        for label in &edge.labels {
            if label.holds(pre, &writes) == Tri::False {
                return Outcome::Blocked;
            }
        }
        for (path, value) in &writes {
            if !PathType::of(path).fits(value) {
                return Outcome::BadWrite {
                    path: path.clone(),
                    value: value.clone(),
                };
            }
        }
        let mut after = pre.clone();
        for (p, v) in &writes {
            after.insert(p.clone(), v.clone());
        }
        Outcome::Moved { writes, after }
    }
}

/// One step of a run.
#[derive(Debug, Clone)]
pub struct Step {
    pub edge: usize,
    pub writes: Writes,
    pub after: State,
}

/// A run from posted inputs, taking the first possible move each time.
#[derive(Debug, Clone)]
pub struct Run {
    pub start: String,
    pub steps: Vec<Step>,
    /// Where the run stopped, and why: no move possible, or a write that
    /// does not fit (`BadWrite`), or more than one move possible.
    pub stop: RunStop,
    pub final_state: State,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunStop {
    NoMove,
    BadWrite { edge: usize, path: String, value: Value },
    Limit,
}

impl Run {
    pub fn path(&self, m: &Machine) -> String {
        let mut out = self.start.clone();
        for s in &self.steps {
            out.push_str(" -> ");
            out.push_str(&m.edges[s.edge].to);
        }
        out
    }
}

/// Run `m` from its first start node. Each input is written by the first
/// move that may write it; the run takes the first possible move each step.
pub fn run(m: &Machine, inputs: &[(String, Value)], limit: usize) -> Run {
    let start = m.start.first().cloned().unwrap_or_default();
    let mut node = start.clone();
    let mut state = State::new();
    let mut steps = Vec::new();
    let mut pending: Vec<(String, Value)> = inputs.to_vec();
    let stop = loop {
        if steps.len() >= limit {
            break RunStop::Limit;
        }
        let mut moved = None;
        let mut bad = None;
        for e in m.edges_from(&node) {
            let extra: Writes = pending
                .iter()
                .filter(|(p, _)| e.leaves_open(p))
                .cloned()
                .collect();
            match m.try_move(e, &state, &extra) {
                Outcome::Moved { writes, after } => {
                    moved = Some((e.index, writes, after));
                    break;
                }
                Outcome::BadWrite { path, value } => {
                    bad.get_or_insert((e.index, path, value));
                }
                Outcome::Blocked => {}
            }
        }
        match (moved, bad) {
            (Some((edge, writes, after)), _) => {
                pending.retain(|(p, _)| !writes.contains_key(p));
                node = m.edges[edge].to.clone();
                state = after.clone();
                steps.push(Step {
                    edge,
                    writes,
                    after,
                });
            }
            (None, Some((edge, path, value))) => break RunStop::BadWrite { edge, path, value },
            (None, None) => break RunStop::NoMove,
        }
    };
    Run {
        start,
        steps,
        stop,
        final_state: state,
    }
}
