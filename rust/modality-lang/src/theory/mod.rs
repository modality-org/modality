//! Predicate theory: bounded, versioned reasoning about how predicate atoms
//! interact on one edge.
//!
//! Predicates reach the theory only through declarations (`decl.rs`) that
//! elaborate them into a closed set of sorts (`sort.rs`). The decision
//! procedures are per sort (`order.rs`, `signers.rs`, `paths.rs`,
//! `literals.rs`, `text.rs`). Everything is three-valued and conservative:
//! `Unknown` never kills an edge (a box counts it; a diamond does not).
//! `False` is a contradiction the sort procedures found; `True` is a world
//! built and checked (`witness.rs`).
//!
//! The specification and its proofs are the Lean development in
//! `experiments/predicate-theory/lean`; this module is its transliteration,
//! and the harness (`tests::rust_and_lean_agree`) holds the two together.
//!
//! `TheoryVersion::V0` is today's behaviour: every query returns `Unknown`.
//! `V1` is the per-edge theory. The version is protocol; it is chosen by
//! the caller, never inferred.
//!
//! No floats, no external crates, deterministic output ordering.

pub mod decl;
pub mod flow;
pub mod fragment;
pub mod literals;
pub mod order;
pub mod paths;
pub mod rational;
pub mod signers;
pub mod sort;
pub mod standard;
pub mod state;
pub mod text;
pub mod witness;

pub use decl::{expand, ContractRegistry, Declaration, Expansion, Registry};
pub use fragment::Template;
pub use rational::Rational;
pub use sort::{Constraint, Lit, Op, Term, TextOp};
pub use standard::{standard, StandardRegistry};
pub use state::{Lookup, MapState, NoState, StateValue, StateView};
pub use witness::World;

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
    /// `V1`, and a rule check drops the edges state flow proves no run
    /// from the rule's evaluation node takes (`flow`). Models are refused
    /// only as under `V1`.
    V2,
    /// `V2`, and the evaluator compares numbers exactly (`num_*`,
    /// `amount_in_range`) instead of as `f64`. The theory is `V2`'s.
    V3,
}

