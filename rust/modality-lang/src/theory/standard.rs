//! Declarations of the standard predicates, as data in the fragment.
//!
//! `num_gt` is not special: it is a predicate whose declaration is
//! `(> $1 $2)`. No verdict anywhere in the theory depends on matching a
//! predicate name; that is what case J9 checks.

use super::decl::{Declaration, Registry};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// `(name, signature, necessary, sufficient)`. `sufficient == None` means
/// exact (the same template both ways); `Some("")` means necessary-only.
///
/// The signature is what the evaluator (`predicate_holds`) reads at each
/// position: e.g. `num_*` reads its first argument from state only, and
/// `text_eq` reads a string, so `text_eq(/x.num, "5")` is not numeric
/// equality. With arguments outside the signature the predicate never holds.
const TABLE: &[(&str, &str, &str, Option<&str>)] = &[
    // numeric
    ("num_gt", "num-path num", "(> $1 $2)", None),
    ("num_gte", "num-path num", "(>= $1 $2)", None),
    ("num_lt", "num-path num", "(< $1 $2)", None),
    ("num_lte", "num-path num", "(<= $1 $2)", None),
    ("num_eq", "num-path num", "(= $1 $2)", None),
    // necessary is the conjunction; sufficient is the same conjunction, but
    // negation of a two-atom sufficient is a disjunction, so `-amount_in_range`
    // stays opaque by construction (see decl::expand).
    (
        "amount_in_range",
        "num-path num num",
        "(and (<= $2 $1) (<= $1 $3))",
        None,
    ),
    // signatures. `all-signed` means "non-empty and every key signed", and
    // `signed` implies the key exists; the per-sort procedures know both, so
    // one atom is exact.
    ("signed_by", "id-path", "(signed $1)", None),
    ("any_signed", "path", "(>= (card $1) 1)", None),
    ("all_signed", "path", "(all-signed $1)", None),
    ("threshold", "nat path", "(>= (card $2) $1)", None),
    // pending body. `posts` implies `writes` in the path lattice.
    ("modifies", "path", "(writes $1)", None),
    ("post_to_path", "path", "(posts $1)", None),
    // `sets(path, value)` / `post_to`: every post to `path` writes `value`.
    // The value is not modelled, so the declaration is necessary-only, and
    // two `sets` of one path to different values are not known to clash.
    ("post_to", "path", "(posts $1)", Some("")),
    // literals. Every value constraint implies existence in the procedure.
    ("text_eq", "text-path text", "(= $1 $2)", None),
    ("bool_true", "bool-path", "$1", None),
    ("bool_false", "bool-path", "(not $1)", None),
    ("state_exists", "path", "(exists $1)", None),
    ("has_property", "path any", "(exists $1)", Some("")),
    (
        "text_contains",
        "text-path needle",
        "(contains $1 $2)",
        None,
    ),
    (
        "text_starts_with",
        "text-path needle",
        "(starts-with $1 $2)",
        None,
    ),
    (
        "text_ends_with",
        "text-path needle",
        "(ends-with $1 $2)",
        None,
    ),
    // `oracle_attests`, `timestamp_valid`, `before`, `after`, hashes, custom
    // `wasm`: no row, therefore opaque here. Governance's registry declares
    // the ones its evaluator never reads as never holding.
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
        for (name, signature, necessary, sufficient) in TABLE {
            let decl = match sufficient {
                None => Declaration::exact(necessary),
                Some("") => Declaration::parse(Some(necessary), None),
                Some(s) => Declaration::parse(Some(necessary), Some(s)),
            };
            decls.insert(name.to_string(), decl.with_params(signature));
        }
        StandardRegistry { decls }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_parses_in_the_direction_it_declares() {
        for (name, signature, necessary, sufficient) in TABLE {
            let d = standard().declaration(name).expect(name);
            assert!(
                d.params.is_some(),
                "{name}: signature failed to parse: {signature}"
            );
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
