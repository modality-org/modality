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
use super::rational::Rational;
use super::sort::{ext, norm_path, Constraint, Lit};
use crate::ast::{Property, PropertySource};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// What the evaluator reads at one argument position. A predicate whose
/// arguments do not fit its signature never holds (from `V1`, the evaluator
/// returns false for it too).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    /// `/path`, any extension.
    Path,
    /// `/path.num`.
    NumPath,
    /// `/path.text` or `/path.id`.
    TextPath,
    /// `/path.bool`.
    BoolPath,
    /// `/path.id`.
    IdPath,
    /// `/path.num` or a decimal literal.
    Num,
    /// `/path.text`, `/path.id`, or a literal not starting with `/`.
    Text,
    /// A literal, read as-is, never as a path. One starting with `/` fits
    /// but does not elaborate, so the predicate stays opaque.
    Needle,
    /// A natural-number literal.
    Nat,
    /// Anything.
    Any,
}

impl Param {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "path" => Param::Path,
            "num-path" => Param::NumPath,
            "text-path" => Param::TextPath,
            "bool-path" => Param::BoolPath,
            "id-path" => Param::IdPath,
            "num" => Param::Num,
            "text" => Param::Text,
            "needle" => Param::Needle,
            "nat" => Param::Nat,
            "any" => Param::Any,
            _ => return None,
        })
    }

    pub fn fits(self, arg: &str) -> bool {
        let path_ext = |allowed: &[&str]| {
            arg.starts_with('/') && ext(&norm_path(arg)).is_some_and(|e| allowed.contains(&e))
        };
        match self {
            Param::Path => arg.starts_with('/'),
            Param::NumPath => path_ext(&["num"]),
            Param::TextPath => path_ext(&["text", "id"]),
            Param::BoolPath => path_ext(&["bool"]),
            Param::IdPath => path_ext(&["id"]),
            Param::Num => path_ext(&["num"]) || (!arg.starts_with('/') && is_decimal(arg)),
            Param::Text => path_ext(&["text", "id"]) || !arg.starts_with('/'),
            Param::Needle => true,
            Param::Nat => arg.parse::<u32>().is_ok(),
            Param::Any => true,
        }
    }
}

/// Decimal syntax, `[+-]digits[.digits]`, at any length. A decimal outside
/// the exact domain still fits; its template fails to instantiate and the
/// predicate stays opaque. Lean: `isDecimal`.
fn is_decimal(arg: &str) -> bool {
    let body = match arg.strip_prefix('-') {
        Some(rest) => rest,
        None => arg.strip_prefix('+').unwrap_or(arg),
    };
    let (int_part, frac_part) = body.split_once('.').unwrap_or((body, ""));
    !(int_part.is_empty() && frac_part.is_empty())
        && int_part.chars().all(|c| c.is_ascii_digit())
        && frac_part.chars().all(|c| c.is_ascii_digit())
}

/// `0 < 0`: no world satisfies it. A declared predicate used with an
/// argument of the wrong kind expands to it. Lean: `neverAtom`.
fn never_constraint() -> Constraint {
    static NEVER: OnceLock<Constraint> = OnceLock::new();
    NEVER
        .get_or_init(|| {
            Template::parse("(< 0 0)")
                .and_then(|t| t.instantiate(&[]))
                .and_then(|cs| cs.into_iter().next())
                .expect("(< 0 0) elaborates")
        })
        .clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub necessary: Option<Template>,
    pub sufficient: Option<Template>,
    /// Argument kinds, when the declaration is only valid for some. `None`
    /// means every argument is accepted as the fragment types it.
    pub params: Option<Vec<Param>>,
}

impl Declaration {
    /// Parse both directions from fragment text. `None` text means "not
    /// declared in that direction". A direction that fails to parse is
    /// dropped (the predicate is weaker, never wrong).
    pub fn parse(necessary: Option<&str>, sufficient: Option<&str>) -> Self {
        Self {
            necessary: necessary.and_then(Template::parse),
            sufficient: sufficient.and_then(Template::parse),
            params: None,
        }
    }

