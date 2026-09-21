use anyhow::Result;
use modality_common::independent_replay::{
    artifact_from_prefix, parse_commit_file, sequenced_tips, ReplayArtifact,
};
use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::reqres::Response;
use modality_validator_consensus::communication::Message as ConsensusMessage;

#[derive(Serialize, Deserialize, Debug)]
pub struct ReplayRequest {
    pub contract_id: String,
    pub through_commit: Option<String>,
}

pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: ReplayRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    match export_replay_artifact(
        datastore_manager,
        &req.contract_id,
        req.through_commit.as_deref(),
    )
    .await
    {
        Ok(artifact) => Ok(Response {
            ok: true,
            data: Some(serde_json::to_value(artifact)?),
            errors: None,
        }),
        Err(err) => Ok(Response {
            ok: false,
            data: None,
            errors: Some(serde_json::json!({ "error": err.to_string() })),
        }),
    }
}

pub async fn export_replay_artifact(
    datastore_manager: &DatastoreManager,
    contract_id: &str,
    through_commit: Option<&str>,
) -> Result<ReplayArtifact> {
    let mut sequenced = Vec::new();
    for commit in Commit::find_by_contract_multi(datastore_manager, contract_id).await? {
        if !commit.is_sequenced() {
            continue;
        }
        sequenced.push((
            commit.commit_id.clone(),
            parse_commit_file(&commit.commit_data)?,
        ));
    }
    if sequenced.is_empty() {
        anyhow::bail!("no sequenced commits for contract {contract_id}");
    }

    let through = if let Some(through) = through_commit {
        through.to_string()
    } else {
        let tips = sequenced_tips(&sequenced);
        match tips.as_slice() {
            [only] => only.clone(),
            [] => anyhow::bail!("no sequenced tip for contract {contract_id}"),
            _ => anyhow::bail!("multiple sequenced tips {:?}; pass through_commit", tips),
        }
    };

    let by_id: std::collections::HashMap<_, _> = sequenced.iter().cloned().collect();
    let mut prefix = Vec::new();
    let mut current = Some(through.clone());
    let mut seen = std::collections::HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            anyhow::bail!("cycle in sequenced parent chain at {id}");
        }
        let file = by_id
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("through_commit '{id}' has not been sequenced"))?
            .clone();
        current = file.head.parent.clone().filter(|parent| !parent.is_empty());
        prefix.push((id, file));
    }
    prefix.reverse();
    artifact_from_prefix(contract_id, &through, &prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn replay_exports_only_sequenced_prefix() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(100);

        Commit {
            contract_id: "c1".to_string(),
            commit_id: "g".to_string(),
            commit_data: json!({
                "body": [{ "method": "post", "path": "/notes/a.text", "value": "a" }],
                "head": {}
            })
            .to_string(),
            timestamp: 1,
            in_batch: Some("b1".to_string()),
        }
        .save_to_final(&mgr)
        .await
        .unwrap();
        Commit {
            contract_id: "c1".to_string(),
            commit_id: "queued".to_string(),
            commit_data: json!({
                "body": [{ "method": "post", "path": "/notes/nope.text", "value": "x" }],
                "head": { "parent": "g" }
            })
            .to_string(),
            timestamp: 2,
            in_batch: None,
        }
        .save_to_final(&mgr)
        .await
        .unwrap();

        let response = handler(
            Some(json!({ "contract_id": "c1", "through_commit": "g" })),
            &mgr,
            _tx,
        )
        .await
        .unwrap();
        assert!(response.ok);
        let data = response.data.unwrap();
        assert_eq!(data["through_commit"], "g");
        assert_eq!(data["commits"].as_array().unwrap().len(), 1);
        assert_eq!(data["commits"][0]["commit_id"], "g");
        assert_eq!(data["type"], "modality_replay_artifact");
    }
}
