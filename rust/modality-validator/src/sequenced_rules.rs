use anyhow::Result;
use modality_common::contract_store::CommitFile;
use modality_common::model_governance::TheoryActivation;
use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;
use modality_lang::TheoryVersion;
use std::collections::{HashMap, HashSet};

pub use modality_common::independent_replay::parse_commit_file;

pub async fn load_sequenced_parent_chain(
    ds: &DatastoreManager,
    contract_id: &str,
    parent: Option<&str>,
) -> Result<Vec<(String, CommitFile)>> {
    let mut by_id = HashMap::new();
    for commit in Commit::find_by_contract_multi(ds, contract_id).await? {
        if !commit.is_sequenced() {
            continue;
        }
        by_id.insert(
            commit.commit_id.clone(),
            parse_commit_file(&commit.commit_data)?,
        );
    }

    let mut chain = Vec::new();
    let mut current = parent.map(str::to_string);
    let mut seen = HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            anyhow::bail!("cycle in sequenced parent chain at {id}");
        }
        let file = by_id
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("parent commit '{id}' has not been sequenced"))?;
        chain.push((id.clone(), file.clone()));
        current = file.head.parent.clone();
    }
    chain.reverse();
    Ok(chain)
}

/// A contract is one log: a commit may follow `parent` only while no other
/// sequenced commit does. A second child would be checked, and would run
/// its programs, against a history that leaves out the first.
pub async fn assert_extends_head(
    ds: &DatastoreManager,
    contract_id: &str,
    commit_id: &str,
    parent: Option<&str>,
) -> Result<()> {
    for commit in Commit::find_by_contract_multi(ds, contract_id).await? {
        if !commit.is_sequenced() || commit.commit_id == commit_id {
            continue;
        }
        if parse_commit_file(&commit.commit_data)?
            .head
            .parent
            .as_deref()
            == parent
        {
            anyhow::bail!(
                "commit '{}' forks the contract: sequenced commit '{}' already follows {}; commit again on the current head",
                commit_id,
                commit.commit_id,
                parent.map_or("the genesis".to_string(), |p| format!("'{p}'"))
            );
        }
    }
    Ok(())
}

pub fn validate_against_local_rules(accepted: &[CommitFile], pending: &CommitFile) -> Result<()> {
    modality_common::model_governance::validate_sequenced_commit(accepted, pending)
}

pub fn validate_against_local_rules_for_commit(
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: &str,
    contract_id: &str,
) -> Result<()> {
    validate_against_local_rules_for_commit_at(
        accepted,
        pending,
        pending_commit_id,
        contract_id,
        None,
        TheoryActivation::V0,
    )
}

pub fn validate_against_local_rules_for_commit_at(
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: &str,
    contract_id: &str,
    evaluation_timestamp: Option<u64>,
    theory: TheoryActivation,
) -> Result<()> {
    modality_common::model_governance::validate_sequenced_commit_with_theory(
        accepted,
        pending,
        Some(pending_commit_id),
        Some(contract_id),
        evaluation_timestamp,
        theory,
    )
}

/// The predicate theory this network's validators enforce, in force from
/// network genesis. A version this build does not know is an error, not a
/// fallback: judged under `V0`, the commit could be accepted here and refused
/// by every upgraded peer. `v1` is refused too: it accepts rules that some
/// runs of the model break.
pub fn network_theory(ds: &DatastoreManager) -> Result<TheoryActivation> {
    let raw = ds.predicate_theory_version()?;
    let version: TheoryVersion = raw.parse().map_err(|err: String| {
        anyhow::anyhow!("network parameter predicate_theory_version: {err}; upgrade this node")
    })?;
    if version == TheoryVersion::V1 {
        anyhow::bail!(
            "network parameter predicate_theory_version: v1 is withdrawn, since it accepts rules that some runs of the model break; use v2"
        );
    }
    Ok(TheoryActivation::always(version))
}
