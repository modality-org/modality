//! Predicate theory: bounded, versioned reasoning about how predicate atoms
//! interact on one edge.
//!
//! Predicates reach the theory only through declarations (`decl.rs`) that
//! elaborate them into a closed set of sorts (`sort.rs`). The decision
//! procedures are per sort (`order.rs`, `signers.rs`, `paths.rs`,
//! `literals.rs`). Everything is three-valued and conservative: `Unknown`
//! never refuses anything.
//!
//! `TheoryVersion::V0` is today's behaviour: every query returns `Unknown`.
//! `V1` is the per-edge theory. The version is protocol; it is chosen by
//! the caller, never inferred.
//!
//! No floats, no external crates, deterministic output ordering.

pub mod decl;
pub mod fragment;
pub mod literals;
pub mod order;
pub mod paths;
pub mod rational;
pub mod signers;
pub mod sort;
pub mod standard;
pub mod state;

pub use decl::{expand, ContractRegistry, Declaration, Expansion, Registry};
pub use fragment::Template;
pub use rational::Rational;
pub use sort::{Constraint, Lit, Op, Term, TextOp};
pub use standard::{standard, StandardRegistry};
pub use state::{Lookup, MapState, NoState, StateValue, StateView};

use crate::ast::Property;
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum TheoryVersion {
    /// Atoms are opaque. Structural matching only. Every query is `Unknown`.
    #[default]
    V0,
    /// Per-edge theory over the V1 fragment.
    V1,
}

impl TheoryVersion {
    /// The spelling used in network parameters and replay artifacts.
    pub fn as_str(self) -> &'static str {
        match self {
            TheoryVersion::V0 => "v0",
            TheoryVersion::V1 => "v1",
        }
    }
}

impl std::fmt::Display for TheoryVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for TheoryVersion {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "v0" => Ok(TheoryVersion::V0),
            "v1" => Ok(TheoryVersion::V1),
            other => Err(format!(
                "unknown predicate theory version `{other}` (this build knows v0 and v1)"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    fn not(self) -> Tri {
        match self {
            Tri::True => Tri::False,
            Tri::False => Tri::True,
            Tri::Unknown => Tri::Unknown,
        }
    }
}

/// Result of a consistency query, with the offending literals when `False`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub tri: Tri,
    /// Sorted, deterministic. Empty unless `tri == False`.
    pub offending: Vec<Lit>,
}

impl Verdict {
    fn unknown() -> Self {
        Self {
            tri: Tri::Unknown,
            offending: Vec::new(),
        }
    }
    fn yes() -> Self {
        Self {
            tri: Tri::True,
            offending: Vec::new(),
        }
    }
    fn no(mut offending: Vec<Lit>) -> Self {
        offending.sort();
        offending.dedup();
        Self {
            tri: Tri::False,
            offending,
        }
    }
    pub fn explain(&self) -> Vec<String> {
        self.offending.iter().map(|l| l.to_string()).collect()
    }
}

pub struct Theory<'a> {
    pub version: TheoryVersion,
    registry: &'a dyn Registry,
    state: &'a dyn StateView,
}

impl<'a> Theory<'a> {
    pub fn new(
        version: TheoryVersion,
        registry: &'a dyn Registry,
        state: &'a dyn StateView,
    ) -> Self {
        Self {
            version,
            registry,
            state,
        }
    }

