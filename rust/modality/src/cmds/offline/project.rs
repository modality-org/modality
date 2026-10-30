//! The files a check reads: one model, rule files, an invariant file.

use anyhow::{anyhow, bail, Result};
use modality_lang::ast::FormulaExpr;
use modality_lang::data::invariants::{self, InvariantFile};
use modality_lang::data::Machine;
use modality_lang::rule_file::parse_rule_file;
use modality_lang::Model;
use std::path::{Path, PathBuf};

pub struct Source {
    /// The path as the user gave it (or as found in a directory).
    pub name: String,
    pub path: PathBuf,
    pub text: String,
}

impl Source {
    pub fn line(&self, n: usize) -> &str {
        self.text.lines().nth(n.saturating_sub(1)).unwrap_or("")
    }

    /// `(line, col)` of byte offset `at`, both from 1.
    pub fn pos(&self, at: usize) -> (usize, usize) {
        let before = &self.text[..at.min(self.text.len())];
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        (line, col)
    }
}

#[derive(Clone)]
pub struct Item {
    /// `rule.formula`, or the formula's name in a model file.
    pub name: String,
    pub expr: FormulaExpr,
    pub file: usize,
    pub line: usize,
    pub col: usize,
}

pub struct Project {
    pub sources: Vec<Source>,
    pub model_src: usize,
    pub model: Model,
    pub machine: Machine,
    pub formulas: Vec<Item>,
    /// Rule files, with how many formulas each holds.
    pub rule_files: Vec<(usize, usize)>,
    pub invariants: Option<(usize, InvariantFile)>,
}

#[derive(PartialEq)]
enum FileKind {
    Model,
    Rules,
    Invariants,
}

fn first_word(text: &str) -> String {
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        return t
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("")
            .to_string();
    }
    String::new()
}

fn kind_of(text: &str) -> FileKind {
    let has_model = text.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("model ") || t.starts_with("export default model")
    });
    match first_word(text).as_str() {
        "invariant" | "decreases" => FileKind::Invariants,
        _ if has_model => FileKind::Model,
        _ => FileKind::Rules,
    }
}

fn expand(inputs: &[String]) -> Result<Vec<(String, PathBuf)>> {
    let mut out = Vec::new();
    for input in inputs {
        let p = Path::new(input);
        if p.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(p)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|e| e.extension().is_some_and(|x| x == "modality"))
                .collect();
            entries.sort();
            for e in entries {
                let name = format!(
                    "{}/{}",
                    input.trim_end_matches('/'),
                    e.file_name().unwrap_or_default().to_string_lossy()
                );
                out.push((name, e));
            }
        } else {
            out.push((input.clone(), p.to_path_buf()));
        }
    }
    Ok(out)
}

/// Where `formula <name>` starts in `text`.
fn find_formula(text: &str, name: &str) -> Option<usize> {
    let re = regex::Regex::new(&format!(r"\bformula\s+{}\b", regex::escape(name))).ok()?;
    re.find(text).map(|m| m.start())
}

pub fn parse_formula(name: &str, body: &str) -> Result<FormulaExpr> {
    let wrapped = format!("formula {name} {{\n{body}\n}}");
    let mut f = modality_lang::parse_all_formulas_content_lalrpop(&wrapped)
        .map_err(|e| anyhow!("formula {name}: {e}"))?;
    if f.is_empty() {
        bail!("formula {name} is empty");
    }
    Ok(f.remove(0).expression)
}

pub fn load(inputs: &[String]) -> Result<Project> {
    let mut sources = Vec::new();
    for (name, path) in expand(inputs)? {
        let text = std::fs::read_to_string(&path).map_err(|e| anyhow!("{name}: {e}"))?;
        sources.push(Source { name, path, text });
    }
    load_sources(sources)
}

