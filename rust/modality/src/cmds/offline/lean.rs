//! `modal prove --emit-lean DIR`: a Lake project in which Lean checks every
//! proven obligation again.
//!
//! Each obligation is one theorem: the facts the proof refuted, together,
//! are impossible (`… → False`). Numbers are `Int` (a `.natural` is a
//! `Nat`, cast), text is a list of Unicode scalar values, and the proof is
//! the text lemmas the certificate used followed by `omega`. The lemmas are
//! proved in `ModalityProof/Text.lean`. No mathlib.
//!
//! What Lean checks is each obligation. That the obligations together mean
//! the model meets its rules is the prover's method (`modal prove`'s
//! documentation), not a Lean theorem yet.

use super::project::Project;
use anyhow::{anyhow, Result};
use modality_lang::data::term::{Func, Term};
use modality_lang::data::value::{PathType, Value};
use modality_lang::prove::solve::{Atom, Close, Fml, Hyp, Lit, Op, Proof};
use modality_lang::prove::{Obligation, Prover, Verdict};
use std::collections::BTreeMap;
use std::fmt::Write as _;

const TEXT: &str = include_str!("lean/Text.lean");

struct Gen {
    vars: BTreeMap<String, (String, &'static str)>,
    opaque: BTreeMap<String, String>,
}

fn ident(s: &str) -> String {
    let s = s.replace(" -> ", "_to_");
    let mut out: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, 'v');
    }
    out
}

impl Gen {
    fn var(&mut self, path: &str, names: &super::fmt::Names) -> Result<String> {
        if let Some((v, _)) = self.vars.get(path) {
            return Ok(v.clone());
        }
        let (base, post) = match path.strip_prefix('\'') {
            Some(p) => (p, true),
            None => (path, false),
        };
        let ty = match PathType::of(base) {
            PathType::Text | PathType::Id => "Text",
            PathType::Natural => "Nat",
            PathType::Integer => "Int",
            PathType::Bool => "Bool",
            PathType::Rational => return Err(anyhow!("{base} is a rational; the export covers whole numbers")),
            PathType::Any => return Err(anyhow!("{base} has no type")),
        };
        let mut name = ident(&names.short(base));
        if post {
            name.push_str("_after");
        }
        while self.vars.values().any(|(v, _)| *v == name) {
            name.push('_');
        }
        self.vars.insert(path.to_string(), (name.clone(), ty));
        Ok(name)
    }

    fn num(&mut self, t: &Term, n: &super::fmt::Names) -> Result<String> {
        Ok(match t {
            Term::Path(p) => {
                let v = self.var(p, n)?;
                match self.vars[p].1 {
                    "Nat" => format!("({v} : Int)"),
                    "Int" => v,
                    other => return Err(anyhow!("{p} is {other}, not a number")),
                }
            }
            Term::Lit(Value::Num(r)) if r.is_integer() => format!("({r} : Int)"),
            Term::Lit(other) => return Err(anyhow!("{other} is not a whole number")),
            Term::Call(Func::Len, a) => format!("(({}).length : Int)", self.text(&a[0], n)?),
            Term::Call(Func::Plus, a) => {
                let parts = a.iter().map(|x| self.num(x, n)).collect::<Result<Vec<_>>>()?;
                format!("({})", parts.join(" + "))
            }
            Term::Call(Func::Minus, a) => format!("({} - {})", self.num(&a[0], n)?, self.num(&a[1], n)?),
            Term::Call(Func::Max, a) => format!("(max {} {})", self.num(&a[0], n)?, self.num(&a[1], n)?),
            Term::Call(Func::Min, a) => format!("(min {} {})", self.num(&a[0], n)?, self.num(&a[1], n)?),
            Term::Call(Func::Concat, _) => return Err(anyhow!("{t} is text, not a number")),
        })
    }

    fn text(&mut self, t: &Term, n: &super::fmt::Names) -> Result<String> {
        Ok(match t {
            Term::Path(p) => self.var(p, n)?,
            Term::Lit(Value::Text(s)) => format!("({s:?}.toList)"),
            Term::Call(Func::Concat, a) => {
                let parts = a.iter().map(|x| self.text(x, n)).collect::<Result<Vec<_>>>()?;
                format!("({})", parts.join(" ++ "))
            }
            other => return Err(anyhow!("{other} is not text")),
        })
    }

