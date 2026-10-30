//! Invariants as facts about states, and the three questions `--inductive`
//! asks of them on a search domain.
//!
//! An invariant `on q -> *` whose formulas are `always([L] false)` over
//! predicates says, of every state a run leaves `q` from, that the labels
//! `L` are not all true. That is a fact about the state at `q`.

use super::formula::{match_move, safety_labels};
use super::invariants::InvariantFile;
use super::label::{Kind, Label};
use super::machine::{Machine, Outcome};
use super::search::{Domain, Graph};
use super::value::Value;
use super::{State, Tri, Writes};
use crate::ast::FormulaExpr;
use std::collections::{BTreeMap, HashSet};

/// One formula of an invariant, as a fact about the state at its nodes:
/// not every label in `labels` holds.
#[derive(Debug, Clone)]
pub struct Fact {
    /// `invariant.formula`
    pub name: String,
    pub formula: String,
    pub invariant: String,
    pub nodes: Vec<String>,
    pub labels: Vec<Label>,
    pub line: usize,
    pub col: usize,
}

impl Fact {
    pub fn holds(&self, state: &State) -> bool {
        let none = Writes::new();
        !self
            .labels
            .iter()
            .all(|l| l.holds(state, &none) == Tri::True)
    }
}

/// The facts an invariant file gives each node.
pub fn facts(file: &InvariantFile) -> Result<Vec<Fact>, String> {
    let mut out = Vec::new();
    for inv in &file.invariants {
        for f in &inv.formulas {
            let name = format!("{}.{}", inv.name, f.name);
            let ls = safety_labels(&f.expr).ok_or_else(|| {
                format!(
                    "{name}: an invariant formula is `always([...] false)`, refusing the moves \
                     out of its node that start where the fact is false"
                )
            })?;
            let labels = ls
                .iter()
                .map(Label::from_property)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("{name}: {e}"))?;
            if let Some(l) = labels.iter().find(|l| !matches!(l.kind, Kind::Pred { .. })) {
                return Err(format!(
                    "{name}: `{}` is about a move, not a state; an invariant's labels are \
                     predicates that read the state a move starts from",
                    l.text()
                ));
            }
            out.push(Fact {
                name,
                formula: f.name.clone(),
                invariant: inv.name.clone(),
                nodes: inv.nodes.clone(),
                labels,
                line: f.line,
                col: f.col,
            });
        }
    }
    Ok(out)
}

pub fn facts_at<'a>(facts: &'a [Fact], node: &str) -> Vec<&'a Fact> {
    facts.iter().filter(|f| f.nodes.iter().any(|n| n == node)).collect()
}

pub fn annotated(facts: &[Fact]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in facts {
        for n in &f.nodes {
            if !out.contains(n) {
                out.push(n.clone());
            }
        }
    }
    out
}

/// Every string over `alphabet` up to `max_len`, shortest first.
pub fn strings(alphabet: &[String], max_len: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut layer = vec![String::new()];
    for _ in 0..max_len {
        let mut next = Vec::new();
        for s in &layer {
            for a in alphabet {
                next.push(format!("{s}{a}"));
            }
        }
        out.extend(next.iter().cloned());
        layer = next;
    }
    out
}

/// Every state over the domain's paths: one value for each.
pub fn domain_states(d: &Domain) -> Vec<State> {
    let mut out = vec![State::new()];
    for (path, values) in &d.values {
        let mut next = Vec::with_capacity(out.len() * values.len());
        for s in &out {
            for v in values {
                let mut s = s.clone();
                s.insert(path.clone(), v.clone());
                next.push(s);
            }
        }
        out = next;
    }
    out
}

/// Every state over the domain's paths where the facts hold, each fact
/// checked as soon as every path it reads has a value.
pub fn states_where(d: &Domain, facts: &[&Fact]) -> Vec<State> {
    let n = d.values.len();
    let index = |p: &str| d.values.iter().position(|(q, _)| q == p);
    let mut checks: Vec<Vec<&Fact>> = vec![Vec::new(); n.max(1)];
    for f in facts {
        let at = f
            .labels
            .iter()
            .flat_map(|l| l.terms().into_iter().flat_map(|t| t.paths()))
            .map(|p| index(&p).unwrap_or(n.saturating_sub(1)))
            .max()
            .unwrap_or(0);
        checks[at.min(n.saturating_sub(1))].push(*f);
    }
    fn dfs(i: usize, s: &mut State, d: &Domain, checks: &[Vec<&Fact>], out: &mut Vec<State>) {
        if i == d.values.len() {
            out.push(s.clone());
            return;
        }
        let (path, values) = &d.values[i];
        for v in values {
            s.insert(path.clone(), v.clone());
            if checks[i].iter().all(|f| f.holds(s)) {
                dfs(i + 1, s, d, checks, out);
            }
        }
        s.remove(path);
    }
    let mut out = Vec::new();
    if n == 0 {
        return out;
    }
    dfs(0, &mut State::new(), d, &checks, &mut out);
    out
}

