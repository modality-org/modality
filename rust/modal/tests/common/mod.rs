//! Common test utilities for hub integration tests

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static CONTRACT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Test wrapper for hub operations
pub struct TestHub {
    data_dir: PathBuf,
}

impl TestHub {
    /// Create a new test hub
    pub async fn new(data_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");
        Self { data_dir }
    }

    /// Create a new contract
    pub async fn create_contract(&self) -> Result<String, String> {
        let contract_id = format!(
            "test-contract-{}",
            CONTRACT_COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        
        let contract_dir = self.data_dir.join("contracts").join(&contract_id);
        std::fs::create_dir_all(contract_dir.join("commits"))
            .map_err(|e| e.to_string())?;
        
        Ok(contract_id)
    }

    /// Commit a MODEL to a contract
    pub async fn commit_model(&self, contract_id: &str, model: &str) -> Result<String, String> {
        self.commit(contract_id, "model", model).await
    }

    /// Commit a RULE to a contract
    pub async fn commit_rule(&self, contract_id: &str, rule: &str) -> Result<String, String> {
        self.commit(contract_id, "rule", rule).await
    }

    /// Generic commit
    async fn commit(&self, contract_id: &str, method: &str, value: &str) -> Result<String, String> {
        use sha2::{Sha256, Digest};

        let contract_dir = self.data_dir.join("contracts").join(contract_id);
        let commits_dir = contract_dir.join("commits");

        // Read current HEAD
        let head_file = contract_dir.join("HEAD");
        let parent = if head_file.exists() {
            Some(std::fs::read_to_string(&head_file).map_err(|e| e.to_string())?.trim().to_string())
        } else {
            None
        };

        // Load existing commits for validation
        let commits = self.load_commits(contract_id)?;

        // For MODEL commits, validate against rules
        if method == "model" {
            self.validate_model(contract_id, value, &commits)?;
        }

        // Create commit
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let commit = serde_json::json!({
            "head": {
                "parent": parent,
                "timestamp": timestamp
            },
            "body": [{
                "method": method,
                "path": if method == "model" { "/model.modality".to_string() } else { format!("/rules/{}.modality", timestamp) },
                "value": value
            }],
            "timestamp": timestamp
        });

        // Compute hash
        let commit_str = serde_json::to_string(&commit).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        hasher.update(commit_str.as_bytes());
        let hash = hex::encode(hasher.finalize());

        // Write commit
        let commit_path = commits_dir.join(format!("{}.json", hash));
        std::fs::write(&commit_path, commit_str).map_err(|e| e.to_string())?;

        // Update HEAD
        std::fs::write(&head_file, &hash).map_err(|e| e.to_string())?;

        Ok(hash)
    }

    /// Load commits from disk
    fn load_commits(&self, contract_id: &str) -> Result<Vec<StoredCommit>, String> {
        let commits_dir = self.data_dir.join("contracts").join(contract_id).join("commits");
        let mut commits = Vec::new();

        if !commits_dir.exists() {
            return Ok(commits);
        }

        for entry in std::fs::read_dir(&commits_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                let content = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                let commit: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                
                let hash = entry.path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                commits.push(StoredCommit {
                    hash,
                    body: commit.get("body").cloned().unwrap_or(serde_json::json!([])),
                });
            }
        }

        Ok(commits)
    }

    /// Validate MODEL commit against rules
    fn validate_model(&self, _contract_id: &str, model_content: &str, commits: &[StoredCommit]) -> Result<(), String> {
        use modal::cmds::hub::model_validator::{ModelValidator, ReplayCommit};

        // Build replay commits
        let replay_commits: Vec<ReplayCommit> = commits.iter().enumerate()
            .map(|(i, c)| {
                let mut method = String::new();
                let mut rule_content = None;
                let mut model_content = None;

                if let Some(actions) = c.body.as_array() {
                    for action in actions {
                        let m = action.get("method")
                            .and_then(|m| m.as_str())
                            .unwrap_or("")
                            .to_lowercase();

                        if !m.is_empty() {
                            method = m.clone();
                        }

                        match m.as_str() {
                            "model" => {
                                model_content = action.get("value")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                            "rule" => {
                                rule_content = action.get("value")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                            _ => {}
                        }
                    }
                }

                ReplayCommit {
                    index: i,
                    method,
                    body: c.body.clone(),
                    action_labels: vec![],
                    rule_content,
                    model_content,
                }
            })
            .collect();

        let validator = ModelValidator::from_commits(&replay_commits)?;
        let result = validator.validate_new_model(model_content);

        if !result.valid {
            return Err(format!("MODEL rejected: {}", result.errors.join("; ")));
        }

        Ok(())
    }
}

/// Minimal stored commit for tests
struct StoredCommit {
    #[allow(dead_code)]
    hash: String,
    body: serde_json::Value,
}