impl TheoryVersion {
    /// The spelling used in network parameters and replay artifacts.
    pub fn as_str(self) -> &'static str {
        match self {
            TheoryVersion::V0 => "v0",
            TheoryVersion::V1 => "v1",
            TheoryVersion::V2 => "v2",
            TheoryVersion::V3 => "v3",
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
            "v2" => Ok(TheoryVersion::V2),
            "v3" => Ok(TheoryVersion::V3),
            other => Err(format!(
                "unknown predicate theory version `{other}` (this build knows v0, v1, v2, and v3)"
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

/// Result of a consistency query: the offending literals when `False`, the
/// witness when `True`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub tri: Tri,
    /// Sorted, deterministic. Empty unless `tri == False`. May include what
    /// the accepted state says about a path.
    pub offending: Vec<Lit>,
    /// A world where every literal holds. Under a known accepted state, the
    /// commit only (`World::state` is empty).
    pub witness: Option<World>,
}

impl Verdict {
    pub(crate) fn unknown() -> Self {
        Self {
            tri: Tri::Unknown,
            offending: Vec::new(),
            witness: None,
        }
    }
    fn yes(w: World) -> Self {
        Self {
            tri: Tri::True,
            offending: Vec::new(),
            witness: Some(w),
        }
    }
    fn no(mut offending: Vec<Lit>) -> Self {
        offending.sort();
        offending.dedup();
        Self {
            tri: Tri::False,
            offending,
            witness: None,
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

    /// Consistency over already-elaborated literals. `exact`: the literals
    /// say exactly what the labels say, so a witness for them is an
    /// accepted commit. Lean: `consistent` / `runtime` in `Spec.lean`.
    pub fn consistent_lits(&self, lits: &[Lit], exact: bool) -> Verdict {
        if self.version == TheoryVersion::V0 {
            return Verdict::unknown();
        }
        let mut all = lits.to_vec();
        all.extend(self.state_facts(lits));
        all.sort();
        all.dedup();
        if let Some(why) = dead(&all) {
            return Verdict::no(why);
        }
        if let Some(why) = self.state_denied(lits) {
            return Verdict::no(why);
        }
        if !exact || lits.iter().any(|l| l.is_opaque() && !self.free(l)) {
            return Verdict::unknown();
        }
        let decided: Vec<Lit> = lits.iter().filter(|l| !l.is_opaque()).cloned().collect();
        let w = if self.state.is_known() {
            witness::build_in(self.state, &decided)
        } else {
            witness::build(&decided)
        };
        match w {
            Some(w) => Verdict::yes(w),
            None => Verdict::unknown(),
        }
    }

    /// An external literal the commit decides: `-p` by leaving the evidence
    /// out, `+p` by carrying it, which needs the key its first argument
    /// names when the state is known. `dead` has already refused `+p -p`.
    /// Lean: `external_live_sound`.
    fn free(&self, l: &Lit) -> bool {
        let Constraint::Opaque { name, args } = &l.c else {
            return false;
        };
        if !self.registry.external(name) {
            return false;
        }
        !l.positive
            || !self.state.is_known()
            || args.first().is_some_and(|key| {
                matches!(
                    self.state.value_at(key),
                    Lookup::Present(StateValue::Text(_))
                )
            })
    }

    /// Can one commit meet `lits` in **every** accepted state where `facts`
    /// hold? Sufficient, not necessary. Every literal that reads state must
    /// follow from `facts`. Signatures must not depend on keys `facts`
    /// leave open: with only positive signature literals the commit signs
    /// every key present, which needs the keys to be known; with only
    /// negative ones it signs none; with both, every one is `signed` at a
    /// known key. A body or method literal depends on the commit alone.
    ///
    /// `consistent_lits` asks whether some state and commit meet the
    /// literals; this is what a diamond needs after a step, where a run may
    /// arrive in any state the facts allow.
    pub fn robust(&self, lits: &[Lit], facts: &[Lit]) -> bool {
        let pinned = |p: &str| {
            facts
                .iter()
                .any(|f| f.positive && matches!(&f.c, Constraint::Eq { path, .. } if path == p))
        };
        let keys_under = |q: &str| {
            facts
                .iter()
                .filter(|f| f.positive)
                .filter_map(|f| match &f.c {
                    Constraint::Eq { path, lit } if sort::key_under(path, q) => Some(lit),
                    _ => None,
                })
                .collect::<BTreeSet<_>>()
                .len()
        };
        let signature = |l: &Lit| {
            matches!(
                l.c,
                Constraint::Signer { .. }
                    | Constraint::SignerCount { .. }
                    | Constraint::SignerAll { .. }
            )
        };
        let signs = lits.iter().any(|l| l.positive && signature(l));
        let refuses = lits.iter().any(|l| !l.positive && signature(l));
        lits.iter().all(|l| match &l.c {
            Constraint::Label { .. } | Constraint::Writes { .. } | Constraint::Posts { .. } => true,
            Constraint::Signer { id } => (!l.positive && !signs) || pinned(id),
            Constraint::SignerCount { prefix, at_least } => {
                if l.positive {
                    !refuses && keys_under(prefix) >= *at_least as usize
                } else {
                    !signs
                }
            }
            Constraint::SignerAll { prefix } => {
                if l.positive {
                    !refuses && keys_under(prefix) >= 1
                } else {
                    !signs
                }
            }
            Constraint::Opaque { name, args } => {
                self.registry.external(name)
                    && (!l.positive
                        || args.first().is_some_and(|k| pinned(&sort::norm_path(k))))
            }
            _ => {
                let mut without = facts.to_vec();
                without.push(l.negated());
                self.consistent_lits(&without, false).tri == Tri::False
            }
        })
    }

    /// What the accepted state says about every path the literals mention,
    /// as literals true in every world with that state. Lean: `stateFacts`.
    pub fn state_facts(&self, lits: &[Lit]) -> Vec<Lit> {
        let mut out = Vec::new();
        for p in witness::all_paths(lits) {
            match self.state.value_at(&p) {
                Lookup::Present(StateValue::Num(q)) => out.push(Lit::pos(Constraint::Order {
                    lhs: Term::Path(p),
                    op: Op::Eq,
                    rhs: Term::Const(q),
                })),
                Lookup::Present(StateValue::Text(s)) => {
                    out.push(Lit::pos(Constraint::Eq { path: p, lit: s }))
                }
                Lookup::Present(StateValue::Bool(b)) => {
                    out.push(Lit::pos(Constraint::Is { path: p, value: b }))
                }
                Lookup::Present(StateValue::Structured) => {
                    out.push(Lit::pos(Constraint::Exists { path: p.clone() }));
                    for value in [true, false] {
                        out.push(Lit::neg(Constraint::Is {
                            path: p.clone(),
                            value,
                        }));
                    }
                    out.push(Lit::neg(Constraint::Eq2 {
                        a: p.clone(),
                        b: p.clone(),
                    }));
                    out.push(Lit::neg(Constraint::Order {
                        lhs: Term::Path(p.clone()),
                        op: Op::Le,
                        rhs: Term::Path(p),
                    }));
                }
                Lookup::Absent => out.push(Lit::neg(Constraint::Exists { path: p })),
                Lookup::Unknown | Lookup::Present(StateValue::Other) => {}
            }
        }
        out
    }

    /// Under a known state the posted keys are a closed set, which no
    /// literal can say. Lean: `Lit.stateDenied`.
    fn state_denied(&self, lits: &[Lit]) -> Option<Vec<Lit>> {
        let signed_in: Vec<String> = lits
            .iter()
            .filter(|l| l.positive)
            .filter_map(|l| match &l.c {
                Constraint::Signer { id } => match self.state.value_at(id) {
                    Lookup::Present(StateValue::Text(k)) => Some(k),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        for lit in lits {
            let denied = match (&lit.c, lit.positive) {
                (Constraint::SignerCount { prefix, at_least }, true) => self
                    .state
                    .keys_under(prefix)
                    .is_some_and(|k| (k.len() as u64) < u64::from(*at_least)),
                (Constraint::SignerAll { prefix }, true) => {
                    self.state.keys_under(prefix).is_some_and(|k| k.is_empty())
                }
                (Constraint::SignerAll { prefix }, false) => self
                    .state
                    .keys_under(prefix)
                    .is_some_and(|k| !k.is_empty() && k.iter().all(|x| signed_in.contains(x))),
                _ => false,
            };
            if denied {
                let mut why = vec![lit.clone()];
                if !lit.positive {
                    why.extend(
                        lits.iter()
                            .filter(|l| l.positive && matches!(l.c, Constraint::Signer { .. }))
                            .cloned(),
                    );
                }
                return Some(why);
            }
        }
        None
    }

    /// Do the edge's atoms entail the goal atom?
    ///
    /// `True`: every way the goal could fail contradicts the premises.
    /// `False`: a witness where the premises hold and the goal fails; only
    /// when the premises and the goal's declaration are exact. `Unknown`:
    /// cannot tell. Lean: `entailsV` in `Spec.lean`.
    pub fn entails(&self, premises: &[Property], goal: &Property) -> Tri {
        if premises.iter().any(|p| p == goal) {
            return Tri::True;
        }
        if self.version == TheoryVersion::V0 {
            return Tri::Unknown;
        }
        let (lits, exact) = self.expand_all(premises);
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
        // For +P, one query per sufficient atom (its negation); for -P, one
        // query with the necessary atoms.
        let template = if positive_goal {
            &decl.sufficient
        } else {
            &decl.necessary
        };
        let Some(cs) = template.as_ref().and_then(|t| t.instantiate(&args)) else {
            return Tri::Unknown;
        };
        let queries: Vec<Vec<Lit>> = if positive_goal {
            cs.into_iter().map(|c| vec![Lit::neg(c)]).collect()
        } else {
            vec![cs.into_iter().map(Lit::pos).collect()]
        };
        let exact = exact && decl.sufficient == decl.necessary;
        let mut all_dead = true;
        let mut any_live = false;
        for q in queries {
            let mut with = lits.clone();
            with.extend(q);
            with.sort();
            with.dedup();
            match self.consistent_lits(&with, exact).tri {
                Tri::False => {}
                Tri::True => {
                    all_dead = false;
                    any_live = true;
                }
                Tri::Unknown => all_dead = false,
            }
        }
        if all_dead {
            Tri::True
        } else if any_live {
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
                let ord = l.cmp_exact(&r);
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

/// Structural contradiction: no world satisfies every literal. Lean: `dead`
/// in `Decide.lean`. The explanation is the literals that clash.
pub fn dead(lits: &[Lit]) -> Option<Vec<Lit>> {
    let positives: BTreeSet<&Constraint> =
        lits.iter().filter(|l| l.positive).map(|l| &l.c).collect();
    for lit in lits.iter().filter(|l| !l.positive) {
        if positives.contains(&lit.c) {
            return Some(vec![lit.clone(), lit.negated()]);
        }
    }
    let classes = text::TextClasses::new(lits);
    if let Some(why) = literals::check(lits, &classes) {
        return Some(why);
    }
    if let Some(why) = paths::check(lits) {
        return Some(why);
    }
    if let Some(why) = signers::check(&signers::Ctx {
        lits,
        classes: &classes,
    }) {
        return Some(why);
    }
    let numeric = literals::forced_numeric(lits);
    order::check(&order::Ctx {
        lits,
        numeric: &numeric,
    })
}

#[cfg(test)]
mod tests;
