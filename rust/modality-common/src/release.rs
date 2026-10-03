//! Releases are commits to a release contract. A package is trusted only when
//! the contract accepts the commit that names it: the commit's id hashes to
//! its body and head, it extends the contract's log from a genesis this
//! binary pins, its signatures verify, and it meets the contract's model and
//! rules, replayed from the log alone. The rules say which key may post a
//! release (`/keys/ci.id`) and that changing that key, the maintainers, the
//! model or the rules takes every maintainer's signature. So the key that
//! signs releases is the contract's to name and replace, not the binary's.
//!
//! A release posts `/releases/<channel>/<version>.json` (an [`Entry`]: each
//! file's sha256) and `/releases/<channel>/latest.text` (that version).

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::contract_store::CommitFile;

/// The release contract's model: one state, two kinds of step. The CI key
/// posts releases and touches nothing else; every maintainer together
/// changes anything but releases.
pub const MODEL: &str = r#"model release {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/keys/ci.id) -modifies(/keys) -modifies(/maintainers) -modifies(/rules) -modifies(/model)
    q1 --> q1: +all_signed(/maintainers) -modifies(/releases)
  }
}
"#;

/// The release contract's rules, by name. Rules accumulate, so these hold
/// for the contract's life.
pub const RULES: &[(&str, &str)] = &[
    ("releases_by_ci", "always([+modifies(/releases) -signed_by(/keys/ci.id)] false)"),
    ("keys_by_maintainers", "always([+modifies(/keys) -all_signed(/maintainers)] false)"),
    ("maintainers_by_maintainers", "always([+modifies(/maintainers) -all_signed(/maintainers)] false)"),
    ("rules_by_maintainers", "always([+modifies(/rules) -all_signed(/maintainers)] false)"),
    ("model_by_maintainers", "always([+modifies(/model) -all_signed(/maintainers)] false)"),
];

/// A release contract a binary trusts: its id and genesis commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    pub contract_id: &'static str,
    pub genesis_commit_id: &'static str,
}

/// The release contract for the `testnet` channel.
pub const TESTNET: Pin = Pin {
    contract_id: "12D3KooWKGq653tYqRCyvcjQtDdKqLxcUZXxaJqv6KNHFwxuq53g",
    genesis_commit_id: "1b5958dc06e02fb5c8d1a8d30c5d0c98e466bc141c67d610e1b9bad3f512b9d6",
};

/// The pinned contract for `channel`, if this build trusts one.
pub fn pin_for(channel: &str) -> Option<Pin> {
    match channel {
        "testnet" if TESTNET.genesis_commit_id != "PENDING" => Some(TESTNET),
        _ => None,
    }
}

/// Where a channel's release log is published.
pub fn log_url(base_url: &str, channel: &str) -> String {
    format!("{}/{}/release-contract/log.json", base_url.trim_end_matches('/'), channel)
}

/// A release contract's log: every commit, genesis first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub contract_id: String,
    pub commits: Vec<LogCommit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCommit {
    pub commit_id: String,
    pub body: Value,
    pub head: Value,
}

/// What a release names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub version: String,
    pub channel: String,
    /// The source commit the files were built from.
    pub git_commit: String,
    /// Each file's path in the package (as in the manifest) and its sha256.
    pub files: BTreeMap<String, String>,
}

pub fn entry_path(channel: &str, version: &str) -> String {
    format!("/releases/{channel}/{version}.json")
}

pub fn latest_path(channel: &str) -> String {
    format!("/releases/{channel}/latest.text")
}

/// The state a release log reaches, once every commit is checked: ids,
/// parents, the pinned genesis, signatures, and the contract's model and
/// rules (theory v3), all from the log alone.
pub fn verify_log(log: &Log, pin: Pin) -> Result<Map<String, Value>> {
    use crate::model_governance::TheoryActivation;
    if log.contract_id != pin.contract_id {
        bail!(
            "the release log is for contract {}, not the pinned {}",
            log.contract_id,
            pin.contract_id
        );
    }
    let first = log.commits.first().ok_or_else(|| anyhow!("the release log is empty"))?;
    if first.commit_id != pin.genesis_commit_id {
        bail!(
            "the release log starts at {}, not the pinned genesis {}",
            first.commit_id,
            pin.genesis_commit_id
        );
    }
    let mut prefix: Vec<(String, CommitFile)> = Vec::with_capacity(log.commits.len());
    let mut parent: Option<String> = None;
    for commit in &log.commits {
        let file = CommitFile::verified(&commit.commit_id, Some(&commit.body), Some(&commit.head))
            .with_context(|| format!("release log commit {}", commit.commit_id))?;
        let file_parent = file.head.parent.clone().filter(|p| !p.is_empty());
        if file_parent != parent {
            bail!("release log commit {} does not follow the one before it", commit.commit_id);
        }
        parent = Some(commit.commit_id.clone());
        prefix.push((commit.commit_id.clone(), file));
    }
    let (accepted, _) = crate::independent_replay::expand_and_validate_prefix(
        pin.contract_id,
        &prefix,
        &[],
        None,
        TheoryActivation::always(modality_lang::TheoryVersion::V3),
    )
    .context("the release contract refuses its own log")?;
    Ok(crate::independent_replay::accepted_state_from_commits(&accepted))
}

