//! The V1 fragment: s-expression templates that elaborate, typed by path
//! extension, into `Constraint`s.
//!
//! ```text
//! term := $n | /path | decimal
//! atom := (< term term) | (<= term term) | (= term term) | (> term term) | (>= term term)
//!       | (= path "lit") | (= path path)                 ; .text / .id
//!       | (contains path "lit") | (starts-with path "lit") | (ends-with path "lit")
//!       | path | (not path)                             ; .bool
//!       | (exists path) | (signed path)
//!       | (>= (card prefix) n) | (all-signed prefix)
//!       | (writes path) | (posts path)
//! decl := atom | (and atom ...)
//! ```
//!
//! Anything else fails to parse or to type, and the declaring predicate
//! stays opaque. The Rust here is a transliteration of `elab` in the Lean
//! spec; change the Lean first.

use super::rational::Rational;
use super::sort::{ext, norm_path, Constraint, Op, Term, TextOp};

#[derive(Debug, Clone, PartialEq, Eq)]
enum SExpr {
    Atom(String),
    Str(String),
    List(Vec<SExpr>),
}

fn tokenize(src: &str) -> Option<Vec<SExpr>> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut stack: Vec<Vec<SExpr>> = vec![Vec::new()];
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '(' => {
                stack.push(Vec::new());
                i += 1;
            }
            ')' => {
                let list = stack.pop()?;
                stack.last_mut()?.push(SExpr::List(list));
                i += 1;
            }
            '"' => {
                let mut j = i + 1;
                let mut s = String::new();
                while j < chars.len() && chars[j] != '"' {
                    s.push(chars[j]);
                    j += 1;
                }
                if j >= chars.len() {
                    return None;
                }
                stack.last_mut()?.push(SExpr::Str(s));
                i = j + 1;
            }
            _ => {
                let mut j = i;
                let mut s = String::new();
                while j < chars.len()
                    && !chars[j].is_whitespace()
                    && chars[j] != '('
                    && chars[j] != ')'
                    && chars[j] != '"'
                {
                    s.push(chars[j]);
                    j += 1;
                }
                stack.last_mut()?.push(SExpr::Atom(s));
                i = j;
            }
        }
    }
    if stack.len() != 1 {
        return None;
    }
    stack.pop()
}

/// A parsed, not yet instantiated, declaration body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    atoms: Vec<SExpr>,
}

impl Template {
    /// Parse `decl` text. `None` if it is not in the fragment's grammar.
    pub fn parse(src: &str) -> Option<Self> {
        let top = tokenize(src)?;
        if top.len() != 1 {
            return None;
        }
        let atoms = match top.into_iter().next()? {
            SExpr::List(items) if matches!(items.first(), Some(SExpr::Atom(a)) if a == "and") => {
                items.into_iter().skip(1).collect()
            }
            other => vec![other],
        };
        if atoms.is_empty() || !atoms.iter().all(in_grammar) {
            return None;
        }
        Some(Self { atoms })
    }

    /// Number of atoms; a single-atom template can be negated as one literal.
    pub fn len(&self) -> usize {
        self.atoms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }

    /// Instantiate with the predicate's arguments (`$1` is `args[0]`).
    /// `None` if any atom fails to elaborate or to type.
    pub fn instantiate(&self, args: &[String]) -> Option<Vec<Constraint>> {
        self.atoms
            .iter()
            .map(|a| elab_atom(a, args))
            .collect::<Option<Vec<_>>>()
    }
}

/// A resolved argument: a path (normalised) or a literal string.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Arg {
    Path(String),
    Lit(String),
}

fn resolve(e: &SExpr, args: &[String]) -> Option<Arg> {
    match e {
        SExpr::Str(s) => Some(Arg::Lit(s.clone())),
        SExpr::Atom(a) => {
            if let Some(n) = a.strip_prefix('$') {
                let idx: usize = n.parse().ok()?;
                let v = args.get(idx.checked_sub(1)?)?;
                Some(if v.starts_with('/') {
                    Arg::Path(norm_path(v))
                } else {
                    Arg::Lit(v.clone())
                })
            } else if a.starts_with('/') {
                Some(Arg::Path(norm_path(a)))
            } else {
                Some(Arg::Lit(a.clone()))
            }
        }
        SExpr::List(_) => None,
    }
}

