//! Existence, Boolean, and text-equality reasoning.
//!
//! - Every positive value constraint on `p` forces `p` to exist; `-exists(p)`
//!   then contradicts (cases E3, C6).
//! - A path is not `true` and `false` at once.
//! - Text paths form equality classes via `(= a b)`; a class holds at most
//!   one literal (case E1). Substring predicates are checked only against a
//!   class literal (case E5); otherwise they are left alone.

use super::sort::{Constraint, Lit, TextOp};
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

pub fn check(lits: &[Lit], forced: &BTreeSet<String>) -> Option<Vec<Lit>> {
    // -exists(p) with p forced.
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

    // Text classes.
    let mut paths: Vec<&str> = Vec::new();
    for lit in lits {
        match &lit.c {
            Constraint::Eq { path, .. } | Constraint::Text { path, .. } => paths.push(path),
            Constraint::Eq2 { a, b } => {
                paths.push(a);
                paths.push(b);
            }
            _ => {}
        }
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return None;
    }
    let idx: BTreeMap<&str, usize> = paths.iter().enumerate().map(|(i, p)| (*p, i)).collect();
    let mut parent: Vec<usize> = (0..paths.len()).collect();
    fn find(parent: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while parent[r] != r {
            r = parent[r];
        }
        r
    }
    for lit in lits.iter().filter(|l| l.positive) {
        if let Constraint::Eq2 { a, b } = &lit.c {
            let (ra, rb) = (
                find(&mut parent, idx[a.as_str()]),
                find(&mut parent, idx[b.as_str()]),
            );
            if ra != rb {
                let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
                parent[hi] = lo;
            }
        }
    }
    // One literal per class.
    let mut class_lit: BTreeMap<usize, (&str, &Lit)> = BTreeMap::new();
    for lit in lits.iter().filter(|l| l.positive) {
        if let Constraint::Eq { path, lit: value } = &lit.c {
            let r = find(&mut parent, idx[path.as_str()]);
            match class_lit.get(&r) {
                Some((v, other)) if *v != value.as_str() => {
                    let mut why = vec![lit.clone(), (*other).clone()];
                    why.extend(
                        lits.iter()
                            .filter(|l| l.positive && matches!(l.c, Constraint::Eq2 { .. }))
                            .cloned(),
                    );
                    why.sort();
                    why.dedup();
                    return Some(why);
                }
                Some(_) => {}
                None => {
                    class_lit.insert(r, (value.as_str(), lit));
                }
            }
        }
    }
    for lit in lits.iter().filter(|l| !l.positive) {
        match &lit.c {
            Constraint::Eq { path, lit: value } => {
                let r = find(&mut parent, idx[path.as_str()]);
                if let Some((v, other)) = class_lit.get(&r) {
                    if *v == value.as_str() {
                        let mut why = vec![lit.clone(), (*other).clone()];
                        why.sort();
                        return Some(why);
                    }
                }
            }
            // `(= a a)` is false when `a` holds no string, so `-(= a a)`
            // contradicts only when a positive literal makes `a` a string.
            // Distinct paths share a class only through positive `(= a b)`,
            // which does.
            Constraint::Eq2 { a, b } => {
                if find(&mut parent, idx[a.as_str()]) != find(&mut parent, idx[b.as_str()]) {
                    continue;
                }
                let texts_a = |l: &&Lit| {
                    l.positive
                        && match &l.c {
                            Constraint::Eq { path, .. } | Constraint::Text { path, .. } => {
                                path == a
                            }
                            Constraint::Eq2 { a: x, b: y } => x == a || y == a,
                            _ => false,
                        }
                };
                let mut why: Vec<Lit> = if a == b {
                    lits.iter().filter(texts_a).cloned().collect()
                } else {
                    lits.iter()
                        .filter(|l| l.positive && matches!(l.c, Constraint::Eq2 { .. }))
                        .cloned()
                        .collect()
                };
                if why.is_empty() {
                    continue;
                }
                why.push(lit.clone());
                why.sort();
                return Some(why);
            }
            _ => {}
        }
    }
    // Substring predicates against a known class literal.
    for lit in lits {
        if let Constraint::Text { path, op, needle } = &lit.c {
            let r = find(&mut parent, idx[path.as_str()]);
            if let Some((v, other)) = class_lit.get(&r) {
                let holds = text_holds(*op, v, needle);
                if holds != lit.positive {
                    let mut why = vec![lit.clone(), (*other).clone()];
                    why.sort();
                    return Some(why);
                }
            }
        }
    }
    None
}
