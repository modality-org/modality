use crate::reqres::Response;
use anyhow::Result;
use modal_datastore::NetworkDatastore;
use modal_sequencer_consensus::narwhal::{SyncRequest, SyncResponse};
use serde_json::Value;

/// Handler for DAG sync requests
/// Path: /dag/sync
/// 
/// This handler enables nodes to sync their DAG state by responding to sync requests.
/// It's designed to work with the Narwhal DAG used by the Shoal sequencer.
pub async fn handler(data: Option<Value>, _datastore: &mut NetworkDatastore) -> Result<Response> {
    let Some(data) = data else {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(serde_json::json!({"error": "Missing request data"})),
        });
    };
    
    // Deserialize the SyncRequest
    let sync_request: SyncRequest = match serde_json::from_value(data) {
        Ok(req) => req,
        Err(e) => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": format!("Invalid sync request: {}", e)})),
            });
        }
    };
    
    log::debug!("Handling DAG sync request: {:?}", sync_request);
    
    // TODO: Once ShoalSequencer is integrated into modal-node:
    // 1. Add a dag_sync_handler: Arc<Mutex<dyn DagSyncHandler>> to Node struct
    // 2. Pass it through to handle_request()
    // 3. Call dag_sync_handler.handle_sync_request(sync_request).await
    //
    // Example implementation:
    /*
    use modal_sequencer_consensus::narwhal::dag::DAG;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    // In Node struct:
    // pub dag: Option<Arc<RwLock<DAG>>>,
    
    // In this handler:
    if let Some(dag_ref) = dag_reference {
        let dag = dag_ref.read().await;
        let sync_response = dag.handle_sync_request(sync_request);
        
        let response_data = serde_json::to_value(&sync_response)?;
        
        return Ok(Response {
            ok: true,
            data: Some(response_data),
            errors: None,
        });
    }
    */
    
    // For now, return a placeholder indicating the feature is available but not configured
    Ok(Response {
        ok: false,
        data: None,
        errors: Some(serde_json::json!({
            "error": "DAG sync endpoint available but Shoal sequencer not yet integrated",
            "note": "This endpoint will be functional once ShoalSequencer replaces the current consensus runner"
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use modal_sequencer_consensus::narwhal::SyncRequest;
    
    #[tokio::test]
    async fn test_dag_sync_handler_receives_request() {
        let sync_req = SyncRequest::highest_round();
        let data = serde_json::to_value(&sync_req).unwrap();
        
        let mut datastore = modal_datastore::NetworkDatastore::create_in_memory().unwrap();
        let response = handler(Some(data), &mut datastore).await.unwrap();
        
        // Currently returns error since DAG not integrated
        assert!(!response.ok);
        assert!(response.errors.is_some());
    }
}