fn num_term(e: &SExpr, args: &[String]) -> Option<Term> {
    match resolve(e, args)? {
        Arg::Path(p) => (ext(&p) == Some("num")).then_some(Term::Path(p)),
        Arg::Lit(l) => Rational::parse(&l).map(Term::Const),
    }
}

fn typed_path(e: &SExpr, args: &[String], allowed: &[&str]) -> Option<String> {
    match resolve(e, args)? {
        Arg::Path(p) => {
            let x = ext(&p)?;
            allowed.contains(&x).then_some(p)
        }
        Arg::Lit(_) => None,
    }
}

fn any_path(e: &SExpr, args: &[String]) -> Option<String> {
    match resolve(e, args)? {
        Arg::Path(p) => Some(p),
        Arg::Lit(_) => None,
    }
}

fn lit(e: &SExpr, args: &[String]) -> Option<String> {
    match resolve(e, args)? {
        Arg::Lit(l) => Some(l),
        Arg::Path(_) => None,
    }
}

fn order(lhs: Term, op: Op, rhs: Term) -> Constraint {
    Constraint::Order { lhs, op, rhs }
}

/// Is this atom in the V1 fragment's grammar? Heads and arities only;
/// typing happens at instantiation, when the arguments are known.
fn in_grammar(e: &SExpr) -> bool {
    fn is_term(e: &SExpr) -> bool {
        matches!(e, SExpr::Atom(_) | SExpr::Str(_))
    }
    match e {
        SExpr::Atom(_) => true,
        SExpr::Str(_) => false,
        SExpr::List(items) => {
            let Some(SExpr::Atom(head)) = items.first() else {
                return false;
            };
            let rest = &items[1..];
            match (head.as_str(), rest.len()) {
                ("<" | "<=" | ">" | "=", 2) => rest.iter().all(is_term),
                (">=", 2) => match &rest[0] {
                    SExpr::List(card) => {
                        card.len() == 2
                            && card[0] == SExpr::Atom("card".into())
                            && is_term(&card[1])
                            && is_term(&rest[1])
                    }
                    other => is_term(other) && is_term(&rest[1]),
                },
                ("contains" | "starts-with" | "ends-with", 2) => rest.iter().all(is_term),
                ("not" | "exists" | "signed" | "all-signed" | "writes" | "posts", 1) => {
                    is_term(&rest[0])
                }
                _ => false,
            }
        }
    }
}

