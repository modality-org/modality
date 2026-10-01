//! What a contract holds and what waits for it: the account a wallet reads.
//! Read-only.

use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use modality_datastore::models::{AssetBalance, ContractAsset, ReceivedSend, SendRecord};
use modality_datastore::DatastoreManager;

use crate::reqres::Response;
use modality_sequencer_consensus::communication::Message as ConsensusMessage;

/// `{"contract_id"}` → `{contract_id, mod_contract_id, holdings, incoming}`.
/// `holdings` are the assets the contract holds, with each asset's creator,
/// divisibility (amounts are multiples of it) and display decimals. `incoming` are the applied `SEND`s to it that no `RECV`
/// has taken yet, with what a `RECV` must state.
pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let Some(contract_id) = data
        .as_ref()
        .and_then(|d| d.get("contract_id"))
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
    else {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({"error": "contract_id is required"})),
        });
    };
    Ok(Response {
        ok: true,
        data: Some(account(datastore_manager, contract_id).await?),
        errors: None,
    })
}

pub async fn account(ds: &DatastoreManager, contract_id: &str) -> Result<Value> {
    let mut holdings = Vec::new();
    for balance in AssetBalance::find_by_owner_multi(ds, contract_id).await? {
        if balance.balance == 0 {
            continue;
        }
        let keys = [
            ("contract_id".to_string(), balance.contract_id.clone()),
            ("asset_id".to_string(), balance.asset_id.clone()),
        ]
        .into_iter()
        .collect();
        let asset = ContractAsset::find_one_multi(ds, keys).await?;
        holdings.push(json!({
            "asset_contract": balance.contract_id,
            "asset_id": balance.asset_id,
            "balance": balance.balance,
            "divisibility": asset.as_ref().map(|a| a.divisibility),
            "decimals": asset.as_ref().and_then(|a| a.decimals),
        }));
    }

    let mut incoming = Vec::new();
    for send in SendRecord::find_to_multi(ds, contract_id).await? {
        if ReceivedSend::is_received(ds, &send.send_commit_id, send.send_index).await? {
            continue;
        }
        let keys = [
            ("contract_id".to_string(), send.creator().to_string()),
            ("asset_id".to_string(), send.asset_id.clone()),
        ]
        .into_iter()
        .collect();
        let decimals = ContractAsset::find_one_multi(ds, keys)
            .await?
            .and_then(|a| a.decimals);
        incoming.push(json!({
            "decimals": decimals,
            "send_commit_id": send.send_commit_id,
            "send_index": send.send_index,
            "from_contract": send.from_contract,
            "asset_contract": send.creator(),
            "asset_id": send.asset_id,
            "amount": send.amount,
            "memo": send.memo,
        }));
    }

    Ok(json!({
        "contract_id": contract_id,
        "mod_contract_id": ds.mod_contract_id()?,
        "holdings": holdings,
        "incoming": incoming,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_account_shows_holdings_and_unreceived_sends() {
        let ds = DatastoreManager::create_in_memory().unwrap();
        ContractAsset {
            contract_id: "bank".into(),
            asset_id: "tok".into(),
            quantity: 100,
            divisibility: 1,
            created_at: 1,
            creator_commit_id: "c0".into(),
            decimals: Some(2),
        }
        .save_to_final(&ds)
        .await
        .unwrap();
        AssetBalance {
            contract_id: "bank".into(),
            asset_id: "tok".into(),
            owner_contract_id: "alice".into(),
            balance: 7,
        }
        .save_to_final(&ds)
        .await
        .unwrap();
        for (commit, index) in [("s1", 0), ("s2", 0)] {
            SendRecord {
                send_commit_id: commit.into(),
                send_index: index,
                from_contract: "bank".into(),
                asset_id: "tok".into(),
                to_contract: "alice".into(),
                amount: 5,
                asset_contract: String::new(),
                memo: None,
            }
            .save_to_final(&ds)
            .await
            .unwrap();
        }
        ReceivedSend {
            send_commit_id: ReceivedSend::key_for("s1", 0),
            recv_contract_id: "alice".into(),
            recv_commit_id: "r1".into(),
            received_at: 1,
        }
        .save_to_final(&ds)
        .await
        .unwrap();

        let (tx, _rx) = mpsc::channel::<ConsensusMessage>(8);
        let response = handler(Some(json!({"contract_id": "alice"})), &ds, tx.clone())
            .await
            .unwrap();
        assert!(response.ok);
        let data = response.data.unwrap();
        assert_eq!(data["holdings"][0]["asset_contract"], "bank");
        assert_eq!(data["holdings"][0]["balance"], 7);
        assert_eq!(data["holdings"][0]["divisibility"], 1);
        assert_eq!(data["holdings"][0]["decimals"], 2);
        let incoming = data["incoming"].as_array().unwrap();
        assert_eq!(incoming.len(), 1, "s1 was received");
        assert_eq!(incoming[0]["send_commit_id"], "s2");
        assert_eq!(incoming[0]["asset_contract"], "bank");
        assert_eq!(incoming[0]["decimals"], 2);
        assert!(data["mod_contract_id"].is_null());

        let refused = handler(Some(json!({})), &ds, tx).await.unwrap();
        assert!(!refused.ok);
    }
}