    fn atom(&mut self, a: &Atom, n: &super::fmt::Names) -> Result<String> {
        Ok(match a {
            Atom::Cmp(x, op, y) => {
                let op = match op {
                    Op::Eq => "=",
                    Op::Lt => "<",
                    Op::Le => "≤",
                    Op::Gt => ">",
                    Op::Ge => "≥",
                };
                format!("{} {op} {}", self.num(x, n)?, self.num(y, n)?)
            }
            Atom::TextEq(x, y) => format!("{} = {}", self.text(x, n)?, self.text(y, n)?),
            Atom::EndsWith(x, s) => format!("EndsWith {} {}", self.text(x, n)?, self.text(s, n)?),
            Atom::StartsWith(x, s) => format!("StartsWith {} {}", self.text(x, n)?, self.text(s, n)?),
            Atom::Prefix(x, k, c) => format!(
                "PrefixRepeats {} {} {}",
                self.text(x, n)?,
                self.num(k, n)?,
                self.text(c, n)?
            ),
            Atom::BoolIs(t, b) => match t {
                Term::Path(p) => format!("{} = {b}", self.var(p, n)?),
                Term::Lit(Value::Bool(v)) => format!("{v} = {b}"),
                other => return Err(anyhow!("{other} is not a bool")),
            },
            Atom::Const(b) => if *b { "True".into() } else { "False".into() },
            Atom::Contains(..) => return Err(anyhow!("text_contains has no lemma in the export yet")),
            Atom::Opaque(s) => {
                let k = self.opaque.len() + 1;
                self.opaque.entry(s.clone()).or_insert_with(|| format!("o{k}")).clone()
            }
        })
    }

    fn lit(&mut self, l: &Lit, n: &super::fmt::Names) -> Result<String> {
        let a = self.atom(&l.atom, n)?;
        Ok(if l.pos { a } else { format!("¬ ({a})") })
    }

    fn fml(&mut self, f: &Fml, n: &super::fmt::Names) -> Result<String> {
        Ok(match f {
            Fml::Lit(l) => self.lit(l, n)?,
            Fml::And(fs) if fs.is_empty() => "True".into(),
            Fml::Or(fs) if fs.is_empty() => "False".into(),
            Fml::And(fs) => {
                let parts = fs.iter().map(|f| self.fml(f, n)).collect::<Result<Vec<_>>>()?;
                format!("({})", parts.join(" ∧ "))
            }
            Fml::Or(fs) => {
                let parts = fs.iter().map(|f| self.fml(f, n)).collect::<Result<Vec<_>>>()?;
                format!("({})", parts.join(" ∨ "))
            }
        })
    }
}

/// The `have`s a proof's text steps need, in Lean.
fn haves(g: &mut Gen, proof: &Proof, n: &super::fmt::Names, out: &mut Vec<String>) -> Result<()> {
    match proof {
        Proof::Split { cases, .. } => {
            for (_, p) in cases {
                haves(g, p, n, out)?;
            }
        }
        Proof::Closed(Close::Text { lemma, target, from, .. }) => {
            let target_s = g.atom(target, n)?;
            let line = match (*lemma, target, from) {
                ("ends_with_self", Atom::EndsWith(x, s), _) => {
                    if matches!(s, Term::Lit(Value::Text(t)) if t.is_empty()) {
                        format!("have : {target_s} := ⟨{}, by simp⟩", g.text(x, n)?)
                    } else {
                        format!("have : {target_s} := Modality.ends_self _")
                    }
                }
                ("ends_with_concat", Atom::EndsWith(x, _), Some(f)) => {
                    let from_s = g.atom(f, n)?;
                    let Atom::EndsWith(y, _) = f else { return Ok(()) };
                    let parts = match x {
                        Term::Call(Func::Concat, a) => a.clone(),
                        other => vec![other.clone()],
                    };
                    let y_flat = modality_lang::prove::solve::flat(y);
                    let k = (0..parts.len())
                        .find(|&i| modality_lang::prove::solve::flat(&rebuild(&parts[i..])) == y_flat)
                        .unwrap_or(parts.len().saturating_sub(1));
                    let mut proof = format!("(by assumption : {from_s})");
                    for p in parts[..k].iter().rev() {
                        proof = format!("(Modality.ends_concat {} {proof})", g.text(p, n)?);
                    }
                    format!("have : {target_s} := {proof}")
                }
                ("prefix_repeats_zero", _, _) => format!("have : {target_s} := Modality.prefix_zero (by omega)"),
                ("prefix_repeats_cons", _, Some(f)) => {
                    let from_s = g.atom(f, n)?;
                    format!("have : {target_s} := Modality.prefix_cons (by assumption : {from_s}) (by omega) (by omega)")
                }
                ("same_atom", Atom::Prefix(..), Some(f)) => {
                    let from_s = g.atom(f, n)?;
                    format!("have : {target_s} := Modality.prefix_congr (by assumption : {from_s}) (by omega)")
                }
                ("starts_with_self", _, _) => format!("have : {target_s} := Modality.starts_self _"),
                ("text_eq_self", _, _) => format!("have : {target_s} := rfl"),
                _ => return Ok(()),
            };
            if !out.iter().any(|l: &String| l.starts_with(&format!("have : {target_s} :="))) {
                out.push(line);
            }
        }
        Proof::Closed(_) => {}
    }
    Ok(())
}

