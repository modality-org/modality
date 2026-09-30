use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use modality_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Checkout state from commits to working directory")]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;

    checkout(&store)?;

    let state_files = store.list_state_files()?;
    let rules_files = store.list_rules_files()?;
    let total = state_files.len() + rules_files.len();

    println!("✅ Checked out {} file(s)", total);

    if !state_files.is_empty() {
        println!("   state/");
        for file in &state_files {
            println!("     {}", file);
        }
    }

    if !rules_files.is_empty() {
        println!("   rules/");
        for file in &rules_files {
            println!("     {}", file.trim_start_matches("/rules"));
        }
    }

    Ok(())
}

/// Write state, rules and the accepted model from the commits. What a
/// program wrote counts: each `invoke` is replaced by the actions it
/// emitted, as a validator applies it, so a copy of a program-driven
/// contract shows the state its rules read.
pub fn checkout(store: &ContractStore) -> Result<()> {
    store.checkout_state()?;
    if let Some(state) = program_state(store)? {
        for (path, value) in state {
            if !path.starts_with("/rules/") {
                store.write_working_path(&path, &value)?;
            }
        }
    }
    Ok(())
}

/// The contract's state after its commits, with what its programs wrote.
pub fn accepted_state(store: &ContractStore) -> Result<HashMap<String, Value>> {
    let mut state = store.build_state_from_commits()?;
    if let Some(expanded) = program_state(store)? {
        state.retain(|path, _| path.starts_with("/rules/"));
        state.extend(expanded);
    }
    Ok(state)
}

/// The state after the commits with each `invoke` expanded, or `None` when
/// no commit invokes a program (or this build cannot run one).
fn program_state(store: &ContractStore) -> Result<Option<serde_json::Map<String, Value>>> {
    #[cfg(all(feature = "wasm", feature = "model-status"))]
    {
        use modality_common::independent_replay::{
            accepted_state_from_commits, commit_has_invoke, expand_prefix,
            wasm_modules_from_commits,
        };
        if store.get_head()?.is_none() {
            return Ok(None);
        }
        let prefix = crate::replay::prefix_from_store(store)?;
        if !prefix.iter().any(|(_, file)| commit_has_invoke(file)) {
            return Ok(None);
        }
        let files: Vec<_> = prefix.iter().map(|(_, file)| file.clone()).collect();
        let wasm = wasm_modules_from_commits(&files)?;
        let contract_id = store.load_config()?.contract_id;
        let mut engine = crate::replay::CliWasmEngine;
        let expanded = expand_prefix(&contract_id, &prefix, &wasm, Some(&mut engine))?.0;
        Ok(Some(accepted_state_from_commits(&expanded)))
    }
    #[cfg(not(all(feature = "wasm", feature = "model-status")))]
    {
        let _ = store;
        Ok(None)
    }
}
