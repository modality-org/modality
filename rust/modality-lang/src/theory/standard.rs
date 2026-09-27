//! Declarations of the standard predicates, as data in the fragment.
//!
//! `num_gt` is not special: it is a predicate whose declaration is
//! `(> $1 $2)`. No verdict anywhere in the theory depends on matching a
//! predicate name; that is what case J9 checks.

use super::decl::{Declaration, Registry};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// `(name, necessary, sufficient)`. `sufficient == necessary` means exact.
const TABLE: &[(&str, &str, Option<&str>)] = &[
    // numeric
    ("num_gt", "(> $1 $2)", None),
    ("num_gte", "(>= $1 $2)", None),
    ("num_lt", "(< $1 $2)", None),
    ("num_lte", "(<= $1 $2)", None),
    ("num_eq", "(= $1 $2)", None),
    // necessary is the conjunction; sufficient is the same conjunction, but
    // negation of a two-atom sufficient is a disjunction, so `-amount_in_range`
    // stays opaque by construction (see decl::expand).
    ("amount_in_range", "(and (<= $2 $1) (<= $1 $3))", None),
    // signatures. `all-signed` means "non-empty and every key signed", and
    // `signed` implies the key exists; the per-sort procedures know both, so
    // one atom is exact.
    ("signed_by", "(signed $1)", None),
    ("any_signed", "(>= (card $1) 1)", None),
    ("all_signed", "(all-signed $1)", None),
    ("threshold", "(>= (card $2) $1)", None),
    // pending body. `posts` implies `writes` in the path lattice.
    ("modifies", "(writes $1)", None),
    ("post_to_path", "(posts $1)", None),
    // `sets(path, value)` / `post_to`: the value is not modelled, so the
    // declaration is necessary-only.
    ("post_to", "(posts $1)", Some("")),
    // literals. Every value constraint implies existence in the procedure.
    ("text_eq", "(= $1 $2)", None),
    ("bool_true", "$1", None),
    ("bool_false", "(not $1)", None),
    ("state_exists", "(exists $1)", None),
    ("has_property", "(exists $1)", Some("")),
    ("text_contains", "(contains $1 $2)", None),
    ("text_starts_with", "(starts-with $1 $2)", None),
    ("text_ends_with", "(ends-with $1 $2)", None),
    // `oracle_attests`, `timestamp_valid`, `before`, `after`, hashes, custom
    // `wasm`: no row, therefore opaque.
];

pub struct StandardRegistry {
    decls: BTreeMap<String, Declaration>,
}

impl Registry for StandardRegistry {
    fn declaration(&self, key: &str) -> Option<&Declaration> {
        self.decls.get(key)
    }
}

/// The standard registry, built once.
pub fn standard() -> &'static StandardRegistry {
    static REG: OnceLock<StandardRegistry> = OnceLock::new();
    REG.get_or_init(|| {
        let mut decls = BTreeMap::new();
        for (name, necessary, sufficient) in TABLE {
            let decl = match sufficient {
                None => Declaration::exact(necessary),
                Some("") => Declaration::parse(Some(necessary), None),
                Some(s) => Declaration::parse(Some(necessary), Some(s)),
            };
            decls.insert(name.to_string(), decl);
        }
        StandardRegistry { decls }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_parses_in_the_direction_it_declares() {
        for (name, necessary, sufficient) in TABLE {
            let d = standard().declaration(name).expect(name);
            assert!(
                d.necessary.is_some(),
                "{name}: necessary failed to parse: {necessary}"
            );
            match sufficient {
                None => assert_eq!(d.sufficient, d.necessary, "{name}: should be exact"),
                Some("") => assert!(d.sufficient.is_none(), "{name}"),
                Some(s) => assert!(d.sufficient.is_some(), "{name}: sufficient failed: {s}"),
            }
        }
    }
}