fn elab_atom(e: &SExpr, args: &[String]) -> Option<Constraint> {
    match e {
        // bare .bool path
        SExpr::Atom(_) => {
            let p = typed_path(e, args, &["bool"])?;
            Some(Constraint::Is {
                path: p,
                value: true,
            })
        }
        SExpr::Str(_) => None,
        SExpr::List(items) => {
            let head = match items.first()? {
                SExpr::Atom(h) => h.as_str(),
                _ => return None,
            };
            let rest = &items[1..];
            match (head, rest.len()) {
                ("<", 2) => Some(order(
                    num_term(&rest[0], args)?,
                    Op::Lt,
                    num_term(&rest[1], args)?,
                )),
                ("<=", 2) => Some(order(
                    num_term(&rest[0], args)?,
                    Op::Le,
                    num_term(&rest[1], args)?,
                )),
                (">", 2) => Some(order(
                    num_term(&rest[1], args)?,
                    Op::Lt,
                    num_term(&rest[0], args)?,
                )),
                (">=", 2) => {
                    // (>= (card prefix) n)
                    if let SExpr::List(card) = &rest[0] {
                        if card.len() == 2 && card[0] == SExpr::Atom("card".into()) {
                            let prefix = any_path(&card[1], args)?;
                            let n: u32 = lit(&rest[1], args)?.parse().ok()?;
                            return Some(Constraint::SignerCount {
                                prefix,
                                at_least: n,
                            });
                        }
                        return None;
                    }
                    Some(order(
                        num_term(&rest[1], args)?,
                        Op::Le,
                        num_term(&rest[0], args)?,
                    ))
                }
                ("=", 2) => {
                    // numeric equality if both sides type as .num / decimal
                    if let (Some(l), Some(r)) = (num_term(&rest[0], args), num_term(&rest[1], args))
                    {
                        return Some(order(l, Op::Eq, r));
                    }
                    let a = typed_path(&rest[0], args, &["text", "id"])?;
                    match resolve(&rest[1], args)? {
                        Arg::Lit(l) => Some(Constraint::Eq { path: a, lit: l }),
                        Arg::Path(b) => {
                            let bx = ext(&b)?;
                            (bx == "text" || bx == "id").then_some(Constraint::Eq2 { a, b })
                        }
                    }
                }
                ("contains", 2) | ("starts-with", 2) | ("ends-with", 2) => {
                    let path = typed_path(&rest[0], args, &["text"])?;
                    let needle = lit(&rest[1], args)?;
                    let op = match head {
                        "contains" => TextOp::Contains,
                        "starts-with" => TextOp::StartsWith,
                        _ => TextOp::EndsWith,
                    };
                    Some(Constraint::Text { path, op, needle })
                }
                ("not", 1) => {
                    let p = typed_path(&rest[0], args, &["bool"])?;
                    Some(Constraint::Is {
                        path: p,
                        value: false,
                    })
                }
                ("exists", 1) => Some(Constraint::Exists {
                    path: any_path(&rest[0], args)?,
                }),
                ("signed", 1) => Some(Constraint::Signer {
                    id: typed_path(&rest[0], args, &["id"])?,
                }),
                ("all-signed", 1) => Some(Constraint::SignerAll {
                    prefix: any_path(&rest[0], args)?,
                }),
                ("writes", 1) => Some(Constraint::Writes {
                    path: any_path(&rest[0], args)?,
                }),
                ("posts", 1) => Some(Constraint::Posts {
                    path: any_path(&rest[0], args)?,
                }),
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str, args: &[&str]) -> Option<Constraint> {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        Template::parse(src)?.instantiate(&args)?.into_iter().next()
    }

    #[test]
    fn elaborates_order_with_placeholders() {
        let c = one("(> $1 $2)", &["/x.num", "5"]).unwrap();
        assert_eq!(
            c,
            Constraint::Order {
                lhs: Term::Const(Rational::from_int(5)),
                op: Op::Lt,
                rhs: Term::Path("x.num".into()),
            }
        );
    }

    #[test]
    fn rejects_ill_typed_and_out_of_fragment() {
        assert!(one("(> $1 \"5\")", &["/p.text"]).is_none()); // J11
        assert!(one("(odd $1)", &["/p.num"]).is_none()); // J7
        assert!(one("(>= (* $1 $2) $3)", &["/x.num", "/y.num", "100"]).is_none()); // J10
        assert!(one("$1", &["/p.num"]).is_none()); // bare path must be .bool
    }

    #[test]
    fn card_and_signers() {
        assert_eq!(
            one("(>= (card $2) $1)", &["3", "/m"]).unwrap(),
            Constraint::SignerCount {
                prefix: "m".into(),
                at_least: 3
            }
        );
        assert_eq!(
            one("(signed $1)", &["/m/alice.id"]).unwrap(),
            Constraint::Signer {
                id: "m/alice.id".into()
            }
        );
        assert!(one("(signed $1)", &["/m/alice.text"]).is_none());
    }

    #[test]
    fn and_splits_into_atoms() {
        let t = Template::parse("(and (<= $2 $1) (<= $1 $3))").unwrap();
        assert_eq!(t.len(), 2);
        let cs = t
            .instantiate(&["/x.num".into(), "1".into(), "9".into()])
            .unwrap();
        assert_eq!(cs.len(), 2);
    }
}