fn rebuild(parts: &[Term]) -> Term {
    match parts {
        [one] => one.clone(),
        many => Term::Call(Func::Concat, many.to_vec()),
    }
}

/// Fold constants: `true = true` is `True`, a conjunction drops `True`.
fn simplify(f: &Fml) -> Fml {
    match f {
        Fml::Lit(Lit { pos, atom: Atom::BoolIs(Term::Lit(Value::Bool(v)), b) }) => {
            Fml::Lit(Lit { pos: true, atom: Atom::Const((v == b) == *pos) })
        }
        Fml::Lit(Lit { pos: false, atom: Atom::Const(b) }) => Fml::Lit(Lit { pos: true, atom: Atom::Const(!b) }),
        Fml::Lit(l) => Fml::Lit(l.clone()),
        Fml::And(fs) => {
            let fs: Vec<Fml> = fs.iter().map(simplify).collect();
            if fs.iter().any(|f| matches!(f, Fml::Lit(Lit { pos: true, atom: Atom::Const(false) }))) {
                return Fml::Lit(Lit { pos: true, atom: Atom::Const(false) });
            }
            let mut fs: Vec<Fml> = fs.into_iter().filter(|f| !matches!(f, Fml::Lit(Lit { pos: true, atom: Atom::Const(true) }))).collect();
            if fs.len() == 1 { fs.remove(0) } else { Fml::And(fs) }
        }
        Fml::Or(fs) => {
            let fs: Vec<Fml> = fs.iter().map(simplify).collect();
            if fs.iter().any(|f| matches!(f, Fml::Lit(Lit { pos: true, atom: Atom::Const(true) }))) {
                return Fml::Lit(Lit { pos: true, atom: Atom::Const(true) });
            }
            let mut fs: Vec<Fml> = fs.into_iter().filter(|f| !matches!(f, Fml::Lit(Lit { pos: true, atom: Atom::Const(false) }))).collect();
            if fs.len() == 1 { fs.remove(0) } else { Fml::Or(fs) }
        }
    }
}

/// Top-level conjuncts as separate facts.
fn conjuncts(f: Fml, out: &mut Vec<Fml>) {
    match f {
        Fml::And(fs) => fs.into_iter().for_each(|f| conjuncts(f, out)),
        Fml::Lit(Lit { pos: true, atom: Atom::Const(true) }) => {}
        f => out.push(f),
    }
}

