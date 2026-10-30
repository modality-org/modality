//! `modal test`: runs, checks, proofs and mutations in `*.test.toml`.
//!
//! ```toml
//! [[run]]       # a run with inputs, and values it must end with
//! name = "pads foo to five"
//! set = { "/in/c.text" = "0" }
//! expect = { "/out/v.text" = "00foo" }
//!
//! [[check]]     # a check (bounded with `bounded`), every item or by name
//! bounded = "domains.toml"
//! expect = { all = "no-counterexample" }
//!
//! [[prove]]     # a proof
//! expect = { all = "proven" }
//!
//! [[mutation]]  # break the model on purpose; the rule must notice
//! edge = "q2 -> q2"
//! remove = "-modifies(/out/done.bool)"
//! bounded = "domains.toml"
//! expect = { "spec.length" = "violated" }
//! ```
//!
//! A mutation removes a label from an edge (`edge`, `remove`), a formula
//! from an invariant (`invariant`, `remove`), a measure (`remove =
//! "decreases q2 -> q2"`), or renames paths (`retype`). With `prove = true`
//! it is proved, otherwise checked.

use super::check::{self, Verdicts};
use super::fmt::{summary_line, thousands};
use super::project::{self, Source};
use super::run::{parse_set, render};
use anyhow::{anyhow, bail, Result};
use clap::Parser;
use modality_lang::data::machine::run as run_model;
use modality_lang::data::{PathType, Value};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug, Clone)]
pub struct Opts {
    /// Test files (default: every *.test.toml here)
    pub files: Vec<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let (out, code) = report(opts)?;
    print!("{out}");
    std::process::exit(code);
}

fn sibling(test: &Path, suffix: &str) -> PathBuf {
    let name = test.file_name().unwrap_or_default().to_string_lossy();
    let stem = name.trim_end_matches(".test.toml");
    test.with_file_name(format!("{stem}{suffix}"))
}

fn table_str(t: &toml::Value, k: &str) -> Option<String> {
    t.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

fn raw(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub fn report(opts: &Opts) -> Result<(String, i32)> {
    let files: Vec<PathBuf> = if opts.files.is_empty() {
        let mut fs: Vec<PathBuf> = std::fs::read_dir(".")?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_string_lossy().ends_with(".test.toml"))
            .collect();
        fs.sort();
        fs
    } else {
        opts.files.iter().map(PathBuf::from).collect()
    };
    if files.is_empty() {
        bail!("no *.test.toml here");
    }
    let mut out = String::new();
    let (mut passed, mut failed) = (0, 0);
    for file in &files {
        let _ = writeln!(out, "{}", file.display().to_string().trim_start_matches("./"));
        let text = std::fs::read_to_string(file).map_err(|e| anyhow!("{}: {e}", file.display()))?;
        let doc: toml::Value = toml::from_str(&text).map_err(|e| anyhow!("{}: {e}", file.display()))?;
        let model = doc
            .get("model")
            .and_then(|m| m.as_str())
            .map(|m| file.with_file_name(m))
            .unwrap_or_else(|| sibling(file, ".modality"));
        // Every test kind, in the order the file lists them.
        let mut tests: Vec<(usize, &str, &toml::Value)> = Vec::new();
        for kind in ["run", "check", "prove", "mutation"] {
            for t in doc.get(kind).and_then(|v| v.as_array()).into_iter().flatten() {
                let name = table_str(t, "name").unwrap_or_default();
                let at = text.find(&format!("name = {name:?}")).unwrap_or(usize::MAX);
                tests.push((at, kind, t));
            }
        }
        tests.sort_by_key(|(at, _, _)| *at);
        for (_, kind, t) in tests {
            let name = table_str(t, "name").unwrap_or_else(|| kind.to_string());
            match one(file, &model, kind, t) {
                Ok(note) => {
                    passed += 1;
                    let _ = writeln!(out, "{}", format!("  ✓ {name:<48} {note}").trim_end());
                }
                Err(e) => {
                    failed += 1;
                    let _ = writeln!(out, "  ✗ {name}");
                    let _ = writeln!(out, "      {e}");
                }
            }
        }
    }
    let code = if failed > 0 { 1 } else { 0 };
    let text = if failed > 0 {
        format!("{passed} passed, {failed} failed")
    } else {
        format!("{passed} passed")
    };
    let _ = writeln!(out, "{}", summary_line(&text, code));
    Ok((out, code))
}

/// The sources a test reads: the model, and its rules, invariants, if they
/// exist next to the test or are named.
fn sources(file: &Path, model: &Path, t: &toml::Value, rules: bool) -> Result<Vec<Source>> {
    let mut paths = vec![model.to_path_buf()];
    if rules {
        let r = table_str(t, "rules").map(|r| file.with_file_name(r)).unwrap_or_else(|| sibling(file, ".rules.modality"));
        if r.exists() {
            paths.push(r);
        }
        let i = table_str(t, "invariants")
            .map(|i| file.with_file_name(i))
            .unwrap_or_else(|| sibling(file, ".invariants.modality"));
        if i.exists() {
            paths.push(i);
        }
    }
    paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).map_err(|e| anyhow!("{}: {e}", p.display()))?;
            Ok(Source {
                name: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
                path: p,
                text,
            })
        })
        .collect()
}

