use anyhow::Result;
use serde_json::Value;
use tokio::sync::mpsc;

use modality_datastore::DatastoreManager;

use crate::explorer;
use crate::reqres::Response;
use modality_validator_consensus::communication::Message as ConsensusMessage;

pub async fn handler(
    _data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let contracts = explorer::list_contracts(datastore_manager).await?;
    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(contracts)?),
        errors: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_datastore::models::Commit;
    use serde_json::json;

    #[tokio::test]
    async fn catalog_lists_contract_ids() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(8);
        Commit {
            contract_id: "c1".into(),
            commit_id: "g".into(),
            commit_data: json!({"body": [], "head": {}}).to_string(),
            timestamp: 1,
            in_batch: Some("b1".into()),
        }
        .save_to_final(&mgr)
        .await
        .unwrap();
        let response = handler(None, &mgr, _tx).await.unwrap();
        assert!(response.ok);
        let contracts = response.data.unwrap();
        assert_eq!(contracts[0]["contract_id"], "c1");
        assert_eq!(contracts[0]["commits_sequenced"], 1);
    }
}
