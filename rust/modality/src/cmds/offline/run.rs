//! `modal run model.modality --set /in/x.text=foo`: run a model on data.

use super::project;
use anyhow::{anyhow, bail, Result};
use clap::Args;
use modality_lang::data::machine::{run as run_model, Machine, Outcome, Run, RunStop};
use modality_lang::data::{PathType, State, Value, Writes};
use std::fmt::Write as _;

#[derive(Args, Debug, Clone, Default)]
pub struct Opts {
    /// The model file to run
    pub model: Option<String>,

    /// Post an input: --set /in/c.text=0 (repeatable)
    #[arg(long = "set", value_name = "PATH=VALUE")]
    pub set: Vec<String>,

    /// Print every move's writes
    #[arg(long)]
    pub trace: bool,

    /// Replay a counterexample from `modal check --bounded`
    #[arg(long)]
    pub replay: Option<String>,

    /// Stop after this many moves
    #[arg(long, default_value_t = 10_000)]
    pub limit: usize,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let (out, code) = report(opts)?;
    print!("{out}");
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

pub fn parse_set(raw: &str) -> Result<(String, Value)> {
    let (path, value) = raw
        .split_once('=')
        .ok_or_else(|| anyhow!("--set takes PATH=VALUE, as in --set /in/c.text=0"))?;
    let v = PathType::of(path)
        .parse_value(value)
        .map_err(|e| anyhow!("--set {path}: {e}"))?;
    Ok((path.to_string(), v))
}

pub fn report(opts: &Opts) -> Result<(String, i32)> {
    let model = opts
        .model
        .as_deref()
        .ok_or_else(|| anyhow!("give a model file: modal run model.modality --set PATH=VALUE"))?;
    let p = project::load(&[model.to_string()])?;
    let m = &p.machine;
    if let Some(file) = &opts.replay {
        return replay(m, file);
    }
    let inputs = opts
        .set
        .iter()
        .map(|s| parse_set(s))
        .collect::<Result<Vec<_>>>()?;
    let r = run_model(m, &inputs, opts.limit);
    Ok(render(m, &r, &inputs, opts.trace))
}

fn writes_text(e: &modality_lang::data::Edge, w: &Writes) -> String {
    e.write_order(w)
        .iter()
        .map(|k| format!("{k} = {}", w[k]))
        .collect::<Vec<_>>()
        .join("  ")
}

pub fn render(m: &Machine, r: &Run, inputs: &[(String, Value)], trace: bool) -> (String, i32) {
    let mut out = String::new();
    if trace {
        for s in &r.steps {
            let e = &m.edges[s.edge];
            let _ = writeln!(out, "{}   {}", e.name(), writes_text(e, &s.writes));
        }
    }
    let _ = writeln!(out, "{}", r.path(m));
    // What the run wrote, in the order it first wrote it, without the inputs.
    let mut order: Vec<String> = Vec::new();
    for s in &r.steps {
        for k in m.edges[s.edge].write_order(&s.writes) {
            if !order.contains(&k) && !inputs.iter().any(|(p, _)| *p == k) {
                order.push(k);
            }
        }
    }
    let finals: Vec<String> = order
        .iter()
        .filter_map(|k| r.final_state.get(k).map(|v| format!("{k} = {v}")))
        .collect();
    if !finals.is_empty() {
        let _ = writeln!(out, "{}", finals.join("   "));
    }
    let code = match &r.stop {
        RunStop::NoMove => {
            let node = r.steps.last().map(|s| m.edges[s.edge].to.clone()).unwrap_or_else(|| r.start.clone());
            if m.edges_from(&node).next().is_some() {
                let _ = writeln!(out, "stopped at {node}: no move from here is possible");
            }
            0
        }
        RunStop::BadWrite { edge, path, value } => {
            let _ = writeln!(
                out,
                "refused: {} would write {value} to {path}, which a {} cannot hold",
                m.edges[*edge].name(),
                PathType::of(path).name()
            );
            1
        }
        RunStop::Limit => {
            let _ = writeln!(out, "stopped after {} moves (--limit)", r.steps.len());
            0
        }
    };
    (out, code)
}

/// A replay file from `modal check --bounded`: each move, and what it wrote
/// of its own accord.
fn replay(m: &Machine, file: &str) -> Result<(String, i32)> {
    let text = std::fs::read_to_string(file).map_err(|e| anyhow!("{file}: {e}"))?;
    let doc: toml::Value = toml::from_str(&text).map_err(|e| anyhow!("{file}: {e}"))?;
    let mut node = doc
        .get("start")
        .and_then(|s| s.as_str())
        .map(str::to_string)
        .or_else(|| m.start.first().cloned())
        .unwrap_or_default();
    let mut state = State::new();
    if let Some(t) = doc.get("state").and_then(|s| s.as_table()) {
        for (k, v) in t {
            let raw = v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string());
            state.insert(k.clone(), PathType::of(k).parse_value(&raw).map_err(|e| anyhow!(e))?);
        }
    }
    let mut out = String::new();
    let mut path = vec![node.clone()];
    let steps = doc.get("step").and_then(|s| s.as_array()).cloned().unwrap_or_default();
    for (i, step) in steps.iter().enumerate() {
        let name = step.get("edge").and_then(|e| e.as_str()).unwrap_or("");
        let (from, to) = name
            .split_once(" -> ")
            .ok_or_else(|| anyhow!("{file}: step {}: edge is `from -> to`", i + 1))?;
        if from != node {
            bail!("{file}: step {} leaves {from}, but the run is at {node}", i + 1);
        }
        let mut extra = Writes::new();
        if let Some(w) = step.get("writes").and_then(|w| w.as_table()) {
            for (k, v) in w {
                let raw = v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string());
                extra.insert(k.clone(), PathType::of(k).parse_value(&raw).map_err(|e| anyhow!(e))?);
            }
        }
        let mut done = None;
        for e in m.edges_from(&node).filter(|e| e.to == to) {
            match m.try_move(e, &state, &extra) {
                Outcome::Moved { writes, after } => {
                    let _ = writeln!(out, "{}   {}", e.name(), writes_text(e, &writes));
                    done = Some(after);
                    break;
                }
                Outcome::BadWrite { path: p, value } => {
                    let _ = writeln!(out, "{}", path.join(" -> "));
                    let _ = writeln!(
                        out,
                        "refused: {} would write {value} to {p}, which a {} cannot hold",
                        e.name(),
                        PathType::of(&p).name()
                    );
                    return Ok((out, 1));
                }
                Outcome::Blocked => {}
            }
        }
        let Some(after) = done else {
            bail!("{file}: step {} ({name}) is not possible from this state", i + 1);
        };
        state = after;
        node = to.to_string();
        path.push(node.clone());
    }
    let _ = writeln!(out, "{}", path.join(" -> "));
    Ok((out, 0))
}