fn theorem(name: &str, from: &[Hyp], proof: &Proof, names: &super::fmt::Names) -> Result<String> {
    let mut g = Gen {
        vars: BTreeMap::new(),
        opaque: BTreeMap::new(),
    };
    let mut hyps = Vec::new();
    let mut lens = Vec::new();
    let mut facts = Vec::new();
    for h in from {
        conjuncts(simplify(&h.fml), &mut facts);
    }
    for (i, f) in facts.iter().enumerate() {
        let h = Hyp { fml: f.clone(), src: modality_lang::prove::solve::Src::Goal };
        let s = g.fml(&h.fml, names)?;
        // What the arithmetic reads from positive text facts.
        if let Fml::Lit(Lit { pos: true, atom }) = &h.fml {
            match atom {
                Atom::EndsWith(..) => lens.push(format!("have := Modality.ends_len h{}", i + 1)),
                Atom::Prefix(..) => lens.push(format!("have := Modality.prefix_len h{}", i + 1)),
                _ => {}
            }
        }
        if let Fml::Or(fs) = &h.fml {
            if let [Fml::Lit(Lit { pos: true, atom })] = fs.as_slice() {
                match atom {
                    Atom::EndsWith(..) => lens.push(format!("have := Modality.ends_len h{}", i + 1)),
                    Atom::Prefix(..) => lens.push(format!("have := Modality.prefix_len h{}", i + 1)),
                    _ => {}
                }
            }
        }
        hyps.push(format!("(h{} : {s})", i + 1));
    }
    let mut steps = Vec::new();
    haves(&mut g, proof, names, &mut steps)?;
    let mut binders: Vec<String> = g.vars.values().map(|(v, t)| format!("({v} : {t})")).collect();
    binders.extend(g.opaque.values().map(|o| format!("({o} : Prop)")));
    let mut out = String::new();
    let _ = writeln!(out, "theorem {name}\n    {}\n    {} : False := by", binders.join(" "), hyps.join("\n    "));
    for l in &lens {
        let _ = writeln!(out, "  {l}");
    }
    // A step from one case may not hold in the others: each is tried.
    for l in &steps {
        let _ = writeln!(out, "  try {l}");
    }
    let _ = writeln!(out, "  try simp only [List.length_append, List.length_cons, List.length_nil] at *");
    let _ = writeln!(out, "  first | omega | contradiction | simp_all");
    Ok(out)
}

/// The Lean the export is written for, pinned: the same project builds the
/// same way on every machine.
fn toolchain() -> String {
    "leanprover/lean4:v4.14.0".to_string()
}

pub fn write(p: &Project, prover: &Prover, obs: &[Obligation], dir: &str) -> Result<String> {
    let names = super::fmt::Names::new(p.paths());
    let root = std::path::Path::new(dir);
    std::fs::create_dir_all(root.join("ModalityProof"))?;
    std::fs::write(root.join("lean-toolchain"), toolchain() + "\n")?;
    std::fs::write(
        root.join("lakefile.lean"),
        "import Lake\nopen Lake DSL\n\npackage «modality_proof»\n\n@[default_target]\nlean_lib «ModalityProof» where\n  globs := #[.submodules `ModalityProof]\n",
    )?;
    std::fs::write(root.join("ModalityProof/Text.lean"), TEXT)?;
    let mut body = String::new();
    let _ = writeln!(body, "import ModalityProof.Text\nopen Modality\nset_option linter.unusedVariables false\n");
    let _ = writeln!(
        body,
        "/-! Obligations of model `{}`, from `modal prove` (semantics {}, prover {}).\nEach theorem is one proven obligation: the facts it lists are impossible together. -/\n",
        p.model.name,
        super::prove::SEMANTICS,
        super::prove::PROVER
    );
    let (mut n, mut skipped) = (0, Vec::new());
    for ob in obs {
        for part in &ob.parts {
            let Verdict::Proven { proof: Some(proof), from, .. } = &part.verdict else { continue };
            let edge = part.edge.map(|e| prover.machine.edges[e].name()).unwrap_or_default();
            let name = ident(&format!("{} {} {}", ob.kind.name(), if ob.kind == modality_lang::prove::ObKind::Rule { ob.subject.clone() } else { edge.clone() }, if part.what != ob.subject { part.what.clone() } else { String::new() }))
                .trim_end_matches('_')
                .to_string();
            let name = if ob.kind == modality_lang::prove::ObKind::Rule { format!("{name}_at_{}", ident(&edge)) } else { name };
            match theorem(&name, from, proof, &names) {
                Ok(t) => {
                    let _ = writeln!(body, "-- {} at {edge}: {}\n{t}", ob.id(), part.what);
                    n += 1;
                }
                Err(e) => skipped.push(format!("{} ({}): {e}", ob.id(), part.what)),
            }
        }
    }
    for s in &skipped {
        let _ = writeln!(body, "-- not exported: {s}");
    }
    std::fs::write(root.join("ModalityProof/Obligations.lean"), body)?;
    std::fs::write(
        root.join("ModalityProof.lean"),
        "import ModalityProof.Text\nimport ModalityProof.Obligations\n",
    )?;
    let mut msg = format!(
        "wrote {dir}: a Lake project ({}, no mathlib) with {n} obligation theorem{}",
        toolchain(),
        if n == 1 { "" } else { "s" }
    );
    if !skipped.is_empty() {
        let _ = write!(msg, "; {} not exported (see the end of Obligations.lean)", skipped.len());
    }
    let _ = write!(msg, "\n  check them: cd {dir} && lake build");
    Ok(msg)
}