fn expect(t: &toml::Value, got: &Verdicts) -> Result<String> {
    let want = t.get("expect").and_then(|e| e.as_table()).ok_or_else(|| anyhow!("give expect = {{ ... }}"))?;
    for (k, v) in want {
        let v = raw(v);
        if k == "all" {
            let items: Vec<(&String, &String)> = got
                .iter()
                .filter(|(k, _)| k.as_str() != "runs" && k.as_str() != "proven")
                .collect();
            // An invariant that holds passes any `all`.
            if let Some((k2, v2)) = items
                .iter()
                .find(|(k, w)| **w != v && !(k.starts_with("invariant ") && w.as_str() == "holds"))
            {
                bail!("expected every item {v}, but {k2} is {v2}");
            }
            continue;
        }
        let key = k.replacen("write ", "write to ", 1).replace("write to to ", "write to ");
        let found = got.get(k).or_else(|| got.get(&key));
        match found {
            Some(g) if *g == v => {}
            Some(g) => bail!("expected {k} {v}, got {g}"),
            None => bail!("no item {k} (items: {})", got.keys().cloned().collect::<Vec<_>>().join(", ")),
        }
    }
    let one = want.len() == 1 && !want.contains_key("all");
    Ok(if one {
        let (k, v) = want.iter().next().unwrap();
        let _ = k;
        format!("({}, as expected)", raw(v))
    } else if let Some(r) = got.get("runs") {
        format!("({} runs)", thousands(r.parse().unwrap_or(0)))
    } else if let Some(p) = got.get("proven") {
        format!("({p} proven)")
    } else {
        String::new()
    })
}

fn one(file: &Path, model: &Path, kind: &str, t: &toml::Value) -> Result<String> {
    match kind {
        "run" => {
            let srcs = sources(file, model, t, false)?;
            let p = project::load_sources(srcs)?;
            let mut inputs = Vec::new();
            for (k, v) in t.get("set").and_then(|s| s.as_table()).into_iter().flatten() {
                inputs.push(parse_set(&format!("{k}={}", raw(v)))?);
            }
            let r = run_model(&p.machine, &inputs, 10_000);
            let (_, code) = render(&p.machine, &r, &inputs, false);
            if code != 0 {
                bail!("the run was refused");
            }
            for (k, v) in t.get("expect").and_then(|s| s.as_table()).into_iter().flatten() {
                let want: Value = PathType::of(k).parse_value(&raw(v)).map_err(|e| anyhow!(e))?;
                match r.final_state.get(k) {
                    Some(got) if *got == want => {}
                    Some(got) => bail!("{k} is {got}, expected {want}"),
                    None => bail!("{k} was never written"),
                }
            }
            Ok(String::new())
        }
        "check" | "prove" => {
            let srcs = sources(file, model, t, true)?;
            verdicts(file, kind == "prove", srcs, t).and_then(|v| expect(t, &v))
        }
        "mutation" => {
            let prove = t.get("prove").and_then(|p| p.as_bool()).unwrap_or(false);
            let srcs = mutate(sources(file, model, t, true)?, t)?;
            verdicts(file, prove, srcs, t).and_then(|v| expect(t, &v))
        }
        other => bail!("unknown test kind {other}"),
    }
}

