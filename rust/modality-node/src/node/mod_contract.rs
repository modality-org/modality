//! The network's MOD contract. Its genesis commits come with the network
//! config; every node applies them at startup through the contract processor,
//! as it would sequenced commits, so the checker accepts the model and rules
//! before anything reads the contract. The network's emission is what the
//! contract posts under `/network/emission`.

use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use modality_common::contract_store::CommitFile;
use modality_datastore::models::{Commit, Contract, ContractAsset};
use modality_datastore::{DatastoreManager, EmissionConfig};
use modality_validator::ContractProcessor;

pub const ASSET: &str = "MOD";
const GENESIS_BATCH: &str = "genesis";

/// Apply the MOD contract's genesis named in `network_config`, if any, and take
/// the network's emission from it. A node that already applied it skips the
/// commits it holds, so a restart re-reads the same contract.
pub async fn apply_genesis(
    datastore: &Arc<Mutex<DatastoreManager>>,
    network_config: &Value,
) -> Result<()> {
    let Some(genesis) = network_config.get("mod_contract") else {
        return Ok(());
    };
    if network_config.get("emission").is_some() {
        bail!("the network config names both mod_contract and emission; the MOD contract's /network/emission posts are the emission");
    }
    let contract_id = genesis
        .get("contract_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("mod_contract needs a contract_id"))?;
    let commits = genesis
        .get("commits")
        .and_then(Value::as_array)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| anyhow!("mod_contract needs its genesis commits"))?;

    {
        let mgr = datastore.lock().await;
        let theory = mgr.predicate_theory_version()?;
        if theory_number(&theory) < 2 {
            bail!("the MOD contract needs predicate theory v2, which verifies commit signatures; this network runs {theory}");
        }
        if let Some(held) = mgr.mod_contract_id()? {
            if held != contract_id {
                bail!("this data dir holds MOD contract {held}, not {contract_id}; clear its storage to join this network");
            }
        }
        if Contract::find_by_id_multi(&mgr, contract_id).await?.is_none() {
            Contract {
                contract_id: contract_id.to_string(),
                genesis: commits[0].to_string(),
                created_at: 0,
            }
            .save_to_final(&mgr)
            .await?;
        }
    }

    let processor = ContractProcessor::new(datastore.clone());
    let mut parent: Option<String> = None;
    let mut applied = 0usize;
    for (n, entry) in commits.iter().enumerate() {
        let commit_id = entry
            .get("commit_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("MOD genesis commit {n} has no commit_id"))?;
        let file: CommitFile = serde_json::from_value(serde_json::json!({
            "body": entry.get("body"),
            "head": entry.get("head"),
        }))?;
        let computed = file.compute_id()?;
        if computed != commit_id {
            bail!("MOD genesis commit {n} is named {commit_id}, but its body and head hash to {computed}");
        }
        if file.head.parent != parent {
            bail!("MOD genesis commit {commit_id} does not follow the one before it");
        }
        parent = Some(commit_id.to_string());
        if sequenced(datastore, contract_id, commit_id).await? {
            continue;
        }
        processor
            .process_commit(contract_id, commit_id, &serde_json::to_string(&file)?)
            .await
            .map_err(|e| anyhow!("MOD genesis commit {commit_id} is refused: {e}"))?;
        let mgr = datastore.lock().await;
        let mut commit = find(&mgr, contract_id, commit_id)
            .await?
            .ok_or_else(|| anyhow!("MOD genesis commit {commit_id} was not saved"))?;
        commit.in_batch = Some(GENESIS_BATCH.to_string());
        commit.save_to_final(&mgr).await?;
        applied += 1;
    }

    let mgr = datastore.lock().await;
    let keys = [
        ("contract_id".to_string(), contract_id.to_string()),
        ("asset_id".to_string(), ASSET.to_string()),
    ]
    .into_iter()
    .collect();
    let asset = ContractAsset::find_one_multi(&mgr, keys)
        .await?
        .ok_or_else(|| anyhow!("the MOD contract's genesis creates no {ASSET}"))?;
    let emission = emission_from_state(&mgr, contract_id).await?;
    mgr.set_mod_contract(contract_id, &emission)?;
    log::info!(
        "MOD contract {}: {} of {} genesis commits applied; {} {} at divisibility {}",
        contract_id,
        applied,
        commits.len(),
        asset.quantity,
        ASSET,
        asset.divisibility
    );
    log::info!(
        "Network emission from the MOD contract: {}",
        serde_json::to_string(&emission)?
    );
    Ok(())
}

/// A client commit to the MOD contract. Only its genesis and, later, the
/// network's own mint commits write it.
pub fn refusal(mgr: &DatastoreManager, contract_id: &str) -> Option<String> {
    match mgr.mod_contract_id() {
        Ok(Some(id)) if id == contract_id => Some(format!(
            "contract {contract_id} is the network's MOD contract; only the network writes it"
        )),
        _ => None,
    }
}

async fn emission_from_state(mgr: &DatastoreManager, contract_id: &str) -> Result<EmissionConfig> {
    let read = |name: &'static str| async move {
        let key = format!("/contracts/{contract_id}/network/emission/{name}.num");
        match mgr.get_data_by_key(&key).await? {
            None => Ok::<u64, anyhow::Error>(0),
            Some(raw) => {
                let text = String::from_utf8_lossy(&raw);
                text.trim_matches('"')
                    .parse()
                    .map_err(|_| anyhow!("{key} is {text}, not a whole number"))
            }
        }
    };
    Ok(EmissionConfig {
        block_subsidy: read("block_subsidy").await?,
        halving_interval_blocks: read("halving_interval").await?,
        slow_start_blocks: read("slow_start").await?,
        cap: read("cap").await?,
        genesis_allocations: Vec::new(),
    })
}

async fn find(mgr: &DatastoreManager, contract_id: &str, commit_id: &str) -> Result<Option<Commit>> {
    let keys = [
        ("contract_id".to_string(), contract_id.to_string()),
        ("commit_id".to_string(), commit_id.to_string()),
    ]
    .into_iter()
    .collect();
    Commit::find_one_multi(mgr, keys).await
}

async fn sequenced(
    datastore: &Arc<Mutex<DatastoreManager>>,
    contract_id: &str,
    commit_id: &str,
) -> Result<bool> {
    let mgr = datastore.lock().await;
    Ok(find(&mgr, contract_id, commit_id)
        .await?
        .is_some_and(|c| c.is_sequenced()))
}

fn theory_number(version: &str) -> u32 {
    version
        .strip_prefix('v')
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}
