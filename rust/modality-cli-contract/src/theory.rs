use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    about = "Show what the predicate theory derives from a contract: dead edges and open, blocked, or forced moves"
)]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Predicate theory version to preview (v0, v1, v2, or v3; networks refuse v1)
    #[clap(long, default_value = "v3")]
    theory: String,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

#[cfg(feature = "model-status")]
pub async fn run(opts: &Opts) -> Result<()> {
    use modality_common::contract_store::ContractStore;
    use modality_common::model_governance::{
        derived_view_for_store, format_properties, TheoryActivation,
    };
    use modality_lang::MoveStatus;

    let dir = match &opts.dir {
        Some(d) => d.clone(),
        None => std::env::current_dir()?,
    };
    let store = ContractStore::open(&dir)?;
    let model_path = dir.join("model").join("default.modality");
    let fallback = if model_path.exists() {
        std::fs::read_to_string(&model_path)?
    } else {
        String::new()
    };
    let theory: modality_lang::TheoryVersion =
        opts.theory.parse().map_err(|err: String| anyhow::anyhow!(err))?;
    let view = derived_view_for_store(&fallback, &store, TheoryActivation::always(theory))?;

    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&view)?);
        return Ok(());
    }

    println!(
        "Predicate theory {} (preview; validators enforce the network's predicate_theory_version, V0 unless it names one)",
        view.theory
    );
    println!();
    println!("  Current state: {}", view.current_states.join(", "));

    println!();
    if view.dead_edges.is_empty() {
        println!("  ✅ No dead edges.");
    } else {
        println!("  ⚠️  Dead edges (no commit can take them):");
        for edge in &view.dead_edges {
            println!(
                "     {}: {} --> {} [{}]",
                edge.part_name,
                edge.from,
                edge.to,
                format_properties(&edge.properties)
            );
            println!("       cannot hold together: {}", edge.offending.join(", "));
        }
    }

    if !view.dead_after_step.is_empty() {
        println!();
        println!(
            "  ⚠️  Dead after a step (modality/dead-end-after-step; a warning, contracts may end):"
        );
        for edge in &view.dead_after_step {
            println!(
                "     {}: {} --> {} [{}]",
                edge.part_name,
                edge.from,
                edge.to,
                format_properties(&edge.properties)
            );
            println!(
                "       what arrives at {} contradicts it: {}",
                edge.from,
                edge.offending.join(", ")
            );
        }
    }

    if !view.unparsed_declarations.is_empty() {
        println!();
        println!("  ⚠️  Declarations the theory cannot read (their predicates stay opaque):");
        for module in &view.unparsed_declarations {
            println!("     {module}");
        }
    }

    println!();
    if view.moves.is_empty() {
        println!("  No moves out of the current state.");
    } else {
        println!("  Moves from the current state:");
        for m in &view.moves {
            let status = match m.status {
                MoveStatus::Open => "open   ",
                MoveStatus::Blocked => "blocked",
                MoveStatus::Forced => "forced ",
            };
            println!(
                "     {status} {}: {} --> {} [{}]",
                m.part_name,
                m.from,
                m.to,
                format_properties(&m.properties)
            );
            if !m.offending.is_empty() {
                println!("             false now: {}", m.offending.join(", "));
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "model-status"))]
pub async fn run(_opts: &Opts) -> Result<()> {
    anyhow::bail!("`modal contract theory` needs a build with the model-status feature")
}