    /// Today's behaviour: standard registry, no state, `V0`.
    pub fn v0() -> Theory<'static> {
        Theory {
            version: TheoryVersion::V0,
            registry: standard(),
            state: &NoState,
        }
    }

    /// `V1` with the standard registry and no state.
    pub fn v1_structural() -> Theory<'static> {
        Theory {
            version: TheoryVersion::V1,
            registry: standard(),
            state: &NoState,
        }
    }

    pub fn expand(&self, p: &Property) -> Expansion {
        expand(self.registry, p)
    }

    /// Elaborate a label set. `exact` is true when a `True` verdict over
    /// the result is a real model.
    pub fn expand_all(&self, props: &[Property]) -> (Vec<Lit>, bool) {
        let mut lits = Vec::new();
        let mut exact = true;
        for p in props {
            let e = self.expand(p);
            exact &= e.exact;
            lits.extend(e.lits);
        }
        lits.sort();
        lits.dedup();
        (lits, exact)
    }

    /// Can these label atoms all hold on one edge?
    pub fn consistent(&self, props: &[Property]) -> Verdict {
        if self.version == TheoryVersion::V0 {
            return Verdict::unknown();
        }
        let (lits, exact) = self.expand_all(props);
        self.consistent_lits(&lits, exact)
    }

    /// Consistency over already-elaborated literals.
    pub fn consistent_lits(&self, lits: &[Lit], exact: bool) -> Verdict {
        if self.version == TheoryVersion::V0 {
            return Verdict::unknown();
        }
        // 1. Identical constraint with both signs.
        let positives: BTreeSet<&Constraint> =
            lits.iter().filter(|l| l.positive).map(|l| &l.c).collect();
        for lit in lits.iter().filter(|l| !l.positive) {
            if positives.contains(&lit.c) {
                return Verdict::no(vec![lit.clone(), lit.negated()]);
            }
        }
        // 2. Facts from state: a literal that is false now cannot hold.
        for lit in lits {
            if self.evaluate(lit) == Tri::False {
                return Verdict::no(vec![lit.clone()]);
            }
        }
        // 3. What is forced, by positive literals or by state. Existence and
        //    type are separate: a present `.num` path may hold a string.
        let mut exists = literals::forced_paths(lits);
        let mut numeric = literals::forced_numeric(lits);
        let mut members = BTreeSet::new();
        for lit in lits {
            for p in constraint_paths(&lit.c) {
                if let Lookup::Present(v) = self.state.value_at(&p) {
                    exists.insert(p.clone());
                    match v {
                        StateValue::Num(_) => {
                            numeric.insert(p);
                        }
                        StateValue::Text(_) if sort::ext(&p) == Some("id") => {
                            members.insert(p);
                        }
                        _ => {}
                    }
                }
            }
        }
        // 4. Per-sort procedures.
        if let Some(why) = literals::check(lits, &exists) {
            return Verdict::no(why);
        }
        if let Some(why) = paths::check(lits) {
            return Verdict::no(why);
        }
        if let Some(why) = signers::check(&signers::Ctx {
            lits,
            members: &members,
            state: self.state,
        }) {
            return Verdict::no(why);
        }
        let mut unknown = false;
        match order::check(&order::Ctx {
            lits,
            numeric: &numeric,
        }) {
            Ok(Some(why)) => return Verdict::no(why),
            Ok(None) => {}
            Err(order::Overflow) => unknown = true,
        }
        // 5. No contradiction found.
        if exact && !unknown && lits.iter().all(Lit::decidable) {
            Verdict::yes()
        } else {
            Verdict::unknown()
        }
    }

    /// Do the edge's atoms entail the goal atom?
    ///
    /// `True`: entailed. `False`: a countermodel exists in the theory.
    /// `Unknown`: cannot tell (opaque goal, non-exact premises, overflow).
    pub fn entails(&self, premises: &[Property], goal: &Property) -> Tri {
        if premises.iter().any(|p| p == goal) {
            return Tri::True;
        }
        if self.version == TheoryVersion::V0 {
            return Tri::Unknown;
        }
        let (mut lits, exact) = self.expand_all(premises);
        let (key, args) = decl::registry_key_and_args(goal);
        let decl = if goal.is_static() {
            None
        } else {
            self.registry.declaration(&key).filter(|d| d.accepts(&args))
        };
        let Some(decl) = decl else {
            return Tri::Unknown;
        };
        let positive_goal = goal.sign == crate::ast::PropertySign::Plus;

        // Sub-goals whose negation we add to the premises.
        let subgoals: Vec<Lit> = if positive_goal {
            let Some(t) = &decl.sufficient else {
                return Tri::Unknown;
            };
            let Some(cs) = t.instantiate(&args) else {
                return Tri::Unknown;
            };
            cs.into_iter().map(Lit::pos).collect()
        } else {
            // premises ⊨ ¬P  ⇐  premises ∧ necessary(P) inconsistent
            let Some(t) = &decl.necessary else {
                return Tri::Unknown;
            };
            let Some(cs) = t.instantiate(&args) else {
                return Tri::Unknown;
            };
            let mut with = lits.clone();
            with.extend(cs.into_iter().map(Lit::pos));
            with.sort();
            with.dedup();
            return match self.consistent_lits(&with, exact).tri {
                Tri::False => Tri::True,
                Tri::True => Tri::False,
                Tri::Unknown => Tri::Unknown,
            };
        };

        let mut all_entailed = true;
        let mut any_countermodel = false;
        for g in subgoals {
            let neg = g.negated();
            lits.push(neg.clone());
            lits.sort();
            let v = self.consistent_lits(&lits, exact && neg.decidable());
            lits.retain(|l| l != &neg);
            match v.tri {
                Tri::False => {}
                Tri::True => {
                    all_entailed = false;
                    any_countermodel = true;
                }
                Tri::Unknown => all_entailed = false,
            }
        }
        if all_entailed {
            Tri::True
        } else if any_countermodel {
            Tri::False
        } else {
            Tri::Unknown
        }
    }

    /// Truth of a literal under the state view. `Unknown` when the view
    /// does not know, or the sort is not about accepted state.
    pub fn evaluate(&self, lit: &Lit) -> Tri {
        if self.version == TheoryVersion::V0 {
            return Tri::Unknown;
        }
        let t = self.eval_constraint(&lit.c);
        if lit.positive {
            t
        } else {
            t.not()
        }
    }

    /// Typed read of a number: `Some(Some(v))` a number, `Some(None)` no
    /// number there (absent or another type; the predicate is false),
    /// `None` unknown.
    fn num_at(&self, t: &Term) -> Option<Option<Rational>> {
        match t {
            Term::Const(c) => Some(Some(*c)),
            Term::Path(p) => match self.state.value_at(p) {
                Lookup::Present(StateValue::Num(v)) => Some(Some(v)),
                other => typed_miss(other),
            },
        }
    }

    fn text_at(&self, p: &str) -> Option<Option<String>> {
        match self.state.value_at(p) {
            Lookup::Present(StateValue::Text(v)) => Some(Some(v)),
            other => typed_miss(other),
        }
    }

    fn eval_constraint(&self, c: &Constraint) -> Tri {
        match c {
            Constraint::Order { lhs, op, rhs } => {
                let (l, r) = (self.num_at(lhs), self.num_at(rhs));
                if matches!(l, Some(None)) || matches!(r, Some(None)) {
                    return Tri::False;
                }
                let (Some(Some(l)), Some(Some(r))) = (l, r) else {
                    return Tri::Unknown;
                };
                let Some(ord) = l.try_cmp(&r) else {
                    return Tri::Unknown;
                };
                let holds = match op {
                    Op::Lt => ord == Ordering::Less,
                    Op::Le => ord != Ordering::Greater,
                    Op::Eq => ord == Ordering::Equal,
                };
                if holds {
                    Tri::True
                } else {
                    Tri::False
                }
            }
            Constraint::Eq { path, lit } => match self.text_at(path) {
                None => Tri::Unknown,
                Some(None) => Tri::False,
                Some(Some(v)) => tri(v == *lit),
            },
            Constraint::Eq2 { a, b } => match (self.text_at(a), self.text_at(b)) {
                (Some(None), _) | (_, Some(None)) => Tri::False,
                (Some(Some(x)), Some(Some(y))) => tri(x == y),
                _ => Tri::Unknown,
            },
            Constraint::Text { path, op, needle } => match self.text_at(path) {
                None => Tri::Unknown,
                Some(None) => Tri::False,
                Some(Some(v)) => tri(match op {
                    TextOp::Contains => v.contains(needle.as_str()),
                    TextOp::StartsWith => v.starts_with(needle.as_str()),
                    TextOp::EndsWith => v.ends_with(needle.as_str()),
                }),
            },
            Constraint::Is { path, value } => match self.state.value_at(path) {
                Lookup::Present(StateValue::Bool(b)) => tri(b == *value),
                other => match typed_miss::<bool>(other) {
                    Some(_) => Tri::False,
                    None => Tri::Unknown,
                },
            },
            Constraint::Exists { path } => match self.state.value_at(path) {
                Lookup::Unknown => Tri::Unknown,
                Lookup::Absent => Tri::False,
                Lookup::Present(_) => Tri::True,
            },
            // Whether the key signed is pending-body; only "no key there"
            // is known from state.
            Constraint::Signer { id } => match self.text_at(id) {
                Some(None) => Tri::False,
                _ => Tri::Unknown,
            },
            Constraint::SignerCount { prefix, at_least } => {
                if *at_least == 0 {
                    return Tri::True;
                }
                match self.state.keys_under(prefix) {
                    Some(keys) if (keys.len() as u32) < *at_least => Tri::False,
                    _ => Tri::Unknown,
                }
            }
            Constraint::SignerAll { prefix } => match self.state.keys_under(prefix) {
                Some(keys) if keys.is_empty() => Tri::False,
                _ => Tri::Unknown,
            },
            Constraint::Writes { .. }
            | Constraint::Posts { .. }
            | Constraint::Label { .. }
            | Constraint::Opaque { .. } => Tri::Unknown,
        }
    }
}

/// A typed read that did not find its type: `Some(None)` when the path is
/// absent or holds a value of another known type, `None` when unknown.
fn typed_miss<T>(l: Lookup) -> Option<Option<T>> {
    match l {
        Lookup::Absent
        | Lookup::Present(
            StateValue::Num(_) | StateValue::Bool(_) | StateValue::Text(_) | StateValue::Structured,
        ) => Some(None),
        Lookup::Unknown | Lookup::Present(StateValue::Other) => None,
    }
}

fn tri(b: bool) -> Tri {
    if b {
        Tri::True
    } else {
        Tri::False
    }
}

fn constraint_paths(c: &Constraint) -> Vec<String> {
    match c {
        Constraint::Order { lhs, rhs, .. } => [lhs, rhs]
            .into_iter()
            .filter_map(|t| match t {
                Term::Path(p) => Some(p.clone()),
                _ => None,
            })
            .collect(),
        Constraint::Eq { path, .. }
        | Constraint::Text { path, .. }
        | Constraint::Is { path, .. }
        | Constraint::Exists { path } => vec![path.clone()],
        Constraint::Eq2 { a, b } => vec![a.clone(), b.clone()],
        Constraint::Signer { id } => vec![id.clone()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
