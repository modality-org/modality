//! Probe for `dev/potential-issues/14-signatures-not-verified`. Run it with `../run-probe.sh 14-signatures-not-verified`.
//! crate: modality-validator

use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;
use modality_validator::contract_processor::ContractProcessor;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

const MODEL: &str = r#"
model M {
  part p {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
  }
}"#;

/// Sequenced apply, as the validator runs it: `process_commit`, then mark
/// the commit sequenced so the next one can name it as parent.
async fn apply(
    processor: &ContractProcessor,
    ds: &Arc<Mutex<DatastoreManager>>,
    id: &str,
    data: serde_json::Value,
) -> Result<(), String> {
    processor
        .process_commit("c1", id, &data.to_string())
        .await
        .map_err(|e| e.to_string())?;
    let ds = ds.lock().await;
    let keys = [
        ("contract_id".to_string(), "c1".to_string()),
        ("commit_id".to_string(), id.to_string()),
    ]
    .into_iter()
    .collect();
    let mut commit = Commit::find_one_multi(&ds, keys).await.unwrap().unwrap();
    commit.in_batch = Some(format!("batch-{id}"));
    commit.save_to_final(&ds).await.unwrap();
    Ok(())
}

/// Alice's key is a real ed25519 public key, and everyone can read it in
/// state. Governance takes the signer set from the keys of
/// `head.signatures` and never checks a signature; nothing on the
/// submit/push → sequencer → validator path does either (the hub checks at
/// its own ingress). So anyone can sign as Alice.
#[tokio::test]
async fn a_commit_with_a_forged_signature_is_refused() {
    let (_alice_secret, alice_public) = modality_lang::crypto::generate_keypair();
    let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
    let processor = ContractProcessor::new(ds.clone());
    apply(
        &processor,
        &ds,
        "bootstrap",
        json!({
            "body": [
                {"method": "post", "path": "/parties/alice.id", "value": alice_public},
                {"method": "model", "path": "/model/default.modality", "value": MODEL}
            ],
            "head": {}
        }),
    )
    .await
    .expect("bootstrap");

    let forged = json!({
        "body": [{"method": "post", "path": "/notes/forged.text", "value": "not alice"}],
        "head": {
            "parent": "bootstrap",
            "signatures": { alice_public.clone(): "00" }
        }
    });
    let result = apply(&processor, &ds, "forged", forged).await;
    eprintln!("commit claiming Alice's key with signature \"00\": {result:?}");
    assert!(result.is_err(), "a commit with a forged signature for Alice was applied");
}
