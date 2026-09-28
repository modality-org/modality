//! Prefix lattice for pending-body sorts.
//!
//! `posts(q)` ⇒ `writes(q)`; `writes(q)` ⇒ `writes(p)` and `posts(q)` ⇒
//! `posts(p)` for every prefix `p` of `q`. A forbidden prefix with a write
//! or post under it is a contradiction (case D1). Siblings are not
//! descendants (case D4). A post is an action with method `POST`, which is
//! what the static label `POST` reads (case K6). Lean twin: check 7 in
//! `Decide.lean`.

use super::sort::{under, Constraint, Lit};

pub fn check(lits: &[Lit]) -> Option<Vec<Lit>> {
    for neg in lits.iter().filter(|l| !l.positive) {
        match &neg.c {
            Constraint::Writes { path: p } => {
                if let Some(pos) = lits.iter().find(|l| {
                    l.positive
                        && matches!(&l.c,
                            Constraint::Writes { path: q } | Constraint::Posts { path: q } if under(q, p))
                }) {
                    let mut why = vec![neg.clone(), pos.clone()];
                    why.sort();
                    return Some(why);
                }
            }
            Constraint::Posts { path: p } => {
                if let Some(pos) = lits.iter().find(|l| {
                    l.positive && matches!(&l.c, Constraint::Posts { path: q } if under(q, p))
                }) {
                    let mut why = vec![neg.clone(), pos.clone()];
                    why.sort();
                    return Some(why);
                }
            }
            Constraint::Label { name } if name == "POST" => {
                if let Some(pos) = lits
                    .iter()
                    .find(|l| l.positive && matches!(&l.c, Constraint::Posts { .. }))
                {
                    let mut why = vec![neg.clone(), pos.clone()];
                    why.sort();
                    return Some(why);
                }
            }
            _ => {}
        }
    }
    None
}
