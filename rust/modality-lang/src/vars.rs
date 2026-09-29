//! Variables over names in label paths.
//!
//! A path segment `$k` (or `$k.id`, `$k.bool`, …) is the variable `k`
//! followed by a suffix. A variable takes a **name**: a nonempty segment
//! with no `/` and no `.`, so `$k.id` is always the name then `.id`. A
//! segment `!$k` (or `!$k.id`) is a **hole**: it stands for every segment
//! whose stem (the part before the first `.`) is not the name `k` takes.
//! `/claimants/!$k` is everything in every slot under `/claimants` but
//! `k`'s: `/claimants/bob`, `/claimants/bob.id`, `/claimants/bob.bool`.
//!
//! - On a model edge the commit picks the names: the edge is taken if some
//!   names make every label hold, where a label with a hole must hold for
//!   every segment the hole can take ([`search`]).
//! - In a rule, a variable ranges over every name. The model checker
//!   decides that by instantiation over the names the model and rule
//!   mention plus fresh ones ([`universe`]); holes are not allowed in rules.
//!
//! A name is only ever compared with the segments of paths, so a name no
//! path has at that position behaves like any other such name, and one
//! fresh name stands for all of them. Lean proves the runtime search exact
//! (`experiments/predicate-theory/lean/PredicateTheory/Vars.lean`,
//! `takesB_iff`); the harness checks this procedure against it.

use crate::ast::{Formula, FormulaExpr, Model, Property, PropertySign, PropertySource};
use std::collections::{BTreeMap, BTreeSet};

/// Variable (and hole) assignments. A hole `!$k` is keyed `!k`.
pub type Env = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seg<'a> {
    Lit(&'a str),
    Var { name: &'a str, suffix: &'a str },
    Hole { of: &'a str, suffix: &'a str },
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// How a path segment reads. A `$` or `!$` segment whose variable is not an
/// identifier reads as a literal; [`check_model`] refuses it.
pub fn seg(s: &str) -> Seg<'_> {
    let (hole, rest) = if let Some(rest) = s.strip_prefix("!$") {
        (true, rest)
    } else if let Some(rest) = s.strip_prefix('$') {
        (false, rest)
    } else {
        return Seg::Lit(s);
    };
    let (name, suffix) = rest.split_at(rest.find('.').unwrap_or(rest.len()));
    if !is_ident(name) {
        return Seg::Lit(s);
    }
    if hole {
        Seg::Hole { of: name, suffix }
    } else {
        Seg::Var { name, suffix }
    }
}

/// A name a variable can take.
pub fn valid_name(n: &str) -> bool {
    valid_seg(n) && !n.contains('.')
}

/// A segment a hole can take.
pub fn valid_seg(s: &str) -> bool {
    !s.is_empty() && !s.contains('/')
}

/// The slot a segment belongs to: `bob.id` is in `bob`.
pub fn stem(s: &str) -> &str {
    s.split('.').next().unwrap_or(s)
}

fn hole_key(of: &str) -> String {
    format!("!{of}")
}

fn args(p: &Property) -> Vec<String> {
    crate::theory::decl::property_args(p)
}

fn segs_of(arg: &str) -> Option<Vec<&str>> {
    arg.strip_prefix('/').map(|rest| rest.split('/').collect())
}

fn has_var_segment(arg: &str) -> bool {
    segs_of(arg).is_some_and(|segs| segs.iter().any(|s| !matches!(seg(s), Seg::Lit(_))))
}

/// The property mentions a variable or a hole.
pub fn has_vars(p: &Property) -> bool {
    args(p).iter().any(|a| has_var_segment(a))
}

pub fn any_vars<'a>(props: impl IntoIterator<Item = &'a Property>) -> bool {
    props.into_iter().any(has_vars)
}

/// Variables the property binds: each `$k`, and the `k` of each `!$k`.
pub fn vars_in(p: &Property) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for a in args(p) {
        for s in segs_of(&a).unwrap_or_default() {
            match seg(s) {
                Seg::Var { name, .. } => {
                    out.insert(name.to_string());
                }
                Seg::Hole { of, .. } => {
                    out.insert(of.to_string());
                }
                Seg::Lit(_) => {}
            }
        }
    }
    out
}

/// Holes in the property, keyed `!k`, with the suffixes each appears with.
fn holes_in(p: &Property) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for a in args(p) {
        for s in segs_of(&a).unwrap_or_default() {
            if let Seg::Hole { of, suffix } = seg(s) {
                out.entry(hole_key(of))
                    .or_default()
                    .insert(suffix.to_string());
            }
        }
    }
    out
}

