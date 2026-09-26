use anyhow::Result;
use modality_common::contract_store::CommitFile;
use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;
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

pub fn validate_against_local_rules(accepted: &[CommitFile], pending: &CommitFile) -> Result<()> {
    modality_common::model_governance::validate_sequenced_commit(accepted, pending)
}

pub fn validate_against_local_rules_for_commit(
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: &str,
    contract_id: &str,
) -> Result<()> {
    modality_common::model_governance::validate_sequenced_commit_with_ids(
        accepted,
        pending,
        Some(pending_commit_id),
        Some(contract_id),
    )
}