    /// Exact: one template serves both directions.
    pub fn exact(src: &str) -> Self {
        let t = Template::parse(src);
        Self {
            necessary: t.clone(),
            sufficient: t,
            params: None,
        }
    }

    /// Restrict to arguments of these kinds (space-separated, e.g.
    /// `"num-path num"`). An unknown kind drops the whole declaration.
    pub fn with_params(mut self, signature: &str) -> Self {
        match signature
            .split_whitespace()
            .map(Param::parse)
            .collect::<Option<Vec<_>>>()
        {
            Some(params) => self.params = Some(params),
            None => {
                self.necessary = None;
                self.sufficient = None;
            }
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.necessary.is_none() && self.sufficient.is_none()
    }

    /// Does the declaration describe the evaluator on these arguments?
    pub fn accepts(&self, args: &[String]) -> bool {
        match &self.params {
            None => true,
            Some(params) => params
                .iter()
                .enumerate()
                .all(|(i, p)| args.get(i).is_some_and(|a| p.fits(a))),
        }
    }
}

/// Where a predicate's declaration comes from.
pub trait Registry {
    /// Declaration for a predicate name, or for a custom module path when the
    /// predicate is `wasm(<module>, args...)`.
    fn declaration(&self, key: &str) -> Option<&Declaration>;

    /// An undeclared predicate whose truth is evidence the commit carries
    /// or leaves out, independent of every other atom (`oracle_attests` on
    /// a validator). The theory treats it as a free boolean.
    fn external(&self, _key: &str) -> bool {
        false
    }
}

/// Arguments of a predicate property as plain strings, in order. Same
/// extraction as the evaluator: `{"arg": a}`, `{"args": [..]}`, or a bare
/// array; strings, numbers, and booleans only (anything else is dropped).
pub fn property_args(p: &Property) -> Vec<String> {
    match &p.source {
        Some(PropertySource::Predicate { args, .. }) => {
            arg_values(args).into_iter().filter_map(arg_text).collect()
        }
        _ => Vec::new(),
    }
}

fn arg_values(args: &Value) -> Vec<&Value> {
    if let Some(arg) = args.get("arg") {
        return vec![arg];
    }
    args.get("args")
        .and_then(Value::as_array)
        .or_else(|| args.as_array())
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn arg_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
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
    /// The literals say exactly what the property says (its declaration is
    /// exact, or it is a static label), so a witness for them is a commit
    /// the evaluator accepts. Lean: `Expansion.exact`.
    pub exact: bool,
}

/// Elaborate a property through the registry.
///
/// Positive: the `necessary` constraints, all as positive literals.
/// Negative: if `sufficient` is a single atom, its negation as one literal
/// (¬P ⇒ ¬sufficient). A multi-atom sufficient negates to a disjunction the
/// literal language cannot express, so the property stays opaque.
/// Arguments that do not fit the declaration's signature: one exact
/// literal over `0 < 0`.
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

    // Static labels (`+APPROVE`) have no declaration by construction; the
    // label literal is the whole meaning (some action has that method).
    if p.is_static() {
        return Expansion {
            lits: vec![Lit {
                c: Constraint::Label {
                    name: p.name.clone(),
                },
                positive,
            }],
            exact: true,
        };
    }
    let Some(decl) = reg.declaration(&key) else {
        if reg.external(&key) {
            return Expansion {
                exact: true,
                ..opaque()
            };
        }
        return opaque();
    };
    // The evaluator reads a wrong-kind argument as false, so the predicate
    // never holds: `+p` kills its edge and `-p` holds on every commit.
    if !decl.accepts(&args) {
        return Expansion {
            lits: vec![Lit {
                c: never_constraint(),
                positive,
            }],
            exact: true,
        };
    }

    if positive {
        let Some(t) = &decl.necessary else {
            return opaque();
        };
        let Some(cs) = t.instantiate(&args) else {
            return opaque();
        };
        let lits: Vec<Lit> = cs.into_iter().map(Lit::pos).collect();
        let exact = decl.sufficient == decl.necessary;
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
        let exact = decl.sufficient == decl.necessary;
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
