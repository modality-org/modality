//! `modal prove --houdini "q2 -> *" --seed domains.toml`: find an invariant.

use super::fmt::{summary_line, thousands, Names};
use super::project::Project;
use anyhow::{anyhow, Result};
use modality_lang::data::search;
use modality_lang::prove::houdini;
use std::fmt::Write as _;

pub fn run(p: &Project, names: &Names, anchor: &str, seed: Option<&str>, write: bool) -> Result<(String, i32)> {
    let node = anchor
        .split_once("->")
        .filter(|(_, to)| to.trim() == "*")
        .map(|(from, _)| from.trim().to_string())
        .ok_or_else(|| anyhow!("--houdini takes the moves out of a node: \"q2 -> *\""))?;
    if !p.machine.nodes().contains(&node) {
        return Err(anyhow!("{node} is not a node of model {}", p.model.name));
    }
    let seed = seed.ok_or_else(|| anyhow!("--houdini needs --seed <domain file>: its runs give the guesses"))?;
    let text = std::fs::read_to_string(seed).map_err(|e| anyhow!("{seed}: {e}"))?;
    let (domain, errors) = super::domain::parse(&text).map_err(|e| anyhow!("{seed}: {e}"))?;
    if let Some(e) = errors.first() {
        return Err(anyhow!("{seed}:{}:{}: {}", e.line, e.col, e.message));
    }
    let g = search::explore(&p.machine, &domain);
    let reached: Vec<_> = g.configs.iter().filter(|c| c.node == node).map(|c| c.state.clone()).collect();
    let mut prover = super::prove::prover(p, names)?;
    // Houdini looks for a fresh invariant at the node.
    prover.facts.retain(|f| !f.nodes.contains(&node));
    let r = houdini::run(&prover, &node, &reached);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "on {node} -> * · {} candidates true at every {node} state in {} bounded runs",
        r.candidates,
        thousands(g.runs())
    );
    let rounds = r.dropped.iter().map(|d| d.round).max().unwrap_or(0);
    for round in 1..=rounds {
        let _ = writeln!(out, "  round {round}");
        for d in r.dropped.iter().filter(|d| d.round == round) {
            let _ = writeln!(out, "    dropped  {:<34} {}", d.candidate.name, d.reason);
        }
    }
    let _ = writeln!(out, "  kept {}, inductive together", r.kept.len());
    if !r.strong {
        let _ = writeln!(out, "  ✗ not strong enough for the rules: the fact you need was never a guess");
        let _ = writeln!(out);
        let _ = writeln!(out, "{}", summary_line("no invariant found", 1));
        return Ok((out, 1));
    }
    let _ = writeln!(
        out,
        "  ✓ strong enough for the rules; {} of the {} are needed:",
        r.needed.len(),
        r.kept.len()
    );
    for c in &r.needed {
        let _ = writeln!(out, "    {}", c.name);
    }
    if write {
        let file = match &p.invariants {
            Some((i, _)) => p.sources[*i].path.clone(),
            None => std::path::PathBuf::from(format!(
                "{}.invariants.modality",
                p.model_source().name.trim_end_matches(".modality")
            )),
        };
        let mut text = std::fs::read_to_string(&file).unwrap_or_default();
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        let _ = writeln!(text, "\ninvariant suggested_{node} {{\n  on {node} -> *");
        for c in &r.needed {
            let _ = writeln!(text, "  formula {} {{\n    {}\n  }}", c.name, c.text);
        }
        let _ = writeln!(text, "}}");
        std::fs::write(&file, text)?;
        let _ = writeln!(out, "  wrote invariant suggested_{node} to {}", file.display());
    } else {
        let _ = writeln!(out, "  = write: add --write to save them as invariant suggested_{node}");
    }
    Ok((out, 0))
}