pub fn load_sources(sources: Vec<Source>) -> Result<Project> {
    let mut model_src = None;
    let mut formulas = Vec::new();
    let mut rule_files = Vec::new();
    let mut invariants = None;
    let mut model = None;
    for (i, s) in sources.iter().enumerate() {
        match kind_of(&s.text) {
            FileKind::Model => {
                if let Some(prev) = model_src {
                    let prev: usize = prev;
                    bail!(
                        "{}: a check reads one model; another is in {}",
                        s.name,
                        sources[prev].name
                    );
                }
                let mut models = modality_lang::parse_all_models_content_lalrpop(&s.text)
                    .map_err(|e| anyhow!("{}: {e}", s.name))?;
                if models.is_empty() {
                    bail!("{}: no model", s.name);
                }
                model = Some(models.remove(0));
                model_src = Some(i);
                let fs = modality_lang::parse_all_formulas_content_lalrpop(&s.text)
                    .map_err(|e| anyhow!("{}: {e}", s.name))?;
                for f in fs {
                    let at = find_formula(&s.text, &f.name).unwrap_or(0);
                    let (line, col) = s.pos(at);
                    formulas.push(Item {
                        name: f.name.clone(),
                        expr: f.expression,
                        file: i,
                        line,
                        col,
                    });
                }
            }
            FileKind::Rules => {
                let blocks = parse_rule_file(&s.text).map_err(|e| anyhow!("{}: {e}", s.name))?;
                let mut n = 0;
                for b in blocks {
                    for (k, f) in b.formulas.iter().enumerate() {
                        let name = match (&b.name, &f.name) {
                            (Some(r), Some(f)) => format!("{r}.{f}"),
                            (Some(r), None) if b.formulas.len() == 1 => r.clone(),
                            (Some(r), None) => format!("{r}.{}", k + 1),
                            (None, Some(f)) => f.clone(),
                            (None, None) => format!("rule.{}", k + 1),
                        };
                        let fname = f.name.clone().unwrap_or_else(|| "rule".into());
                        let expr = parse_formula(&fname, &f.body)
                            .map_err(|e| anyhow!("{}: {e}", s.name))?;
                        let at = f
                            .name
                            .as_deref()
                            .and_then(|n| find_formula(&s.text, n))
                            .unwrap_or(0);
                        let (line, col) = s.pos(at);
                        formulas.push(Item {
                            name,
                            expr,
                            file: i,
                            line,
                            col,
                        });
                        n += 1;
                    }
                }
                rule_files.push((i, n));
            }
            FileKind::Invariants => {
                if invariants.is_some() {
                    bail!("{}: one invariant file per check", s.name);
                }
                let f = invariants::parse(&s.text).map_err(|e| anyhow!("{}: {e}", s.name))?;
                invariants = Some((i, f));
            }
        }
    }
    let Some(model_src) = model_src else {
        bail!("no model: give a .modality file with `model <name> {{ ... }}`");
    };
    let model = model.unwrap();
    let machine = Machine::new(&model).map_err(|e| anyhow!("{}: {e}", sources[model_src].name))?;
    Ok(Project {
        sources,
        model_src,
        model,
        machine,
        formulas,
        rule_files,
        invariants,
    })
}

impl Project {
    pub fn model_source(&self) -> &Source {
        &self.sources[self.model_src]
    }

    /// `(line, col)` of edge `index` in the model file.
    pub fn edge_pos(&self, index: usize) -> (usize, usize) {
        let e = &self.machine.edges[index];
        let src = self.model_source();
        let nth = self.machine.edges[..index]
            .iter()
            .filter(|o| o.from == e.from && o.to == e.to)
            .count();
        let re = regex::Regex::new(&format!(
            r"\b{}\s*-{{1,2}}>\s*{}\b",
            regex::escape(&e.from),
            regex::escape(&e.to)
        ))
        .expect("edge pattern");
        let start = re.find_iter(&src.text).nth(nth).map(|m| m.start());
        match start {
            Some(at) => src.pos(at),
            None => (1, 1),
        }
    }

    /// Every path the model, rules and invariants mention.
    pub fn paths(&self) -> Vec<String> {
        let re = regex::Regex::new(r"/[a-zA-Z0-9_./$!]+\.[a-z]+").expect("path pattern");
        let mut out: Vec<String> = Vec::new();
        for s in &self.sources {
            for m in re.find_iter(&s.text) {
                let p = m.as_str().to_string();
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        // Inputs, which no edge writes, first.
        let written: Vec<String> = self
            .machine
            .edges
            .iter()
            .flat_map(|e| e.required().into_iter().map(|(p, _)| p))
            .collect();
        let (inputs, rest): (Vec<String>, Vec<String>) = out.into_iter().partition(|p| !written.contains(p));
        inputs.into_iter().chain(rest).collect()
    }

    /// The header naming the files: `a.modality · b.modality (3 formulas)`.
    pub fn files_header(&self) -> String {
        let mut parts = Vec::new();
        let model = self.model_source();
        let in_model = self.formulas.iter().filter(|f| f.file == self.model_src).count();
        if self.rule_files.is_empty() && self.invariants.is_none() {
            parts.push(format!("{} · model {}", model.name, self.model.name));
        } else {
            parts.push(model.name.clone());
        }
        if in_model > 0 && !self.rule_files.is_empty() {
            parts[0] = format!("{} ({} formula{})", model.name, in_model, plural(in_model));
        }
        for (f, n) in &self.rule_files {
            parts.push(format!("{} ({n} formula{})", self.sources[*f].name, plural(*n)));
        }
        if let Some((f, inv)) = &self.invariants {
            let n: usize = inv.invariants.iter().map(|i| i.formulas.len()).sum();
            parts.push(format!("{} ({n} formula{})", self.sources[*f].name, plural(n)));
        }
        parts.join(" · ")
    }
}

pub fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}
