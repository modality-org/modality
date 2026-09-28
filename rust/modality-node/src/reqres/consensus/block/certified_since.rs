use anyhow::Result;
use serde_json::{json, Value};

use modality_datastore::models::ValidatorBlock;
use modality_datastore::DatastoreManager;

use crate::reqres::Response;

pub const PATH: &str = "/consensus/block/certified_since";

/// Rounds scanned per request.
pub const MAX_SCAN_ROUNDS: u64 = 2000;

/// Certified blocks returned per request.
pub const MAX_BLOCKS: usize = 16;

/// Certified blocks by `author` after `since_round` that carry events.
///
/// Empty blocks change no contract state, so only blocks with events are
/// returned. `scanned_through` tells the caller where to resume.
pub async fn handler(data: Option<Value>, datastore_manager: &DatastoreManager) -> Result<Response> {
    let data = data.unwrap_or_default();
    let Some(author) = data.get("author").and_then(|v| v.as_str()) else {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({"error": "certified_since requires author"})),
        });
    };
    let since = data.get("since_round").and_then(|v| v.as_u64()).unwrap_or(0);
    let limit = data
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| (l as usize).clamp(1, MAX_BLOCKS))
        .unwrap_or(MAX_BLOCKS);

    let current_round = datastore_manager.get_current_round().await.unwrap_or(0);
    let end = current_round.min(since.saturating_add(MAX_SCAN_ROUNDS));
    let mut blocks = Vec::new();
    let mut scanned_through = since;
    let mut round = since + 1;
    while round <= end {
        if let Some(block) =
            ValidatorBlock::find_final_by_round_peer_multi(datastore_manager, round, author).await?
        {
            if block.cert.is_some() && !block.events.is_empty() {
                blocks.push(block);
            }
        }
        scanned_through = round;
        if blocks.len() >= limit {
            break;
        }
        round += 1;
    }

    Ok(Response {
        ok: true,
        data: Some(json!({
            "author": author,
            "current_round": current_round,
            "scanned_through": scanned_through,
            "blocks": blocks,
        })),
        errors: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn block(peer: &str, round: u64, cert: bool, events: Vec<Value>) -> ValidatorBlock {
        ValidatorBlock {
            peer_id: peer.into(),
            round_id: round,
            prev_round_certs: HashMap::new(),
            opening_sig: None,
            events,
            closing_sig: Some(format!("sig-{round}")),
            hash: None,
            acks: HashMap::new(),
            late_acks: Vec::new(),
            cert: cert.then(|| "[]".to_string()),
            is_section_leader: None,
            section_ending_block_id: None,
            section_starting_block_id: None,
            section_block_number: None,
            block_number: None,
            seen_at_block_id: None,
        }
    }

    #[tokio::test]
    async fn returns_certified_blocks_with_events_after_since() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let event = json!({"type": "contract_push"});
        for b in [
            block("a", 1, true, vec![event.clone()]),
            block("a", 2, true, vec![]),
            block("a", 3, true, vec![event.clone()]),
            block("b", 3, true, vec![event.clone()]),
            block("a", 4, true, vec![event.clone()]),
        ] {
            b.promote_to_final(&mgr).await.unwrap();
        }
        mgr.set_current_round(5).await.unwrap();

        let res = handler(Some(json!({"author": "a", "since_round": 1})), &mgr)
            .await
            .unwrap();
        let data = res.data.unwrap();
        let rounds: Vec<u64> = data["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["round_id"].as_u64().unwrap())
            .collect();
        assert_eq!(rounds, vec![3, 4]);
        assert_eq!(data["scanned_through"], 5);
        assert_eq!(data["current_round"], 5);
    }

    #[tokio::test]
    async fn stops_at_limit_and_reports_resume_round() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        for r in 1..=5 {
            block("a", r, true, vec![json!({"type": "contract_push"})])
                .promote_to_final(&mgr)
                .await
                .unwrap();
        }
        mgr.set_current_round(5).await.unwrap();

        let res = handler(
            Some(json!({"author": "a", "since_round": 0, "limit": 2})),
            &mgr,
        )
        .await
        .unwrap();
        let data = res.data.unwrap();
        assert_eq!(data["blocks"].as_array().unwrap().len(), 2);
        assert_eq!(data["scanned_through"], 2);
    }
}
