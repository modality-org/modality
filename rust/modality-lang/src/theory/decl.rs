//! Declarations: what a predicate means, in the fragment, in two directions.
//!
//! `necessary` — constraints that hold whenever the predicate holds. Feeds
//! `consistent` (dead edges, blocked moves).
//! `sufficient` — constraints that guarantee the predicate holds. Feeds
//! `entails` when the predicate is the goal.
//!
//! A predicate with no declaration is opaque. Standard predicates are
//! declarations shipped as data (`standard.rs`); custom `+wasm(...)`
//! predicates commit theirs next to the module.

use super::fragment::Template;
use super::sort::{Constraint, Lit};
use crate::ast::{Property, PropertySource};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub necessary: Option<Template>,
    pub sufficient: Option<Template>,
}

impl Declaration {
    /// Parse both directions from fragment text. `None` text means "not
    /// declared in that direction". A direction that fails to parse is
    /// dropped (the predicate is weaker, never wrong).
    pub fn parse(necessary: Option<&str>, sufficient: Option<&str>) -> Self {
        Self {
            necessary: necessary.and_then(Template::parse),
            sufficient: sufficient.and_then(Template::parse),
        }
    }

    /// Exact: one template serves both directions.
    pub fn exact(src: &str) -> Self {
        let t = Template::parse(src);
        Self {
            necessary: t.clone(),
            sufficient: t,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.necessary.is_none() && self.sufficient.is_none()
    }
}

/// Where a predicate's declaration comes from.
pub trait Registry {
    /// Declaration for a predicate name, or for a custom module path when the
    /// predicate is `wasm(<module>, args...)`.
    fn declaration(&self, key: &str) -> Option<&Declaration>;
}

/// Arguments of a predicate property as plain strings, in order.
pub fn property_args(p: &Property) -> Vec<String> {
    match &p.source {
        Some(PropertySource::Predicate { args, .. }) => {
            if let Some(a) = args.get("arg") {
                return vec![json_text(a)];
            }
            args.get("args")
                .and_then(|v| v.as_array())
                .map(|items| items.iter().map(json_text).collect())
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn json_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Registry key and the arguments the template sees, for a property.
/// `wasm(/m.wasm, a, b)` → key `/m.wasm`, args `[a, b]`.
pub fn registry_key_and_args(p: &Property) -> (String, Vec<String>) {
    let args = property_args(p);
    if p.name == "wasm" {
        if let Some((module, rest)) = args.split_first() {
            return (module.clone(), rest.to_vec());
        }
    }
    (p.name.clone(), args)
}

/// Result of elaborating one property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    pub lits: Vec<Lit>,
    /// Every literal is decidable and the expansion is exact in the
    /// direction used, so a `consistent = True` over these is a real model.
    pub exact: bool,
}

/// Elaborate a property through the registry.
///
/// Positive: the `necessary` constraints, all as positive literals.
/// Negative: if `sufficient` is a single atom, its negation as one literal
/// (¬P ⇒ ¬sufficient). A multi-atom sufficient negates to a disjunction the
/// literal language cannot express, so the property stays opaque.
/// No usable declaration: one `Opaque` literal carrying the name and args.
pub fn expand(reg: &dyn Registry, p: &Property) -> Expansion {
    let (key, args) = registry_key_and_args(p);
    let positive = p.sign == crate::ast::PropertySign::Plus;
    let opaque = || Expansion {
        lits: vec![Lit {
            c: Constraint::Opaque {
                name: key.clone(),
                args: args.clone(),
            },
            positive,
        }],
        exact: false,
    };

    // Static labels (`+APPROVE`) have no declaration by construction.
    if p.is_static() {
        return opaque();
    }
    let Some(decl) = reg.declaration(&key) else {
        return opaque();
    };

    if positive {
        let Some(t) = &decl.necessary else {
            return opaque();
        };
        let Some(cs) = t.instantiate(&args) else {
            return opaque();
        };
        let lits: Vec<Lit> = cs.into_iter().map(Lit::pos).collect();
        let exact = decl.sufficient == decl.necessary && lits.iter().all(Lit::decidable);
        Expansion { lits, exact }
    } else {
        let Some(t) = &decl.sufficient else {
            return opaque();
        };
        if t.len() != 1 {
            return opaque();
        }
        let Some(cs) = t.instantiate(&args) else {
            return opaque();
        };
        let lits: Vec<Lit> = cs.into_iter().map(Lit::neg).collect();
        let exact = decl.sufficient == decl.necessary && lits.iter().all(Lit::decidable);
        Expansion { lits, exact }
    }
}

/// The registry a contract sees: the standard set plus its own committed
/// declarations, keyed by module path. A committed declaration for a
/// standard name does not override the standard one.
#[derive(Default)]
pub struct ContractRegistry {
    custom: BTreeMap<String, Declaration>,
}

impl ContractRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom declaration. Empty (unparsable) declarations are kept so
    /// callers can report `declaration-unparsed`; they behave as opaque.
    pub fn declare(&mut self, module_path: &str, decl: Declaration) {
        self.custom.insert(module_path.to_string(), decl);
    }

    /// Module paths whose committed declaration failed to parse entirely.
    pub fn unparsed(&self) -> Vec<String> {
        self.custom
            .iter()
            .filter(|(_, d)| d.is_empty())
            .map(|(k, _)| k.clone())
            .collect()
    }
}

impl Registry for ContractRegistry {
    fn declaration(&self, key: &str) -> Option<&Declaration> {
        if let Some(d) = super::standard::standard().declaration(key) {
            return Some(d);
        }
        self.custom.get(key).filter(|d| !d.is_empty())
    }
}