/// A rule for `--inductive`: its name, and the labels of `always([L] false)`.
pub struct SafetyRule<'a> {
    pub name: String,
    pub labels: Vec<Label>,
    pub expr: &'a FormulaExpr,
}

#[derive(Debug, Clone)]
pub enum Finding {
    /// A run reaches the node in a state where a fact is false.
    NotMet {
        node: String,
        config: usize,
        broken: Vec<String>,
    },
    /// From a state the invariant allows, a step leaves it false.
    NotKept {
        edge: usize,
        before: State,
        writes: Writes,
        after: State,
        broken: Vec<String>,
        reached: bool,
    },
    /// From a state the invariant allows, a move breaks a rule.
    TooWeak {
        edge: usize,
        rule: String,
        before: State,
        writes: Writes,
        reached: bool,
    },
}

pub struct Report {
    pub reached_states: usize,
    pub drawn_states: usize,
    pub findings: Vec<Finding>,
}

/// Havoc choices for a move on `edge` (inputs are already in the state).
fn havoc(m: &Machine, edge: usize, d: &Domain) -> Vec<Writes> {
    let e = &m.edges[edge];
    let mut out = vec![Writes::new()];
    for (path, values) in &d.havoc {
        if !e.leaves_open(path) {
            continue;
        }
        let mut next = Vec::new();
        for w in &out {
            next.push(w.clone());
            for v in values {
                let mut w = w.clone();
                w.insert(path.clone(), v.clone());
                next.push(w);
            }
        }
        out = next;
    }
    out
}

pub fn check(
    m: &Machine,
    d: &Domain,
    g: &Graph,
    facts: &[Fact],
    rules: &[SafetyRule],
) -> Report {
    let nodes = annotated(facts);
    let mut findings = Vec::new();
    let mut reached_states = 0;
    let mut reached: HashSet<(String, State)> = HashSet::new();
    for (i, c) in g.configs.iter().enumerate() {
        if !nodes.contains(&c.node) {
            continue;
        }
        reached_states += 1;
        reached.insert((c.node.clone(), c.state.clone()));
        let broken: Vec<String> = facts_at(facts, &c.node)
            .into_iter()
            .filter(|f| !f.holds(&c.state))
            .map(|f| f.name.clone())
            .collect();
        if !broken.is_empty() && !findings.iter().any(|f| matches!(f, Finding::NotMet { .. })) {
            findings.push(Finding::NotMet {
                node: c.node.clone(),
                config: i,
                broken,
            });
        }
    }

    let mut drawn = 0;
    let mut seen_kept: BTreeMap<usize, ()> = BTreeMap::new();
    let mut seen_weak: BTreeMap<String, ()> = BTreeMap::new();
    for node in &nodes {
        let here = facts_at(facts, node);
        let states = states_where(d, &here);
        for s in &states {
            drawn += 1;
            let was_reached = reached.contains(&(node.clone(), s.clone()));
            for e in m.edges_from(node) {
                for extra in havoc(m, e.index, d) {
                    let Outcome::Moved { writes, after } = m.try_move(e, s, &extra) else {
                        continue;
                    };
                    let target = facts_at(facts, &e.to);
                    let broken: Vec<String> = target
                        .iter()
                        .filter(|f| !f.holds(&after))
                        .map(|f| f.name.clone())
                        .collect();
                    if !broken.is_empty() && !seen_kept.contains_key(&e.index) {
                        seen_kept.insert(e.index, ());
                        findings.push(Finding::NotKept {
                            edge: e.index,
                            before: s.clone(),
                            writes: writes.clone(),
                            after: after.clone(),
                            broken,
                            reached: was_reached,
                        });
                    }
                    for r in rules {
                        if seen_weak.contains_key(&r.name) {
                            continue;
                        }
                        if match_move(&r.labels, s, &writes, e) == Tri::True {
                            seen_weak.insert(r.name.clone(), ());
                            findings.push(Finding::TooWeak {
                                edge: e.index,
                                rule: r.name.clone(),
                                before: s.clone(),
                                writes: writes.clone(),
                                reached: was_reached,
                            });
                        }
                    }
                }
            }
        }
    }
    Report {
        reached_states,
        drawn_states: drawn,
        findings,
    }
}

/// A value list for a domain entry: listed values, or every string over an
/// alphabet up to a length.
pub fn text_values(alphabet: &[String], max_len: usize) -> Vec<Value> {
    strings(alphabet, max_len).into_iter().map(Value::Text).collect()
}
