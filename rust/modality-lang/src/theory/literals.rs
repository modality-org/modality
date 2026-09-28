//! Existence, Boolean, text, and type reasoning.
//!
//! - Every positive value constraint on `p` forces `p` to exist; `-exists(p)`
//!   then contradicts (cases E3, C6).
//! - A path is not `true` and `false` at once.
//! - Text classes (`text.rs`) hold at most one literal (case E1); a denied
//!   equality inside a class, a substring test the class literal fails
//!   (case E5), or one key both signed and not (case K3) contradict. So do
//!   a denied substring test that a required one implies (`+ends-with
//!   "ab"`, `-contains "b"`) or with the empty needle, and two required
//!   prefixes (suffixes) neither of which extends the other.
//! - One path does not hold two types (a number and a string, a number and
//!   a boolean, a string and a boolean).
//!
//! Lean twins: checks 2, 3, 5, and 8 in `Decide.lean`.

use super::sort::{Constraint, Lit, Term, TextOp};
use super::text::TextClasses;
use std::collections::{BTreeMap, BTreeSet};

/// Paths forced to exist by positive value constraints in `lits`.
pub fn forced_paths(lits: &[Lit]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for lit in lits.iter().filter(|l| l.positive) {
        match &lit.c {
            Constraint::Order { lhs, rhs, .. } => {
                for t in [lhs, rhs] {
                    if let super::sort::Term::Path(p) = t {
                        out.insert(p.clone());
                    }
                }
            }
            Constraint::Eq { path, .. }
            | Constraint::Text { path, .. }
            | Constraint::Is { path, .. }
            | Constraint::Exists { path } => {
                out.insert(path.clone());
            }
            Constraint::Eq2 { a, b } => {
                out.insert(a.clone());
                out.insert(b.clone());
            }
            Constraint::Signer { id } => {
                out.insert(id.clone());
            }
            _ => {}
        }
    }
    out
}

/// Paths forced to hold a number: those in a positive order literal. Not
/// `exists`: path extensions are not type-checked on write, so a present
/// `.num` path may hold a string, and then every numeric predicate on it is
/// false (case A13).
pub fn forced_numeric(lits: &[Lit]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for lit in lits.iter().filter(|l| l.positive) {
        if let Constraint::Order { lhs, rhs, .. } = &lit.c {
            for t in [lhs, rhs] {
                if let super::sort::Term::Path(p) = t {
                    out.insert(p.clone());
                }
            }
        }
    }
    out
}

fn text_holds(op: TextOp, value: &str, needle: &str) -> bool {
    match op {
        TextOp::Contains => value.contains(needle),
        TextOp::StartsWith => value.starts_with(needle),
        TextOp::EndsWith => value.ends_with(needle),
    }
}

pub fn check(lits: &[Lit], classes: &TextClasses) -> Option<Vec<Lit>> {
    // -exists(p) with p forced.
    let forced = forced_paths(lits);
    for lit in lits.iter().filter(|l| !l.positive) {
        if let Constraint::Exists { path } = &lit.c {
            if forced.contains(path) {
                let mut why: Vec<Lit> = lits
                    .iter()
                    .filter(|l| l.positive && forced_paths(std::slice::from_ref(l)).contains(path))
                    .cloned()
                    .collect();
                why.push(lit.clone());
                why.sort();
                return Some(why);
            }
        }
    }

    // Booleans: `true` and `false` at once. (`-bool_true` with `-bool_false`
    // is satisfiable even when the path exists: it may hold a non-boolean.)
    let mut bool_pos: BTreeMap<&str, Vec<(bool, &Lit)>> = BTreeMap::new();
    for lit in lits.iter().filter(|l| l.positive) {
        if let Constraint::Is { path, value } = &lit.c {
            bool_pos
                .entry(path.as_str())
                .or_default()
                .push((*value, lit));
        }
    }
    for pos in bool_pos.values() {
        let has_t = pos.iter().find(|(v, _)| *v);
        let has_f = pos.iter().find(|(v, _)| !*v);
        if let (Some((_, t)), Some((_, f))) = (has_t, has_f) {
            let mut why = vec![(*t).clone(), (*f).clone()];
            why.sort();
            return Some(why);
        }
    }

    if let Some(why) = text(lits, classes) {
        return Some(why);
    }
    types(lits, classes)
}

fn positive_text(lits: &[Lit]) -> Vec<Lit> {
    let mut v: Vec<Lit> = lits
        .iter()
        .filter(|l| {
            l.positive
                && matches!(
                    l.c,
                    Constraint::Eq { .. }
                        | Constraint::Eq2 { .. }
                        | Constraint::Text { .. }
                        | Constraint::Signer { .. }
                )
        })
        .cloned()
        .collect();
    v.sort();
    v
}

