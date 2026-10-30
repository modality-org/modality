//! Invariant files: claims about one model that the checker and prover use,
//! never posted.
//!
//! ```text
//! file      := (invariant | decreases)*
//! invariant := "invariant" NAME "{" "on" anchor ("," anchor)* formula+ "}"
//! anchor    := NODE "->" "*"
//! formula   := "formula" NAME "{" FORMULA "}"
//! decreases := "decreases" NODE "->" NODE "by" TERM
//! ```
//!
//! An invariant is written like a rule. Its boxes see only moves on the
//! edges out of its anchor nodes, and a label reads the state a move starts
//! from, so `always([-P] false)` on `q2 -> *` says `P` holds whenever a run
//! leaves `q2`. A `decreases` measure is a number that is at least 0 and
//! drops by at least 1 on every move along its edge.

use super::term::Term;
use crate::ast::FormulaExpr;

#[derive(Debug, Clone)]
pub struct Invariant {
    pub name: String,
    pub nodes: Vec<String>,
    pub formulas: Vec<InvFormula>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct InvFormula {
    pub name: String,
    pub expr: FormulaExpr,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Decreases {
    pub from: String,
    pub to: String,
    pub term: Term,
    pub line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct InvariantFile {
    pub invariants: Vec<Invariant>,
    pub decreases: Vec<Decreases>,
}

impl InvariantFile {
    /// `invariant.formula`, for output.
    pub fn qualified(&self) -> Vec<(String, &Invariant, &InvFormula)> {
        self.invariants
            .iter()
            .flat_map(|i| {
                i.formulas
                    .iter()
                    .map(move |f| (format!("{}.{}", i.name, f.name), i, f))
            })
            .collect()
    }
}

pub fn parse(src: &str) -> Result<InvariantFile, String> {
    let mut s = Scanner { src, pos: 0 };
    let mut file = InvariantFile::default();
    s.trivia();
    while !s.at_end() {
        let line = s.line();
        match s.word().as_deref() {
            Some("invariant") => file.invariants.push(s.invariant(line)?),
            Some("decreases") => file.decreases.push(s.decreases(line)?),
            Some("rule") => {
                return s.err(
                    "a rule here (rules go in a rules file; this file holds `invariant` and `decreases`)",
                )
            }
            _ => return s.err("`invariant` or `decreases`"),
        }
        s.trivia();
    }
    Ok(file)
}

struct Scanner<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn line(&self) -> usize {
        self.src[..self.pos].matches('\n').count() + 1
    }

    fn col(&self) -> usize {
        let start = self.src[..self.pos].rfind('\n').map_or(0, |i| i + 1);
        self.src[start..self.pos].chars().count() + 1
    }

    fn err<T>(&self, what: &str) -> Result<T, String> {
        let found: String = self.rest().chars().take(20).collect();
        Err(format!(
            "line {}: expected {what}, found `{}`",
            self.line(),
            found.split_whitespace().collect::<Vec<_>>().join(" ")
        ))
    }

    fn trivia(&mut self) {
        loop {
            let t = self.rest().trim_start();
            self.pos = self.src.len() - t.len();
            if t.starts_with("//") {
                self.pos += t.find('\n').unwrap_or(t.len());
            } else {
                return;
            }
        }
    }

    fn word(&mut self) -> Option<String> {
        self.trivia();
        let r = self.rest();
        let n = r
            .char_indices()
            .find(|&(i, c)| !(c == '_' || c.is_alphanumeric() || (c == '*' && i == 0)))
            .map_or(r.len(), |(i, _)| i);
        let n = if r.starts_with('*') { 1 } else { n };
        if n == 0 {
            return None;
        }
        let w = r[..n].to_string();
        self.pos += n;
        Some(w)
    }

    fn punct(&mut self, p: &str) -> Result<(), String> {
        self.trivia();
        if self.rest().starts_with(p) {
            self.pos += p.len();
            Ok(())
        } else {
            self.err(&format!("`{p}`"))
        }
    }

    fn peek(&mut self, p: &str) -> bool {
        self.trivia();
        self.rest().starts_with(p)
    }

    fn name(&mut self, what: &str) -> Result<String, String> {
        match self.word() {
            Some(w) if w != "*" => Ok(w),
            _ => self.err(what),
        }
    }

    fn invariant(&mut self, line: usize) -> Result<Invariant, String> {
        let name = self.name("an invariant name")?;
        self.punct("{")?;
        if self.word().as_deref() != Some("on") {
            return self.err(
                "`on <node> -> *`: an invariant is anchored on the moves out of a node",
            );
        }
        let mut nodes = Vec::new();
        loop {
            let from = self.name("a node")?;
            self.punct("->")?;
            match self.word().as_deref() {
                Some("*") => {}
                _ => {
                    return self.err(
                        "`*`: an invariant sees every move out of its node (`on q2 -> *`); \
                         a label reads the state a move starts from",
                    )
                }
            }
            nodes.push(from);
            if self.peek(",") {
                self.punct(",")?;
            } else {
                break;
            }
        }
        let mut formulas = Vec::new();
        loop {
            self.trivia();
            if self.peek("}") {
                self.punct("}")?;
                break;
            }
            let (fl, fc) = (self.line(), self.col());
            if self.word().as_deref() != Some("formula") {
                return self.err("`formula <name> { ... }` or `}`");
            }
            let fname = self.name("a formula name")?;
            let body = self.braced()?;
            let wrapped = format!("formula {fname} {{\n{body}\n}}");
            let parsed = crate::parse_all_formulas_content_lalrpop(&wrapped)
                .map_err(|e| format!("line {fl}: formula {fname}: {e}"))?;
            let f = parsed
                .into_iter()
                .next()
                .ok_or_else(|| format!("line {fl}: formula {fname} is empty"))?;
            formulas.push(InvFormula {
                name: fname,
                expr: f.expression,
                line: fl,
                col: fc,
            });
        }
        if formulas.is_empty() {
            return Err(format!("line {line}: invariant {name} has no formula"));
        }
        Ok(Invariant {
            name,
            nodes,
            formulas,
            line,
        })
    }

    /// The text between a `{` and its matching `}`.
    fn braced(&mut self) -> Result<String, String> {
        self.punct("{")?;
        let start = self.pos;
        let mut depth = 1;
        for (i, c) in self.rest().char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let body = self.src[start..start + i].trim().to_string();
                        self.pos = start + i + 1;
                        return Ok(body);
                    }
                }
                _ => {}
            }
        }
        self.err("a closing `}`")
    }

    fn decreases(&mut self, line: usize) -> Result<Decreases, String> {
        let from = self.name("a node")?;
        self.punct("->")?;
        let to = self.name("a node")?;
        if self.word().as_deref() != Some("by") {
            return self.err("`by <term>`");
        }
        self.trivia();
        let end = self.rest().find('\n').unwrap_or(self.rest().len());
        let text = self.rest()[..end].split("//").next().unwrap_or("").trim().to_string();
        self.pos += end;
        let term = Term::parse(&text).map_err(|e| format!("line {line}: decreases: {e}"))?;
        Ok(Decreases {
            from,
            to,
            term,
            line,
        })
    }
}