/// The release `version` of `channel` in a verified state, or the channel's
/// latest when `version` is `None`.
pub fn entry(state: &Map<String, Value>, channel: &str, version: Option<&str>) -> Result<Entry> {
    let version = match version {
        Some(v) => v.to_string(),
        None => state
            .get(&latest_path(channel))
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("the release contract names no latest {channel} release"))?
            .to_string(),
    };
    let value = state
        .get(&entry_path(channel, &version))
        .ok_or_else(|| anyhow!("the release contract has no {channel} release {version}"))?;
    let entry: Entry = serde_json::from_value(value.clone())
        .with_context(|| format!("release {version} is malformed"))?;
    if entry.version != version || entry.channel != channel {
        bail!("release {version} names {} {}", entry.channel, entry.version);
    }
    Ok(entry)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Refuse `bytes` unless they are the release's file at `path`.
pub fn check_file(entry: &Entry, path: &str, bytes: &[u8]) -> Result<()> {
    let expected = entry
        .files
        .get(path)
        .ok_or_else(|| anyhow!("release {} has no file {path}", entry.version))?;
    let actual = sha256_hex(bytes);
    if &actual != expected {
        bail!(
            "{path} has sha256 {actual}, but release {} names {expected}",
            entry.version
        );
    }
    Ok(())
}

