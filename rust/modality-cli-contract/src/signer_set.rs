//! A contract's hash-lane signer set: the keys `modal contract create
//! --signer` fixes at creation, and the contract key's signature over them.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use modality_common::contract_store::ContractStore;

const FILE: &str = "signer_set.json";

/// What `create` saves in `.contract/signer_set.json`. The contract's own
/// key signed `signers` for `genesis_commit_id` and was then discarded, so
/// this is the only copy of `contract_signature`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerSet {
    pub genesis_commit_id: String,
    pub signers: Vec<String>,
    pub contract_signature: String,
}

impl SignerSet {
    pub fn load(store: &ContractStore) -> Result<Option<Self>> {
        let path = store.contract_dir().join(FILE);
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(Some(serde_json::from_str(&text).with_context(|| {
            format!("{} is unreadable", path.display())
        })?))
    }

    pub fn save(&self, store: &ContractStore) -> Result<()> {
        let path = store.contract_dir().join(FILE);
        std::fs::write(path, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }
}

/// A signer given as a Modality ID, or as a passfile (path or identity name)
/// whose public ID is used.
pub fn signer_id(reference: &str) -> Result<String> {
    if let Ok(path) = modality_common::passfile::resolve_passfile_path(reference) {
        if Path::new(&path).exists() {
            let keypair = modality_common::keypair::Keypair::from_json_file(
                path.to_str().unwrap_or_default(),
            )?;
            return Ok(keypair.public_key_as_base58_identity());
        }
    }
    modality_common::peer_id::canonical_peer_id(reference)
        .and_then(|id| modality_common::keypair::Keypair::from_public_key(&id, "ed25519"))
        .map(|k| k.public_key_as_base58_identity())
        .map_err(|_| {
            anyhow::anyhow!("--signer {reference} is neither a passfile nor a Modality ID")
        })
}
