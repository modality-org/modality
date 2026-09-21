use anyhow::Result;
use modality_common::keypair::{Keypair, KeypairOrPublicKey};
use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;
use serde::{Deserialize, Serialize};

pub use modality_common::independent_replay::prefix_digest;

pub const PREFIX_CERT_TYPE: &str = "prefix_cert";
pub const GAS_PER_COMMIT: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrefixCert {
    #[serde(rename = "type")]
    pub event_type: String,
    pub source_contract: String,
    pub through_commit: String,
    pub prefix_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    pub validator_peer_id: String,
    pub gas_used: u64,
    pub fee_quoted: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requester_peer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl PrefixCert {
    pub fn as_event(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}

pub fn parent_of_commit_data(commit_data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(commit_data).ok()?;
    v.get("head")
        .and_then(|h| h.get("parent").or_else(|| h.get("prev")))
        .and_then(|p| p.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Walk genesis → `through_commit` using `head.parent` links.
pub fn commit_prefix_ids(commits: &[Commit], through_commit: &str) -> Result<Vec<String>> {
    let by_id: std::collections::HashMap<_, _> =
        commits.iter().map(|c| (c.commit_id.clone(), c)).collect();
    if !by_id.contains_key(through_commit) {
        anyhow::bail!(
            "prefix cert: through_commit '{}' not found on contract",
            through_commit
        );
    }
    let mut walked = Vec::new();
    let mut current = Some(through_commit.to_string());
    let mut guard = 0usize;
    while let Some(id) = current {
        guard += 1;
        if guard > 10_000 {
            anyhow::bail!("prefix cert: commit parent chain too long or cyclic");
        }
        let commit = by_id
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("prefix cert: missing commit {}", id))?;
        walked.push(id.clone());
        current = parent_of_commit_data(&commit.commit_data);
        if current.as_ref().map(|p| p == &id).unwrap_or(false) {
            anyhow::bail!("prefix cert: commit {} parents itself", id);
        }
    }
    walked.reverse();
    Ok(walked)
}

pub fn signing_payload(cert: &PrefixCert) -> serde_json::Value {
    let mut v = serde_json::to_value(cert).unwrap_or(serde_json::json!({}));
    if let Some(obj) = v.as_object_mut() {
        obj.remove("signature");
    }
    v
}

pub fn peer_id_of(keypair: &Keypair) -> String {
    match &keypair.inner {
        KeypairOrPublicKey::Keypair(k) => k.public().to_peer_id().to_string(),
        KeypairOrPublicKey::PublicKey(pk) => pk.to_peer_id().to_string(),
    }
}

pub fn sign_cert(cert: &mut PrefixCert, keypair: &Keypair) -> Result<()> {
    cert.signature = None;
    let payload = signing_payload(cert);
    cert.signature = Some(keypair.sign_json(&payload)?);
    Ok(())
}

pub fn verify_cert_signature(cert: &PrefixCert) -> Result<bool> {
    let Some(sig) = cert.signature.as_ref() else {
        return Ok(false);
    };
    let verifier = Keypair::from_public_key(&cert.validator_peer_id, "ed25519")?;
    let payload = signing_payload(cert);
    verifier.verify_json(sig, &payload)
}

pub fn signer_is_named(cert: &PrefixCert, contract_validators: &[String]) -> bool {
    contract_validators
        .iter()
        .any(|id| id == &cert.validator_peer_id)
}

/// Cheap inclusion check: well-formed, named signer, signature. No model replay.
pub fn cheap_include_prefix_cert(
    event: &serde_json::Value,
    contract_validators: &[String],
) -> Result<()> {
    let cert: PrefixCert = serde_json::from_value(event.clone())?;
    if cert.event_type != PREFIX_CERT_TYPE {
        anyhow::bail!("not a prefix_cert");
    }
    if contract_validators.is_empty() || !signer_is_named(&cert, contract_validators) {
        anyhow::bail!(
            "prefix_cert signer {} is not a named contract validator",
            cert.validator_peer_id
        );
    }
    if !verify_cert_signature(&cert)? {
        anyhow::bail!("prefix_cert signature invalid");
    }
    Ok(())
}

pub fn filter_includable_events(
    events: Vec<serde_json::Value>,
    contract_validators: &[String],
) -> Vec<serde_json::Value> {
    events
        .into_iter()
        .filter(|event| match event.get("type").and_then(|v| v.as_str()) {
            Some(PREFIX_CERT_TYPE) => cheap_include_prefix_cert(event, contract_validators).is_ok(),
            _ => true,
        })
        .collect()
}

/// Certs before contract_push so dest REPOST in the same batch can see the cert.
pub fn canonical_event_order(events: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut certs = Vec::new();
    let mut rest = Vec::new();
    for e in events {
        if e.get("type").and_then(|v| v.as_str()) == Some(PREFIX_CERT_TYPE) {
            certs.push(e.clone());
        } else {
            rest.push(e.clone());
        }
    }
    certs.extend(rest);
    certs
}

pub async fn build_prefix_from_store(
    ds: &DatastoreManager,
    source_contract: &str,
    through_commit: &str,
) -> Result<(Vec<String>, String, u64)> {
    let commits = Commit::find_by_contract_multi(ds, source_contract).await?;
    let ids = commit_prefix_ids(&commits, through_commit)?;
    let digest = prefix_digest(&ids);
    let gas_used = (ids.len() as u64).saturating_mul(GAS_PER_COMMIT);
    Ok((ids, digest, gas_used))
}

pub fn cert_matches_source(
    cert: &serde_json::Value,
    source_contract: &str,
    source_commit: &str,
    source_path: Option<&str>,
    value: Option<&serde_json::Value>,
) -> bool {
    let Ok(cert) = serde_json::from_value::<PrefixCert>(cert.clone()) else {
        return false;
    };
    if cert.source_contract != source_contract || cert.through_commit != source_commit {
        return false;
    }
    if let Some(path) = source_path {
        if let Some(cert_path) = cert.source_path.as_deref() {
            if cert_path != path {
                return false;
            }
        }
    }
    if let (Some(expected), Some(got)) = (value, cert.value.as_ref()) {
        return modality_common::contract_store::json_values_equal(expected, got);
    }
    true
}

pub fn cert_matches_repost(
    cert: &serde_json::Value,
    source_contract: &str,
    source_commit: &str,
    source_path: Option<&str>,
    value: Option<&serde_json::Value>,
) -> bool {
    cert_matches_source(cert, source_contract, source_commit, source_path, value)
}

/// ⌈n * numerator / denominator⌉, capped at n. `n = 0` cannot form a QC.
pub fn qc_threshold(n: usize, numerator: u64, denominator: u64) -> usize {
    if n == 0 {
        return 0;
    }
    let d = denominator.max(1);
    let num = if numerator == 0 {
        modality_datastore::VALIDATOR_QC_NUMERATOR
    } else {
        numerator
    };
    let t = (n as u64).saturating_mul(num).div_ceil(d) as usize;
    t.min(n)
}

/// Distinct named signers whose cert matches dest's pin and the rebuilt digest.
pub fn matching_qc_signers(
    certs: &[serde_json::Value],
    contract_validators: &[String],
    expected_digest: &str,
    source_contract: &str,
    source_commit: &str,
    source_path: Option<&str>,
    value: Option<&serde_json::Value>,
) -> usize {
    let mut seen = std::collections::HashSet::new();
    for cert_json in certs {
        let Ok(cert) = serde_json::from_value::<PrefixCert>(cert_json.clone()) else {
            continue;
        };
        if cert.prefix_digest != expected_digest {
            continue;
        }
        if !signer_is_named(&cert, contract_validators) {
            continue;
        }
        if !cert_matches_source(
            cert_json,
            source_contract,
            source_commit,
            source_path,
            value,
        ) {
            continue;
        }
        seen.insert(cert.validator_peer_id);
    }
    seen.len()
}

pub fn is_prefix_cert_event(
    event: &serde_json::Value,
    source_contract: &str,
    through_commit: &str,
    peer_id: &str,
) -> bool {
    event.get("type").and_then(|v| v.as_str()) == Some(PREFIX_CERT_TYPE)
        && event.get("source_contract").and_then(|v| v.as_str()) == Some(source_contract)
        && event.get("through_commit").and_then(|v| v.as_str()) == Some(through_commit)
        && event.get("validator_peer_id").and_then(|v| v.as_str()) == Some(peer_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_common::keypair::Keypair;

    fn sample_commit(id: &str, parent: Option<&str>) -> Commit {
        let head = match parent {
            Some(p) => serde_json::json!({"parent": p}),
            None => serde_json::json!({}),
        };
        Commit {
            contract_id: "src".to_string(),
            commit_id: id.to_string(),
            commit_data: serde_json::json!({"body": [], "head": head}).to_string(),
            timestamp: 1,
            in_batch: None,
        }
    }

    #[test]
    fn prefix_walks_parent_chain() {
        let commits = vec![
            sample_commit("c2", Some("c1")),
            sample_commit("c1", None),
            sample_commit("c3", Some("c2")),
        ];
        let ids = commit_prefix_ids(&commits, "c2").unwrap();
        assert_eq!(ids, vec!["c1".to_string(), "c2".to_string()]);
        assert_eq!(prefix_digest(&ids).len(), 64);
    }

    #[test]
    fn missing_through_commit_fails() {
        let commits = vec![sample_commit("c1", None)];
        let err = commit_prefix_ids(&commits, "nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let kp = Keypair::generate().unwrap();
        let peer = peer_id_of(&kp);
        let mut cert = PrefixCert {
            event_type: PREFIX_CERT_TYPE.to_string(),
            source_contract: "src".into(),
            through_commit: "c1".into(),
            prefix_digest: "aa".into(),
            source_path: Some("/hello.text".into()),
            value: Some(serde_json::json!("hi")),
            validator_peer_id: peer.clone(),
            gas_used: 1,
            fee_quoted: 0,
            requester_peer_id: None,
            signature: None,
        };
        sign_cert(&mut cert, &kp).unwrap();
        assert!(verify_cert_signature(&cert).unwrap());
        assert!(signer_is_named(&cert, &[peer.clone()]));
        let event = cert.as_event().unwrap();
        cheap_include_prefix_cert(&event, &[peer]).unwrap();
    }

    #[test]
    fn cheap_include_rejects_wrong_peer() {
        let kp = Keypair::generate().unwrap();
        let mut cert = PrefixCert {
            event_type: PREFIX_CERT_TYPE.to_string(),
            source_contract: "src".into(),
            through_commit: "c1".into(),
            prefix_digest: "aa".into(),
            source_path: None,
            value: None,
            validator_peer_id: peer_id_of(&kp),
            gas_used: 1,
            fee_quoted: 0,
            requester_peer_id: None,
            signature: None,
        };
        sign_cert(&mut cert, &kp).unwrap();
        let event = cert.as_event().unwrap();
        assert!(cheap_include_prefix_cert(&event, &["someone-else".into()]).is_err());
        let filtered = filter_includable_events(
            vec![event, serde_json::json!({"type": "contract_push"})],
            &[],
        );
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn cheap_include_rejects_unsigned() {
        let kp = Keypair::generate().unwrap();
        let peer = peer_id_of(&kp);
        let cert = PrefixCert {
            event_type: PREFIX_CERT_TYPE.to_string(),
            source_contract: "src".into(),
            through_commit: "c1".into(),
            prefix_digest: "aa".into(),
            source_path: None,
            value: None,
            validator_peer_id: peer.clone(),
            gas_used: 1,
            fee_quoted: 0,
            requester_peer_id: None,
            signature: None,
        };
        let event = cert.as_event().unwrap();
        assert!(cheap_include_prefix_cert(&event, &[peer]).is_err());
    }

    #[test]
    fn certs_sort_before_pushes() {
        let ordered = canonical_event_order(&[
            serde_json::json!({"type":"contract_push"}),
            serde_json::json!({"type":"prefix_cert"}),
        ]);
        assert_eq!(ordered[0]["type"], "prefix_cert");
        assert_eq!(ordered[1]["type"], "contract_push");
    }

    #[test]
    fn qc_threshold_two_thirds() {
        assert_eq!(qc_threshold(0, 2, 3), 0);
        assert_eq!(qc_threshold(1, 2, 3), 1);
        assert_eq!(qc_threshold(3, 2, 3), 2);
        assert_eq!(qc_threshold(4, 2, 3), 3);
    }

    #[test]
    fn matching_qc_signers_ignore_conflicting_digests() {
        let named = vec!["p1".into(), "p2".into(), "p3".into()];
        let good = serde_json::json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "src",
            "through_commit": "c1",
            "prefix_digest": "aa",
            "validator_peer_id": "p1",
            "gas_used": 1,
            "fee_quoted": 0
        });
        let conflict = serde_json::json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "src",
            "through_commit": "c1",
            "prefix_digest": "bb",
            "validator_peer_id": "p2",
            "gas_used": 1,
            "fee_quoted": 0
        });
        let also_good = serde_json::json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "src",
            "through_commit": "c1",
            "prefix_digest": "aa",
            "validator_peer_id": "p2",
            "gas_used": 1,
            "fee_quoted": 0
        });
        assert_eq!(
            matching_qc_signers(
                &[good.clone(), conflict],
                &named,
                "aa",
                "src",
                "c1",
                None,
                None
            ),
            1
        );
        assert_eq!(
            matching_qc_signers(&[good, also_good], &named, "aa", "src", "c1", None, None),
            2
        );
    }
}
