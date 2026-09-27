//! Finite-set reasoning over the `.id` keys posted under a prefix.
//!
//! Facts the procedure knows:
//! - `all-signed(p)` ⇒ at least one key under `p`, and every one signed
//! - `signed(id)` ⇒ the key at `id` exists and signed ⇒ `card(q) ≥ 1` for
//!   every prefix `q` that `id` lies under
//! - `card(p) ≥ n` ⇒ `card(q) ≥ n` for every prefix `q` of `p`
//! - with state: `card(p) ≥ n` is impossible when fewer than `n` keys are
//!   posted under `p`; `all-signed(p)` is impossible when none are
//!
//! It does **not** know `all-signed(/m) ⇒ all-signed(/m/a)`: the sub-prefix
//! may be empty (case C7).

use super::sort::{under, Constraint, Lit};
use super::state::StateView;
use std::collections::BTreeSet;

pub struct Ctx<'a> {
    pub lits: &'a [Lit],
    pub forced: &'a BTreeSet<String>,
    pub state: &'a dyn StateView,
}

/// Lower bound on `card(q)` derivable from positive literals, with the
/// literals that give it.
fn lower_bound(ctx: &Ctx, q: &str) -> (u32, Vec<Lit>) {
    let mut best = 0u32;
    let mut why = Vec::new();
    for lit in ctx.lits.iter().filter(|l| l.positive) {
        let n = match &lit.c {
            Constraint::SignerCount { prefix, at_least } if under(prefix, q) => *at_least,
            Constraint::SignerAll { prefix } if under(prefix, q) => 1,
            Constraint::Signer { id } if under(id, q) => 1,
            _ => continue,
        };
        if n > best {
            best = n;
            why = vec![lit.clone()];
        } else if n == best && n > 0 {
            why.push(lit.clone());
        }
    }
    (best, why)
}

pub fn check(ctx: &Ctx) -> Option<Vec<Lit>> {
    for lit in ctx.lits {
        match (&lit.c, lit.positive) {
            // -card(q) ≥ m: contradiction if m == 0, or a lower bound ≥ m.
            (Constraint::SignerCount { prefix, at_least }, false) => {
                if *at_least == 0 {
                    return Some(vec![lit.clone()]);
                }
                let (lb, mut why) = lower_bound(ctx, prefix);
                if lb >= *at_least {
                    why.push(lit.clone());
                    why.sort();
                    return Some(why);
                }
            }
            // -all-signed(p) with +all-signed(q), p under q, and p known non-empty.
            (Constraint::SignerAll { prefix: p }, false) => {
                let known_nonempty = ctx
                    .state
                    .keys_under(p)
                    .map(|k| !k.is_empty())
                    .unwrap_or(false)
                    || ctx.lits.iter().any(|l| {
                        l.positive && matches!(&l.c, Constraint::Signer { id } if under(id, p))
                    });
                if known_nonempty {
                    if let Some(q) = ctx.lits.iter().find(|l| {
                        l.positive
                            && matches!(&l.c, Constraint::SignerAll { prefix: q } if under(p, q))
                    }) {
                        let mut why = vec![lit.clone(), q.clone()];
                        why.sort();
                        return Some(why);
                    }
                }
            }
            // -signed(id) with +all-signed(p), id under p, id forced to exist.
            (Constraint::Signer { id }, false) => {
                if ctx.forced.contains(id) {
                    if let Some(q) = ctx.lits.iter().find(|l| {
                        l.positive
                            && matches!(&l.c, Constraint::SignerAll { prefix } if under(id, prefix))
                    }) {
                        let mut why = vec![lit.clone(), q.clone()];
                        why.sort();
                        return Some(why);
                    }
                }
            }
            // With state: +card(p) ≥ n needs n posted keys; +all-signed(p) needs one.
            (Constraint::SignerCount { prefix, at_least }, true) => {
                if let Some(keys) = ctx.state.keys_under(prefix) {
                    if (keys.len() as u32) < *at_least {
                        return Some(vec![lit.clone()]);
                    }
                }
            }
            (Constraint::SignerAll { prefix }, true) => {
                if let Some(keys) = ctx.state.keys_under(prefix) {
                    if keys.is_empty() {
                        return Some(vec![lit.clone()]);
                    }
                }
            }
            _ => {}
        }
    }
    None
}