/// Check a downloaded file against the release contract: fetch nothing,
/// given the log's JSON. Returns the release.
pub fn verify_download(
    log_json: &str,
    pin: Pin,
    channel: &str,
    version: Option<&str>,
    path: &str,
    bytes: &[u8],
) -> Result<Entry> {
    let log: Log = serde_json::from_str(log_json).context("the release log is not JSON")?;
    let state = verify_log(&log, pin)?;
    let entry = entry(&state, channel, version)?;
    check_file(&entry, path, bytes)?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit_signatures::sign_commit;
    use crate::keypair::Keypair;
    use serde_json::json;

    struct Chain {
        contract: Keypair,
        id: String,
        commits: Vec<LogCommit>,
    }

    impl Chain {
        fn head(&self) -> Option<String> {
            self.commits.last().map(|c| c.commit_id.clone())
        }

        fn push(&mut self, body: Value, signers: &[&Keypair]) -> String {
            let mut head = json!({});
            if let Some(parent) = self.head() {
                head["parent"] = parent.into();
            }
            let mut file: CommitFile =
                serde_json::from_value(json!({"body": body, "head": head})).unwrap();
            let mut sigs = serde_json::Map::new();
            for key in signers {
                let (k, s) = sign_commit(key, &self.id, &file).unwrap();
                sigs.insert(k, s.into());
            }
            if !sigs.is_empty() {
                file.head.signatures = Some(Value::Object(sigs));
            }
            let id = file.compute_id().unwrap();
            self.commits.push(LogCommit {
                commit_id: id.clone(),
                body: serde_json::to_value(&file.body).unwrap(),
                head: serde_json::to_value(&file.head).unwrap(),
            });
            id
        }

        fn log(&self) -> Log {
            Log { contract_id: self.id.clone(), commits: self.commits.clone() }
        }

        fn pin(&self) -> Pin {
            Pin {
                contract_id: Box::leak(self.id.clone().into_boxed_str()),
                genesis_commit_id: Box::leak(self.commits[0].commit_id.clone().into_boxed_str()),
            }
        }
    }

    fn release_contract(ci: &Keypair, maintainer: &Keypair) -> Chain {
        let contract = Keypair::generate().unwrap();
        let id = contract.as_public_address();
        let mut chain = Chain { contract, id: id.clone(), commits: vec![] };
        chain.push(json!([{"method": "genesis", "value": {"genesis": {"contract_id": id}}}]), &[]);
        let mut setup = vec![
            json!({"method": "post", "path": "/keys/ci.id", "value": ci.public_key_as_base58_identity()}),
            json!({"method": "post", "path": "/maintainers/1.id", "value": maintainer.public_key_as_base58_identity()}),
            json!({"method": "model", "path": "/model/default.modality", "value": MODEL}),
        ];
        for (name, formula) in RULES {
            setup.push(json!({
                "method": "rule",
                "path": format!("/rules/{name}.modality"),
                "value": format!("export default rule {{\n  starting_at $PARENT\n  formula {{\n    {formula}\n  }}\n}}\n"),
            }));
        }
        chain.push(Value::Array(setup), &[maintainer]);
        let _ = &chain.contract;
        chain
    }

    fn release_body(version: &str, bytes: &[u8]) -> Value {
        let entry = Entry {
            version: version.into(),
            channel: "testnet".into(),
            git_commit: "abc123".into(),
            files: [("binaries/linux-x86_64/modal".to_string(), sha256_hex(bytes))].into(),
        };
        json!([
            {"method": "post", "path": entry_path("testnet", version), "value": serde_json::to_value(&entry).unwrap()},
            {"method": "post", "path": latest_path("testnet"), "value": version},
        ])
    }

    #[test]
    fn a_release_the_contract_accepts_verifies_its_files() {
        let (ci, maintainer) = (Keypair::generate().unwrap(), Keypair::generate().unwrap());
        let mut chain = release_contract(&ci, &maintainer);
        chain.push(release_body("v1", b"binary one"), &[&ci]);
        let log = serde_json::to_string(&chain.log()).unwrap();
        let path = "binaries/linux-x86_64/modal";
        let entry = verify_download(&log, chain.pin(), "testnet", None, path, b"binary one").unwrap();
        assert_eq!(entry.version, "v1");
        let err = verify_download(&log, chain.pin(), "testnet", None, path, b"tampered").unwrap_err();
        assert!(err.to_string().contains("but release v1 names"), "{err}");
    }

    #[test]
    fn a_release_signed_by_another_key_is_refused() {
        let (ci, maintainer) = (Keypair::generate().unwrap(), Keypair::generate().unwrap());
        let mallory = Keypair::generate().unwrap();
        let mut chain = release_contract(&ci, &maintainer);
        chain.push(release_body("v1", b"evil"), &[&mallory]);
        let err = verify_log(&chain.log(), chain.pin()).unwrap_err();
        assert!(format!("{err:#}").contains("signed_by(/keys/ci.id)"), "{err:#}");
    }

    #[test]
    fn the_ci_key_cannot_replace_itself_but_the_maintainers_can() {
        let (ci, maintainer) = (Keypair::generate().unwrap(), Keypair::generate().unwrap());
        let next = Keypair::generate().unwrap();
        let rotate = json!([{"method": "post", "path": "/keys/ci.id", "value": next.public_key_as_base58_identity()}]);

        let mut stolen = release_contract(&ci, &maintainer);
        stolen.push(rotate.clone(), &[&ci]);
        assert!(verify_log(&stolen.log(), stolen.pin()).is_err(), "the CI key alone cannot rotate");

        let mut rotated = release_contract(&ci, &maintainer);
        rotated.push(rotate, &[&maintainer]);
        rotated.push(release_body("v2", b"two"), &[&next]);
        let state = verify_log(&rotated.log(), rotated.pin()).unwrap();
        assert_eq!(entry(&state, "testnet", None).unwrap().version, "v2");
        rotated.push(release_body("v3", b"three"), &[&ci]);
        assert!(verify_log(&rotated.log(), rotated.pin()).is_err(), "the old key is out");
    }

    #[test]
    fn a_log_from_another_genesis_or_tampered_is_refused() {
        let (ci, maintainer) = (Keypair::generate().unwrap(), Keypair::generate().unwrap());
        let mut chain = release_contract(&ci, &maintainer);
        chain.push(release_body("v1", b"one"), &[&ci]);
        let other = release_contract(&ci, &maintainer);
        assert!(verify_log(&chain.log(), other.pin()).is_err());

        let mut tampered = chain.log();
        tampered.commits[2].body[0]["value"]["files"]["binaries/linux-x86_64/modal"] =
            sha256_hex(b"evil").into();
        assert!(verify_log(&tampered, chain.pin()).is_err(), "the id no longer matches");
    }
}