/// The property has a hole (`!$k`).
pub fn has_holes(p: &Property) -> bool {
    !holes_in(p).is_empty()
}

pub fn vars_of<'a>(props: impl IntoIterator<Item = &'a Property>) -> BTreeSet<String> {
    props.into_iter().flat_map(vars_in).collect()
}

fn map_args(p: &Property, f: impl Fn(&str) -> String) -> Property {
    let Some(PropertySource::Predicate { path, args }) = &p.source else {
        return p.clone();
    };
    let map = |v: &serde_json::Value| match v {
        serde_json::Value::String(s) => serde_json::Value::String(f(s)),
        other => other.clone(),
    };
    let args = if let Some(arg) = args.get("arg") {
        serde_json::json!({ "arg": map(arg) })
    } else if let Some(items) = args.get("args").and_then(|a| a.as_array()) {
        serde_json::json!({ "args": items.iter().map(map).collect::<Vec<_>>() })
    } else if let Some(items) = args.as_array() {
        serde_json::Value::Array(items.iter().map(map).collect())
    } else {
        args.clone()
    };
    Property {
        sign: p.sign.clone(),
        name: p.name.clone(),
        source: Some(PropertySource::Predicate {
            path: path.clone(),
            args,
        }),
    }
}

/// `p` with each `$k` replaced by `env[k]` and each `!$k` by `env[!k]`.
/// Unbound variables stay as written.
pub fn substitute(p: &Property, env: &Env) -> Property {
    if !has_vars(p) {
        return p.clone();
    }
    map_args(p, |arg| {
        let Some(segs) = segs_of(arg) else {
            return arg.to_string();
        };
        let out: Vec<String> = segs
            .iter()
            .map(|s| match seg(s) {
                Seg::Var { name, suffix } => match env.get(name) {
                    Some(n) => format!("{n}{suffix}"),
                    None => s.to_string(),
                },
                Seg::Hole { of, suffix } => match env.get(&hole_key(of)) {
                    Some(n) => format!("{n}{suffix}"),
                    None => s.to_string(),
                },
                Seg::Lit(l) => l.to_string(),
            })
            .collect();
        format!("/{}", out.join("/"))
    })
}

/// Every assignment of `names` to `vars`, in order.
pub fn assignments(vars: &[String], names: &[String]) -> Vec<Env> {
    let mut out = vec![Env::new()];
    for v in vars {
        out = out
            .into_iter()
            .flat_map(|env| {
                names.iter().map(move |n| {
                    let mut e = env.clone();
                    e.insert(v.clone(), n.clone());
                    e
                })
            })
            .collect();
    }
    out
}

/// `count` names not in `taken`: `~0`, `~1`, …
pub fn fresh_names(taken: &BTreeSet<String>, count: usize) -> Vec<String> {
    (0..)
        .map(|i| format!("~{i}"))
        .filter(|n| !taken.contains(n))
        .take(count)
        .collect()
}

// ---------------------------------------------------------------------------
// Model checking: instantiation