fn with(mut why: Vec<Lit>, lit: &Lit) -> Vec<Lit> {
    why.push(lit.clone());
    why.sort();
    why.dedup();
    why
}

fn text(lits: &[Lit], classes: &TextClasses) -> Option<Vec<Lit>> {
    if classes.two_lits().is_some() {
        return Some(positive_text(lits));
    }
    for lit in lits {
        let denied = match (&lit.c, lit.positive) {
            (Constraint::Eq { path, lit: s }, false) => classes.same_lit(path, s),
            (Constraint::Eq2 { a, b }, false) => classes.same_paths(a, b),
            (Constraint::Text { path, op, needle }, positive) => {
                classes
                    .class_lits(path)
                    .iter()
                    .any(|s| text_holds(*op, s, needle) != positive)
                    || (!positive
                        && classes.texted(path)
                        && (needle.is_empty() || implied_test(lits, classes, path, *op, needle)))
            }
            (Constraint::Signer { id: b }, false) => lits.iter().any(|l| {
                l.positive
                    && matches!(&l.c, Constraint::Signer { id: a } if classes.same_paths(a, b))
            }),
            _ => false,
        };
        if denied {
            return Some(with(positive_text(lits), lit));
        }
    }
    for a in lits {
        for b in lits {
            if needles_clash(classes, a, b) {
                let mut why = vec![a.clone(), b.clone()];
                why.sort();
                return Some(why);
            }
        }
    }
    None
}

/// Every string passing `op2` against `m` passes `op` against `n`. Lean:
/// `TextOp.implies`.
fn implies(op2: TextOp, m: &str, op: TextOp, n: &str) -> bool {
    match (op2, op) {
        (_, TextOp::Contains) => m.contains(n),
        (TextOp::StartsWith, TextOp::StartsWith) => m.starts_with(n),
        (TextOp::EndsWith, TextOp::EndsWith) => m.ends_with(n),
        _ => false,
    }
}

/// A positive substring test on `p`'s class implies `op n`.
fn implied_test(lits: &[Lit], classes: &TextClasses, p: &str, op: TextOp, n: &str) -> bool {
    lits.iter().any(|l| {
        l.positive
            && matches!(&l.c, Constraint::Text { path: q, op: op2, needle: m }
                if classes.same_paths(q, p) && implies(*op2, m, op, n))
    })
}

/// Two required prefixes (suffixes) of one string, neither extending the
/// other. Lean: `needlesClash`.
fn needles_clash(classes: &TextClasses, a: &Lit, b: &Lit) -> bool {
    if !a.positive || !b.positive {
        return false;
    }
    let (
        Constraint::Text {
            path: p,
            op: o1,
            needle: m,
        },
        Constraint::Text {
            path: q,
            op: o2,
            needle: n,
        },
    ) = (&a.c, &b.c)
    else {
        return false;
    };
    let extends = match (o1, o2) {
        (TextOp::StartsWith, TextOp::StartsWith) => {
            n.starts_with(m.as_str()) || m.starts_with(n.as_str())
        }
        (TextOp::EndsWith, TextOp::EndsWith) => n.ends_with(m.as_str()) || m.ends_with(n.as_str()),
        _ => return false,
    };
    !extends && classes.same_paths(p, q)
}

/// Paths a positive order literal forces to hold a number.
fn num_forced(lits: &[Lit]) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    for lit in lits.iter().filter(|l| l.positive) {
        if let Constraint::Order { lhs, rhs, .. } = &lit.c {
            for t in [lhs, rhs] {
                if let Term::Path(p) = t {
                    out.insert(p.as_str());
                }
            }
        }
    }
    out
}

fn types(lits: &[Lit], classes: &TextClasses) -> Option<Vec<Lit>> {
    let nums = num_forced(lits);
    let bools: BTreeSet<&str> = lits
        .iter()
        .filter(|l| l.positive)
        .filter_map(|l| match &l.c {
            Constraint::Is { path, .. } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    let clash = nums
        .iter()
        .find(|p| classes.texted(p) || bools.contains(*p))
        .copied()
        .or_else(|| classes.texted_paths().find(|p| bools.contains(p)));
    let p = clash?;
    let mut why: Vec<Lit> = lits
        .iter()
        .filter(|l| l.positive && forced_paths(std::slice::from_ref(l)).contains(p))
        .cloned()
        .collect();
    why.sort();
    Some(why)
}
