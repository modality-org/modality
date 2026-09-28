//! Finite-set reasoning over the `.id` keys posted under a prefix.
//!
//! Facts the procedure knows:
//! - `all-signed(p)` ⇒ at least one key under `p`, and every one signed
//! - `signed(id)` ⇒ the key at `id` exists and signed ⇒ `card(q) ≥ 1` for
//!   every prefix `q` that `id` lies under
//! - `card(p) ≥ n` ⇒ `card(q) ≥ n` for every prefix `q` of `p`
//! - a key forced to be posted under `q` (a path under `q` in a text class,
//!   case K2) is a member of `q`; distinct literal keys that are signed
//!   count separately (case K4)
//! - no key is ever posted under the root (case K5)
//!
//! It does **not** know `all-signed(/m) ⇒ all-signed(/m/a)`: the sub-prefix
//! may be empty (case C7), unless something posts a key there (case K7).
//!
//! Known state reaches this module as literals (`Theory::state_facts`);
//! the closed key set of a known state is checked in `Theory`. Lean twin:
//! check 6 in `Decide.lean`.

use super::sort::{is_id, key_under, no_keys, normalized, under, Constraint, Lit};
use super::text::TextClasses;
use std::collections::BTreeSet;

pub struct Ctx<'a> {
    pub lits: &'a [Lit],
    pub classes: &'a TextClasses,
}

impl Ctx<'_> {
    fn positive(&self) -> impl Iterator<Item = &Lit> {
        self.lits.iter().filter(|l| l.positive)
    }

    /// Some `+all-signed(r)` has `p` under `r`.
    fn all_signed_above(&self, p: &str) -> Option<&Lit> {
        self.positive()
            .find(|l| matches!(&l.c, Constraint::SignerAll { prefix: r } if under(p, r)))
    }

    /// Literal keys forced to be posted under `q` and signed.
    fn signed_lits(&self, q: &str) -> BTreeSet<&str> {
        let mut out = BTreeSet::new();
        for p in self.classes.texted_paths() {
            if !key_under(p, q) {
                continue;
            }
            let signs = self
                .positive()
                .any(|l| matches!(&l.c, Constraint::Signer { id } if id == p))
                || self.all_signed_above(p).is_some();
            if signs {
                out.extend(self.classes.class_lits(p));
            }
        }
        out
    }

    /// Some key is forced to be posted under `q`.
    fn nonempty(&self, q: &str) -> bool {
        self.classes.texted_paths().any(|p| key_under(p, q))
            || self.positive().any(|l| match &l.c {
                Constraint::SignerCount { prefix, at_least } => *at_least >= 1 && under(prefix, q),
                Constraint::SignerAll { prefix } => under(prefix, q),
                _ => false,
            })
    }

    /// A lower bound on the distinct signed keys under `q`, with the
    /// literals that give it.
    fn lower_bound(&self, q: &str) -> (u32, Vec<Lit>) {
        let mut best = 0u32;
        let mut why = Vec::new();
        for lit in self.positive() {
            let n = match &lit.c {
                Constraint::SignerCount { prefix, at_least } if under(prefix, q) => *at_least,
                Constraint::Signer { id } if key_under(id, q) => 1,
                Constraint::SignerAll { prefix }
                    if under(prefix, q) || (under(q, prefix) && self.nonempty(q)) =>
                {
                    1
                }
                _ => continue,
            };
            if n > best {
                best = n;
                why = vec![lit.clone()];
            } else if n == best && n > 0 {
                why.push(lit.clone());
            }
        }
        let keys = self.signed_lits(q).len();
        let keys = u32::try_from(keys).unwrap_or(u32::MAX);
        if keys > best {
            best = keys;
            why = self
                .positive()
                .filter(|l| {
                    matches!(
                        l.c,
                        Constraint::Eq { .. }
                            | Constraint::Eq2 { .. }
                            | Constraint::Signer { .. }
                            | Constraint::SignerAll { .. }
                    )
                })
                .cloned()
                .collect();
        }
        (best, why)
    }
}

fn why(mut lits: Vec<Lit>, lit: &Lit) -> Vec<Lit> {
    lits.push(lit.clone());
    lits.sort();
    lits.dedup();
    lits
}

pub fn check(ctx: &Ctx) -> Option<Vec<Lit>> {
    for lit in ctx.lits {
        match (&lit.c, lit.positive) {
            // -card(q) ≥ n at or below the lower bound (n = 0 always).
            (Constraint::SignerCount { prefix, at_least }, false) => {
                let (lb, because) = ctx.lower_bound(prefix);
                if *at_least <= lb {
                    return Some(why(because, lit));
                }
            }
            // No key can be posted under the root.
            (Constraint::SignerCount { prefix, at_least }, true)
                if *at_least >= 1 && no_keys(prefix) =>
            {
                return Some(vec![lit.clone()]);
            }
            (Constraint::SignerAll { prefix }, true) if no_keys(prefix) => {
                return Some(vec![lit.clone()]);
            }
            // -signed(id) for a key posted under an all-signed prefix.
            (Constraint::Signer { id }, false) => {
                if ctx.classes.texted(id) && is_id(id) && normalized(id) {
                    if let Some(all) = ctx.all_signed_above(id) {
                        let mut because: Vec<Lit> = ctx
                            .positive()
                            .filter(|l| forces_text(l, id))
                            .cloned()
                            .collect();
                        because.push(all.clone());
                        return Some(why(because, lit));
                    }
                }
            }
            // -all-signed(p) for a non-empty p under an all-signed prefix.
            (Constraint::SignerAll { prefix: p }, false) => {
                if ctx.nonempty(p) {
                    if let Some(all) = ctx.all_signed_above(p) {
                        return Some(why(vec![all.clone()], lit));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn forces_text(l: &Lit, p: &str) -> bool {
    match &l.c {
        Constraint::Eq { path, .. } | Constraint::Text { path, .. } => path == p,
        Constraint::Signer { id } => id == p,
        Constraint::Eq2 { a, b } => a == p || b == p,
        _ => false,
    }
}