/// Literal segments of every path argument.
pub fn segments<'a>(props: impl IntoIterator<Item = &'a Property>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for p in props {
        for a in args(p) {
            for s in segs_of(&a).unwrap_or_default() {
                if let Seg::Lit(l) = seg(s) {
                    if !l.is_empty() {
                        out.insert(l.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Suffixes variables appear with (`.id` for `$k.id`).
pub fn var_suffixes<'a>(props: impl IntoIterator<Item = &'a Property>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for p in props {
        for a in args(p) {
            for s in segs_of(&a).unwrap_or_default() {
                if let Seg::Var { suffix, .. } = seg(s) {
                    out.insert(suffix.to_string());
                }
            }
        }
    }
    out
}

/// Names to instantiate variables with: the stem of every mentioned
/// segment that is a name, then `fresh` distinct fresh names. A name no
/// label mentions behaves like a fresh one, and `fresh` of them cover every
/// way that many variables can be equal or differ.
pub fn universe(segments: &BTreeSet<String>, fresh: usize) -> Vec<String> {
    let names: BTreeSet<String> = segments
        .iter()
        .map(|s| stem(s).to_string())
        .filter(|n| valid_name(n))
        .collect();
    let taken: BTreeSet<String> = names.iter().chain(segments).cloned().collect();
    names
        .into_iter()
        .chain(fresh_names(&taken, fresh))
        .collect()
}

/// Ground instances of `p` under `env`: variables substituted, and each
/// hole filled with every segment in `segments` it can take (one property
/// per filling). A hole's instances beyond `segments` are dropped, which
/// only weakens the edge: a dead verdict or an entailment on the instances
/// holds for the edge.
pub fn expand(p: &Property, env: &Env, segments: &BTreeSet<String>) -> Vec<Property> {
    let holes = holes_in(p);
    if holes.is_empty() {
        return vec![substitute(p, env)];
    }
    let mut fills = vec![env.clone()];
    for (key, suffixes) in &holes {
        let excluded = env.get(&key[1..]).cloned();
        let cands: BTreeSet<String> = segments
            .iter()
            .flat_map(|s| {
                suffixes
                    .iter()
                    .filter_map(move |suf| s.strip_suffix(suf.as_str()))
            })
            .filter(|j| valid_seg(j) && excluded.as_deref() != Some(stem(j)))
            .map(str::to_string)
            .collect();
        fills = fills
            .into_iter()
            .flat_map(|e| {
                cands.iter().map(move |j| {
                    let mut e = e.clone();
                    e.insert(key.clone(), j.clone());
                    e
                })
            })
            .collect();
    }
    fills.iter().map(|e| substitute(p, e)).collect()
}

/// Every edge of the model instantiated: an edge with variables becomes one
/// edge per assignment of `names` to its variables, with holes filled from
/// `segments`. Edges without variables are unchanged.
pub fn ground_model(model: &Model, names: &[String], segments: &BTreeSet<String>) -> Model {
    let ground_edge = |t: &crate::ast::Transition| -> Vec<crate::ast::Transition> {
        if !any_vars(&t.properties) {
            return vec![t.clone()];
        }
        let vars: Vec<String> = vars_of(&t.properties).into_iter().collect();
        assignments(&vars, names)
            .into_iter()
            .map(|env| crate::ast::Transition {
                from: t.from.clone(),
                to: t.to.clone(),
                properties: t
                    .properties
                    .iter()
                    .flat_map(|p| expand(p, &env, segments))
                    .collect(),
            })
            .collect()
    };
    let mut out = model.clone();
    for part in &mut out.parts {
        part.transitions = part.transitions.iter().flat_map(ground_edge).collect();
    }
    out.transitions = model.transitions.iter().flat_map(ground_edge).collect();
    out
}

fn visit_props<'a>(e: &'a FormulaExpr, out: &mut Vec<&'a Property>) {
    use FormulaExpr as F;
    match e {
        F::True | F::False | F::Prop(_) | F::Var(_) => {}
        F::And(a, b) | F::Or(a, b) | F::Implies(a, b) | F::Until(a, b) => {
            visit_props(a, out);
            visit_props(b, out);
        }
        F::Not(a)
        | F::Paren(a)
        | F::Lfp(_, a)
        | F::Gfp(_, a)
        | F::Eventually(a)
        | F::Always(a)
        | F::Next(a) => visit_props(a, out),
        F::Diamond(ps, a) | F::Box(ps, a) | F::DiamondBox(ps, a) => {
            out.extend(ps.iter());
            visit_props(a, out);
        }
    }
}

/// Every property in a formula's modal labels.
pub fn formula_props(f: &FormulaExpr) -> Vec<&Property> {
    let mut out = Vec::new();
    visit_props(f, &mut out);
    out
}

/// The formula with `env` substituted into every label.
pub fn substitute_formula(e: &FormulaExpr, env: &Env) -> FormulaExpr {
    use FormulaExpr as F;
    let s = |x: &FormulaExpr| Box::new(substitute_formula(x, env));
    let ps = |v: &[Property]| v.iter().map(|p| substitute(p, env)).collect();
    match e {
        F::True | F::False | F::Prop(_) | F::Var(_) => e.clone(),
        F::And(a, b) => F::And(s(a), s(b)),
        F::Or(a, b) => F::Or(s(a), s(b)),
        F::Implies(a, b) => F::Implies(s(a), s(b)),
        F::Until(a, b) => F::Until(s(a), s(b)),
        F::Not(a) => F::Not(s(a)),
        F::Paren(a) => F::Paren(s(a)),
        F::Lfp(v, a) => F::Lfp(v.clone(), s(a)),
        F::Gfp(v, a) => F::Gfp(v.clone(), s(a)),
        F::Eventually(a) => F::Eventually(s(a)),
        F::Always(a) => F::Always(s(a)),
        F::Next(a) => F::Next(s(a)),
        F::Diamond(p, a) => F::Diamond(ps(p), s(a)),
        F::Box(p, a) => F::Box(ps(p), s(a)),
        F::DiamondBox(p, a) => F::DiamondBox(ps(p), s(a)),
    }
}

pub fn model_props(model: &Model) -> Vec<&Property> {
    model
        .parts
        .iter()
        .flat_map(|p| p.transitions.iter())
        .chain(model.transitions.iter())
        .flat_map(|t| t.properties.iter())
        .collect()
}

// ---------------------------------------------------------------------------
// Checks

/// Predicates whose path arguments may hold variables: the standard ones
/// the evaluator reads from paths alone.
const PATH_PREDICATES: [&str; 24] = [
    "signed_by",
    "any_signed",
    "all_signed",
    "threshold",
    "modifies",
    "post_to_path",
    "post_to",
    "state_exists",
    "text_eq",
    "text_contains",
    "text_starts_with",
    "text_ends_with",
    "amount_in_range",
    "num_eq",
    "num_gt",
    "num_gte",
    "num_lt",
    "num_lte",
    "bool_true",
    "bool_false",
    "sent_eq",
    "sent_lte",
    "sent_to",
    "posts_own_key",
];

fn allows_vars(name: &str) -> bool {
    PATH_PREDICATES.contains(&name)
}

/// Predicates that read a path's type from its extension. A hole with no
/// suffix there would stand for segments of every type.
fn typed(name: &str) -> bool {
    !matches!(
        name,
        "any_signed" | "all_signed" | "threshold" | "modifies" | "post_to_path" | "state_exists"
    )
}

fn check_property(p: &Property, holes_allowed: bool) -> Result<(), String> {
    for a in args(p) {
        let Some(segs) = segs_of(&a) else { continue };
        for s in segs {
            let parsed = seg(s);
            if matches!(parsed, Seg::Lit(_)) {
                if s.starts_with('$') || s.starts_with("!$") {
                    return Err(format!(
                        "{}({a}): `{s}` is not a variable (`$k`, `$k.id`) or a hole (`!$k`)",
                        p.name
                    ));
                }
                continue;
            }
            if !allows_vars(&p.name) {
                return Err(format!(
                    "{}({a}): variables are only read by the standard path predicates",
                    p.name
                ));
            }
            if let Seg::Hole { suffix, .. } = parsed {
                if !holes_allowed {
                    return Err(format!(
                        "{}({a}): a hole (`!$k`) is for model edges; in a rule a variable already ranges over every name",
                        p.name
                    ));
                }
                if suffix.is_empty() && typed(&p.name) {
                    return Err(format!(
                        "{}({a}): `{s}` needs the type it reads (`{s}.id`, `{s}.bool`, …)",
                        p.name
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Variables and holes in a model's labels are well formed.
pub fn check_model(model: &Model) -> Result<(), String> {
    model_props(model)
        .into_iter()
        .try_for_each(|p| check_property(p, true))
}

/// Variables in a rule are well formed; rules take no holes.
pub fn check_formula(formula: &Formula) -> Result<(), String> {
    formula_props(&formula.expression)
        .into_iter()
        .try_for_each(|p| check_property(p, false))
}

// ---------------------------------------------------------------------------
// Runtime: does a commit take an edge with variables?

/// The paths one commit can be read against, normalised (no leading `/`).
pub struct Paths<'a> {
    pub state: Vec<&'a str>,
    pub body: Vec<&'a str>,
}

/// `modifies`, `post_to_path`, `post_to` and `posts_own_key` read the body;
/// everything else reads accepted state.
fn reads_body(name: &str) -> bool {
    matches!(name, "modifies" | "post_to_path" | "post_to" | "posts_own_key")
}

/// For each variable and hole of `props`: every value that meets a path the
/// predicate reads, at the position it sits. A path segment `e[i]` meets
/// `$k.suf` at `i` when `e[i]` is a name then `suf`, and `!$k.suf` when it
/// ends in `suf`. When everything before `i` is literal, only paths with
/// that prefix count; otherwise any path long enough does. A variable also
/// takes the stem of each value its hole meets. Any other value meets
/// nothing, so it behaves like a fresh name.
fn candidates(
    props: &[Property],
    paths: &Paths,
) -> (
    BTreeMap<String, BTreeSet<String>>,
    BTreeMap<String, BTreeSet<String>>,
) {
    let mut vars: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut holes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for p in props {
        let world = if reads_body(&p.name) {
            &paths.body
        } else {
            &paths.state
        };
        for a in args(p) {
            let Some(segs) = segs_of(&a) else { continue };
            for (i, s) in segs.iter().enumerate() {
                let (key, suffix, is_var) = match seg(s) {
                    Seg::Var { name, suffix } => (name.to_string(), suffix, true),
                    Seg::Hole { of, suffix } => (hole_key(of), suffix, false),
                    Seg::Lit(_) => continue,
                };
                let prefix: Option<Vec<&str>> = segs[..i]
                    .iter()
                    .map(|s| match seg(s) {
                        Seg::Lit(l) => Some(l),
                        _ => None,
                    })
                    .collect();
                let out = if is_var {
                    vars.entry(key).or_default()
                } else {
                    holes.entry(key).or_default()
                };
                for e in world {
                    let e: Vec<&str> = e.split('/').collect();
                    if e.len() <= i || prefix.as_ref().is_some_and(|pre| e[..i] != pre[..]) {
                        continue;
                    }
                    let Some(v) = e[i].strip_suffix(suffix) else {
                        continue;
                    };
                    if (is_var && valid_name(v)) || (!is_var && valid_seg(v)) {
                        out.insert(v.to_string());
                    }
                }
            }
        }
    }
    // A variable also matters where its hole skips a slot.
    for (key, cands) in &holes {
        let stems = cands
            .iter()
            .map(|j| stem(j).to_string())
            .filter(|n| valid_name(n));
        vars.entry(key[1..].to_string()).or_default().extend(stems);
    }
    (vars, holes)
}

/// Longer than every segment of every path, so it meets none.
fn fresh_for(paths: &Paths) -> String {
    let longest = paths
        .state
        .iter()
        .chain(paths.body.iter())
        .flat_map(|p| p.split('/'))
        .map(str::len)
        .max()
        .unwrap_or(0);
    "~".repeat(longest + 1)
}

/// The outcome of [`search`].
#[derive(Debug, Clone, PartialEq)]
pub enum Search {
    /// These names make every label hold.
    Takes(Env),
    /// No names do. The assignment that came closest (fewest failures, then
    /// fewest `+` labels missing), and the instances that fail under it.
    Fails(Env, Vec<Property>),
}

/// Does a commit take an edge with variables? `holds(p)` says whether a
/// ground property holds (sign included). Tries every candidate name, plus
/// one fresh name, for each variable, and every candidate segment plus the
/// fresh name for each hole, skipping a candidate in the excluded slot.
pub fn search(
    props: &[Property],
    paths: &Paths,
    holds: &mut dyn FnMut(&Property) -> bool,
) -> Search {
    let (var_cands, hole_cands) = candidates(props, paths);
    let fresh = fresh_for(paths);
    let vars: Vec<String> = vars_of(props).into_iter().collect();
    let mut best: Option<(Env, Vec<Property>)> = None;
    let names_for = |v: &String| -> Vec<String> {
        let mut ns: Vec<String> = var_cands.get(v).into_iter().flatten().cloned().collect();
        ns.push(fresh.clone());
        ns
    };
    let mut envs = vec![Env::new()];
    for v in &vars {
        let ns = names_for(v);
        envs = envs
            .into_iter()
            .flat_map(|env| {
                ns.iter().map(move |n| {
                    let mut e = env.clone();
                    e.insert(v.clone(), n.clone());
                    e
                })
            })
            .collect();
    }
    for env in envs {
        let mut failures = Vec::new();
        for p in props {
            let holes = holes_in(p);
            let mut fills = vec![env.clone()];
            for key in holes.keys() {
                let excluded = env.get(&key[1..]).cloned();
                let mut cands: Vec<String> = hole_cands
                    .get(key)
                    .into_iter()
                    .flatten()
                    .filter(|j| excluded.as_deref() != Some(stem(j)))
                    .cloned()
                    .collect();
                cands.push(fresh.clone());
                fills = fills
                    .into_iter()
                    .flat_map(|e| {
                        cands.iter().map(move |j| {
                            let mut e = e.clone();
                            e.insert(key.clone(), j.clone());
                            e
                        })
                    })
                    .collect();
            }
            for f in &fills {
                let ground = substitute(p, f);
                if !holds(&ground) {
                    failures.push(ground);
                }
            }
        }
        if failures.is_empty() {
            return Search::Takes(env);
        }
        // Closest: fewest failures, then fewest required labels missing.
        let rank = |f: &[Property]| (f.len(), f.iter().filter(|p| is_positive(p)).count());
        if best.as_ref().is_none_or(|(_, b)| rank(&failures) < rank(b)) {
            best = Some((env, failures));
        }
    }
    let (env, failures) = best.unwrap_or_default();
    Search::Fails(env, failures)
}

/// The property is `+`: it must hold, where `-` must not.
pub fn is_positive(p: &Property) -> bool {
    p.sign == PropertySign::Plus
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, a: &[&str]) -> Property {
        Property::new_predicate_from_call_args(
            name.into(),
            a.iter().map(|s| s.to_string()).collect(),
        )
    }

    #[test]
    fn segments_read_as_variables_holes_or_literals() {
        assert_eq!(
            seg("$k"),
            Seg::Var {
                name: "k",
                suffix: ""
            }
        );
        assert_eq!(
            seg("$k.id"),
            Seg::Var {
                name: "k",
                suffix: ".id"
            }
        );
        assert_eq!(
            seg("!$who.bool"),
            Seg::Hole {
                of: "who",
                suffix: ".bool"
            }
        );
        assert_eq!(seg("alice.id"), Seg::Lit("alice.id"));
        assert_eq!(seg("$1"), Seg::Lit("$1"));
        assert_eq!(stem("bob.id"), "bob");
        assert!(valid_name("bob") && !valid_name("bob.id") && valid_seg("bob.id"));
    }

    #[test]
    fn substitution_fills_variables_and_holes() {
        let mut env = Env::new();
        env.insert("k".into(), "alice".into());
        env.insert("!k".into(), "bob.id".into());
        let got = substitute(&p("signed_by", &["/claimants/$k.id"]), &env);
        assert_eq!(args(&got), vec!["/claimants/alice.id"]);
        let got = substitute(&p("modifies", &["/claimants/!$k"]), &env);
        assert_eq!(args(&got), vec!["/claimants/bob.id"]);
        assert_eq!(
            vars_of(&[p("signed_by", &["/c/$k.id"]), p("modifies", &["/c/!$j"])]),
            ["j", "k"].iter().map(|s| s.to_string()).collect()
        );
    }

    #[test]
    fn holes_expand_over_mentioned_segments_outside_the_excluded_slot() {
        let segs: BTreeSet<String> = ["c", "alice", "alice.id", "bob.id"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut env = Env::new();
        env.insert("k".into(), "alice".into());
        let got: Vec<Vec<String>> = expand(&p("modifies", &["/c/!$k"]), &env, &segs)
            .iter()
            .map(args)
            .collect();
        assert_eq!(got, vec![vec!["/c/bob.id"], vec!["/c/c"]]);
    }

    #[test]
    fn checks_refuse_malformed_or_misplaced_variables() {
        let edge = |props: Vec<Property>| Model {
            name: "M".into(),
            parts: vec![],
            state: None,
            initial: None,
            transitions: vec![crate::ast::Transition {
                from: "a".into(),
                to: "a".into(),
                properties: props,
            }],
        };
        assert!(check_model(&edge(vec![
            p("signed_by", &["/c/$k.id"]),
            p("modifies", &["/c/!$k"])
        ]))
        .is_ok());
        assert!(check_model(&edge(vec![p("signed_by", &["/c/!$k"])])).is_err());
        assert!(check_model(&edge(vec![p("wasm", &["/m.wasm", "/c/$k"])])).is_err());
        assert!(check_model(&edge(vec![p("modifies", &["/c/$1"])])).is_err());
    }

    #[test]
    fn search_finds_the_signer_whose_slot_is_written() {
        let state = ["c/alice.id", "c/bob.id"];
        let signed = "alice_key";
        let keys: BTreeMap<&str, &str> =
            [("c/alice.id", "alice_key"), ("c/bob.id", "bob_key")].into();
        let edge = [
            p("signed_by", &["/c/$k.id"]),
            Property::new_predicate_from_call_args_negated(
                "modifies".into(),
                vec!["/c/!$k".into()],
            ),
        ];
        let run = |body: &[&str]| {
            let paths = Paths {
                state: state.to_vec(),
                body: body.to_vec(),
            };
            let mut holds = |g: &Property| {
                let a = args(g);
                let path = a[0].trim_start_matches('/');
                let v = match g.name.as_str() {
                    "signed_by" => keys.get(path) == Some(&signed),
                    "modifies" => body
                        .iter()
                        .any(|b| *b == path || b.starts_with(&format!("{path}/"))),
                    _ => unreachable!(),
                };
                v == is_positive(g)
            };
            search(&edge, &paths, &mut holds)
        };
        assert!(matches!(run(&["c/alice/claimed.bool"]), Search::Takes(e) if e["k"] == "alice"));
        assert!(matches!(run(&["c/alice.id"]), Search::Takes(_)));
        assert!(matches!(run(&["c/bob/claimed.bool"]), Search::Fails(..)));
        assert!(matches!(run(&["c/bob.id"]), Search::Fails(..)));
    }

    fn checker(model: &str) -> crate::ModelChecker {
        let model = crate::parse_all_models_content_lalrpop(model)
            .unwrap()
            .remove(0);
        crate::ModelChecker::with_theory(model, crate::TheoryVersion::V1, None, None)
    }

    fn holds(checker: &crate::ModelChecker, rule: &str) -> bool {
        let formula = crate::parse_all_formulas_content_lalrpop(&format!("formula r {{ {rule} }}"))
            .unwrap()
            .remove(0);
        checker.check_formula(&formula).is_satisfied
    }

    #[test]
    fn a_variable_edge_is_dead_only_when_every_instance_is() {
        let c = checker(
            r#"
model M {
  part flow {
    q0 --> q1: +bool_true(/c/$k.bool) -bool_true(/c/$k.bool)
    q0 --> q1: +bool_true(/c/$k.bool) -bool_true(/c/alice.bool)
    q1 --> q1: +modifies(/c/$k) -modifies(/c/!$k)
  }
}"#,
        );
        let dead = c.dead_transitions();
        assert_eq!(dead.len(), 1, "{dead:?}");
        assert_eq!(args(&dead[0].properties[0]), vec!["/c/$k.bool"]);
        assert!(!dead[0].offending.is_empty());
        let moves = c.classify_moves("q0");
        assert_eq!(moves.len(), 1, "{moves:?}");
        assert_eq!(moves[0].status, crate::MoveStatus::Open);
    }

    #[test]
    fn a_rule_variable_holds_only_when_every_instance_does() {
        let own = "[] always([+modifies(/c/$k) -signed_by(/c/$k.id)] false)";
        let guarded = checker(
            r#"
model M {
  part flow {
    q0 --> q0: +signed_by(/c/$k.id) -modifies(/c/!$k)
    q0 --> q0: -modifies(/c)
  }
}"#,
        );
        assert!(holds(&guarded, own));
        // A name the model never mentions still gets a slot: the hole
        // covers it, the literal edge does not.
        let literal = checker(
            r#"
model M {
  part flow {
    q0 --> q0: +signed_by(/c/alice.id) -modifies(/c/bob)
    q0 --> q0: -modifies(/c)
  }
}"#,
        );
        assert!(!holds(&literal, own));
        let open = checker(
            r#"
model M {
  part flow {
    q0 --> q0: +signed_by(/c/$k.id)
  }
}"#,
        );
        assert!(!holds(&open, own));
        // The edge's signer is one name; every other name need not sign.
        assert!(!holds(&open, "[] always([-signed_by(/c/$j.id)] false)"));
        // A hole in a rule is not decided.
        assert!(!holds(&guarded, "[] always([+modifies(/c/!$k)] false)"));
    }

    /// `docs/language/model-cookbook.md`, "Any number of claimants".
    #[test]
    fn the_cookbook_claimant_registry_meets_its_rule() {
        let rule = "[] always(([+modifies(/claimants/$k) -signed_by(/claimants/$k.id)] false) & ([+modifies(/claimants/$k.id) +state_exists(/claimants/$k.id) -signed_by(/claimants/$k.id)] false))";
        let model = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: -modifies(/claimants)
    q1 --> q1: -state_exists(/claimants/$k.id) +post_to_path(/claimants/$k.id) -modifies(/claimants/$k) -modifies(/claimants/!$k)
    q1 --> q1: +signed_by(/claimants/$k.id) -modifies(/claimants/!$k)
  }
}"#;
        let c = checker(model);
        assert!(c.dead_transitions().is_empty());
        assert!(holds(&c, rule));
        let loose = model.replace(
            "+signed_by(/claimants/$k.id) -modifies(/claimants/!$k)",
            "+signed_by(/claimants/$k.id)",
        );
        assert!(!holds(&checker(&loose), rule));
    }

    /// Issue 11: grounding over the mentioned names plus one fresh name per
    /// variable is exact only if a name the labels do not mention behaves
    /// like any other. For random models with variables and holes, and
    /// rules with a variable, mentioning three more names (in a conjunct
    /// that is `true`) never changes the verdict: more names for
    /// variables, and more segments to fill holes with.
    /// `MODALITY_GROUNDING_ROUNDS` runs more rounds (3,000 take minutes).
    #[test]
    fn more_names_never_change_a_verdict() {
        const EDGE: &[&str] = &[
            "+POST",
            "+modifies(/c/$k)",
            "-modifies(/c/!$k)",
            "-modifies(/c)",
            "+modifies(/c/alice)",
            "-modifies(/c/alice)",
            "+bool_true(/c/$k.bool)",
            "-bool_true(/c/$k.bool)",
            "+bool_true(/c/alice.bool)",
            "-bool_true(/c/alice.bool)",
            "+signed_by(/c/$k.id)",
            "-signed_by(/c/!$k.id)",
            "+signed_by(/c/$m.id)",
            "+modifies(/c/$m)",
            "+post_to_path(/c/$k/claimed.bool)",
            "-state_exists(/c/$k.bool)",
        ];
        const LABEL: &[&str] = &[
            "+POST",
            "+modifies(/c/$j)",
            "-modifies(/c/$j)",
            "+signed_by(/c/$j.id)",
            "-signed_by(/c/$j.id)",
            "+bool_true(/c/$j.bool)",
            "-bool_true(/c/$j.bool)",
            "+modifies(/c/alice)",
            "+post_to_path(/c/$j/claimed.bool)",
        ];
        let mut x: u64 = 0x11_6A0D_D1CE;
        let mut next = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let pick = |pool: &[&'static str], next: &mut dyn FnMut() -> u64| {
            pool[(next() % pool.len() as u64) as usize]
        };
        let rounds: u32 = std::env::var("MODALITY_GROUNDING_ROUNDS")
            .ok()
            .and_then(|r| r.parse().ok())
            .unwrap_or(150);
        let (mut held, mut failed) = (0, 0);
        for round in 0..rounds {
            let mut edges = String::new();
            for _ in 0..2 + next() % 3 {
                let from = next() % 2;
                let to = next() % 2;
                let labels: Vec<&str> = (0..1 + next() % 3).map(|_| pick(EDGE, &mut next)).collect();
                edges.push_str(&format!("    q{from} --> q{to}: {}\n", labels.join(" ")));
            }
            let model = format!("model M {{\n  part p {{\n{edges}  }}\n}}");
            let label = |next: &mut dyn FnMut() -> u64| {
                (0..1 + next() % 2)
                    .map(|_| LABEL[(next() % LABEL.len() as u64) as usize])
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let (a, b) = (label(&mut next), label(&mut next));
            let rule = match next() % 7 {
                0 => format!("[{a}] false"),
                1 => format!("<{a}> true"),
                2 => format!("[{a}] <{b}> true"),
                3 => format!("always([{a}] false)"),
                4 => format!("always(<{a}> true)"),
                5 => format!("[{a}] [{b}] false"),
                _ => format!("eventually(<{a}> true)"),
            };
            let Ok(mut models) = crate::parse_all_models_content_lalrpop(&model) else {
                continue;
            };
            let m = models.remove(0);
            if check_model(&m).is_err() {
                continue;
            }
            let formula = |text: &str| {
                crate::parse_all_formulas_content_lalrpop(&format!("formula r {{ {text} }}"))
                    .unwrap()
                    .remove(0)
            };
            let plain = formula(&rule);
            let wider = formula(&format!("({rule}) & (<+modifies(/x1/x2/x3)> true | true)"));
            for version in [crate::TheoryVersion::V1, crate::TheoryVersion::V2] {
                let c = crate::ModelChecker::with_theory(m.clone(), version, None, None);
                let verdict = c.check_formula(&plain).is_satisfied;
                assert_eq!(
                    verdict,
                    c.check_formula(&wider).is_satisfied,
                    "round {round} {version:?}: more names change the verdict of {rule} on\n{model}"
                );
                if verdict {
                    held += 1;
                } else {
                    failed += 1;
                }
            }
        }
        assert!(
            held > rounds / 6 && failed > rounds / 6,
            "held {held}, failed {failed}"
        );
    }
}