fn verdicts(file: &Path, prove: bool, srcs: Vec<Source>, t: &toml::Value) -> Result<Verdicts> {
    let p = project::load_sources(srcs)?;
    let mut v = Verdicts::new();
    let here = std::env::current_dir()?;
    let dir = file.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    // Counterexample files go next to the test, as from the command line.
    std::env::set_current_dir(dir)?;
    let result = if prove {
        let opts = super::prove::Opts {
            inputs: Vec::new(),
            list: false,
            only: None,
            explain: None,
            emit_certificate: None,
            emit_lean: None,
            houdini: None,
            seed: None,
            write: false,
            watch: false,
        };
        super::prove::report_project(&p, &opts, &mut v).map(|_| ())
    } else {
        let opts = check::Opts {
            inputs: Vec::new(),
            bounded: table_str(t, "bounded"),
            inductive: t.get("inductive").and_then(|i| i.as_bool()).unwrap_or(false),
            state: None,
            theory: modality_lang::TheoryVersion::V5,
            watch: false,
        };
        check::check_project(&p, &opts, &mut v).map(|_| ())
    };
    std::env::set_current_dir(here)?;
    result?;
    Ok(v)
}

/// The text between the `{` at or after `from` and its matching `}`.
fn block_end(text: &str, from: usize) -> Option<usize> {
    let open = from + text[from..].find('{')?;
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn mutate(mut srcs: Vec<Source>, t: &toml::Value) -> Result<Vec<Source>> {
    if let Some(map) = t.get("retype").and_then(|r| r.as_table()) {
        for (from, to) in map {
            let to = raw(to);
            for s in &mut srcs {
                s.text = s.text.replace(from.as_str(), &to);
            }
        }
    }
    let remove = table_str(t, "remove");
    if let (Some(edge), Some(label)) = (table_str(t, "edge"), remove.as_deref()) {
        let (from, to) = edge.split_once("->").ok_or_else(|| anyhow!("edge is `from -> to`"))?;
        let re = regex::Regex::new(&format!(r"\b{}\s*->\s*{}\s*\[", regex::escape(from.trim()), regex::escape(to.trim())))?;
        let s = &mut srcs[0];
        let m = re.find(&s.text).ok_or_else(|| anyhow!("no edge {edge} in {}", s.name))?;
        let close = m.end() + s.text[m.end()..].find(']').ok_or_else(|| anyhow!("edge {edge} has no `]`"))?;
        let labels = &s.text[m.end()..close];
        let at = labels.find(label).ok_or_else(|| anyhow!("edge {edge} has no label {label}"))?;
        let start = m.end() + at;
        s.text.replace_range(start..start + label.len(), "");
        return Ok(srcs);
    }
    if let (Some(inv), Some(formula)) = (table_str(t, "invariant"), remove.as_deref()) {
        let s = srcs
            .iter_mut()
            .find(|s| s.text.contains(&format!("invariant {inv}")))
            .ok_or_else(|| anyhow!("no invariant {inv}"))?;
        let re = regex::Regex::new(&format!(r"\bformula\s+{}\b", regex::escape(formula)))?;
        let m = re.find(&s.text).ok_or_else(|| anyhow!("invariant {inv} has no formula {formula}"))?;
        let end = block_end(&s.text, m.start()).ok_or_else(|| anyhow!("formula {formula} is not closed"))?;
        let line_start = s.text[..m.start()].rfind('\n').map_or(0, |i| i + 1);
        let end = if s.text[end..].starts_with('\n') { end + 1 } else { end };
        s.text.replace_range(line_start..end, "");
        return Ok(srcs);
    }
    if let Some(r) = remove.as_deref().filter(|r| r.starts_with("decreases")) {
        let s = srcs
            .iter_mut()
            .find(|s| s.text.contains(r))
            .ok_or_else(|| anyhow!("no `{r}`"))?;
        s.text = s.text.lines().filter(|l| !l.trim_start().starts_with(r)).collect::<Vec<_>>().join("\n");
        return Ok(srcs);
    }
    if t.get("retype").is_none() {
        bail!("a mutation gives edge + remove, invariant + remove, remove = \"decreases …\", or retype");
    }
    Ok(srcs)
}
