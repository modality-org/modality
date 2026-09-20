//! Modality Contract Processor
//!
//! Validates and processes Modality contract commits during consensus.
//! Each commit is checked against accumulated formulas before acceptance.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use modal_datastore::DatastoreManager;
use modal_datastore::models::{
    ModalityContract, ModalityRule, ModalityAction as ModalityActionRecord,
    ModalityCommitBody,
};
use modality_lang::crypto::{verify_ed25519, VerifyResult};
use serde::{Serialize, Deserialize};

/// State change from processing a Modality commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModalityStateChange {
    ContractCreated {
        contract_id: String,
        parties: Vec<String>,
    },
    RuleAdded {
        contract_id: String,
        rule_id: String,
        formula: String,
        added_by: String,
    },
    ActionExecuted {
        contract_id: String,
        action_id: String,
        action: String,
        executed_by: String,
    },
    ContractFinalized {
        contract_id: String,
        finalized_by: String,
    },
}

/// Error types for Modality processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModalityError {
    InvalidSignature { signer: String, reason: String },
    UnauthorizedParty { party: String },
    FormulaViolation { action: String, formula: String, reason: String },
    InvalidPhase { expected: String, actual: String },
    ContractNotFound { contract_id: String },
    ParseError { reason: String },
}

impl std::fmt::Display for ModalityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModalityError::InvalidSignature { signer, reason } => 
                write!(f, "Invalid signature from {}: {}", signer, reason),
            ModalityError::UnauthorizedParty { party } => 
                write!(f, "Unauthorized party: {}", party),
            ModalityError::FormulaViolation { action, formula, reason } => 
                write!(f, "Action {} violates formula {}: {}", action, formula, reason),
            ModalityError::InvalidPhase { expected, actual } => 
                write!(f, "Invalid phase: expected {}, got {}", expected, actual),
            ModalityError::ContractNotFound { contract_id } => 
                write!(f, "Contract not found: {}", contract_id),
            ModalityError::ParseError { reason } => 
                write!(f, "Parse error: {}", reason),
        }
    }
}

impl std::error::Error for ModalityError {}

/// Processes Modality contract commits
pub struct ModalityContractProcessor {
    datastore: Arc<Mutex<DatastoreManager>>,
    /// Cache of loaded contracts for performance
    contract_cache: HashMap<String, ModalityContract>,
    /// Cache of rules per contract
    rules_cache: HashMap<String, Vec<ModalityRule>>,
}

impl ModalityContractProcessor {
    pub fn new(datastore: Arc<Mutex<DatastoreManager>>) -> Self {
        Self {
            datastore,
            contract_cache: HashMap::new(),
            rules_cache: HashMap::new(),
        }
    }

    /// Process a Modality commit during consensus
    pub async fn process_commit(
        &mut self,
        contract_id: &str,
        commit_id: &str,
        commit_body_json: &str,
    ) -> Result<Vec<ModalityStateChange>> {
        // Check if this is a modality commit
        if !ModalityCommitBody::is_modality_commit(commit_body_json) {
            return Ok(vec![]); // Not a modality commit, skip
        }

        let commit_body = ModalityCommitBody::from_json(commit_body_json)
            .map_err(|e| anyhow!(ModalityError::ParseError { reason: e.to_string() }))?;

        let mut changes = Vec::new();

        match commit_body {
            ModalityCommitBody::Init { version: _, parties } => {
                changes.extend(self.process_init(contract_id, commit_id, parties).await?);
            }
            ModalityCommitBody::AddRule { formula, signed_by, signature } => {
                changes.extend(self.process_add_rule(
                    contract_id, commit_id, &formula, &signed_by, &signature
                ).await?);
            }
            ModalityCommitBody::DomainAction { action, payload, signed_by, signature } => {
                changes.extend(self.process_domain_action(
                    contract_id, commit_id, &action, &payload, &signed_by, &signature
                ).await?);
            }
            ModalityCommitBody::Finalize { signed_by, signature } => {
                changes.extend(self.process_finalize(
                    contract_id, commit_id, &signed_by, &signature
                ).await?);
            }
        }

        Ok(changes)
    }

    /// Initialize a new Modality contract
    async fn process_init(
        &mut self,
        contract_id: &str,
        _commit_id: &str,
        parties: Vec<String>,
    ) -> Result<Vec<ModalityStateChange>> {
        let contract = ModalityContract::new(contract_id.to_string(), parties.clone());
        
        // Save to datastore
        {
            let ds = self.datastore.lock().await;
            contract.save(&ds).await?;
        }
        
        // Update cache
        self.contract_cache.insert(contract_id.to_string(), contract);
        self.rules_cache.insert(contract_id.to_string(), Vec::new());

        log::info!("Modality contract initialized: {} with {} parties", contract_id, parties.len());

        Ok(vec![ModalityStateChange::ContractCreated {
            contract_id: contract_id.to_string(),
            parties,
        }])
    }

    /// Add a rule (formula) to the contract
    async fn process_add_rule(
        &mut self,
        contract_id: &str,
        commit_id: &str,
        formula: &str,
        signed_by: &str,
        signature: &str,
    ) -> Result<Vec<ModalityStateChange>> {
        // Get contract
        let contract = self.get_contract(contract_id).await?;

        // Verify party is authorized
        if !contract.parties.contains(&signed_by.to_string()) {
            return Err(anyhow!(ModalityError::UnauthorizedParty { 
                party: signed_by.to_string() 
            }));
        }

        // Verify phase
        if contract.phase != "negotiating" {
            return Err(anyhow!(ModalityError::InvalidPhase {
                expected: "negotiating".to_string(),
                actual: contract.phase.clone(),
            }));
        }

        // Verify signature
        let message = format!("add_rule:{}:{}:{}", contract_id, commit_id, formula);
        self.verify_signature(signed_by, message.as_bytes(), signature)?;

        // Create rule record
        let rule_id = format!("rule_{}", contract.rule_count);
        let rule = ModalityRule {
            contract_id: contract_id.to_string(),
            rule_id: rule_id.clone(),
            formula: formula.to_string(),
            added_by: signed_by.to_string(),
            signature: signature.to_string(),
            commit_id: commit_id.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };

        // Save rule
        {
            let ds = self.datastore.lock().await;
            rule.save(&ds).await?;
        }

        // Update contract
        let mut updated_contract = contract.clone();
        updated_contract.rule_count += 1;
        updated_contract.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        {
            let ds = self.datastore.lock().await;
            updated_contract.save(&ds).await?;
        }

        // Update caches
        self.contract_cache.insert(contract_id.to_string(), updated_contract);
        self.rules_cache
            .entry(contract_id.to_string())
            .or_default()
            .push(rule);

        log::info!("Rule added to contract {}: {} by {}", contract_id, formula, signed_by);

        Ok(vec![ModalityStateChange::RuleAdded {
            contract_id: contract_id.to_string(),
            rule_id,
            formula: formula.to_string(),
            added_by: signed_by.to_string(),
        }])
    }

    /// Execute a domain action
    async fn process_domain_action(
        &mut self,
        contract_id: &str,
        commit_id: &str,
        action: &str,
        payload: &serde_json::Value,
        signed_by: &str,
        signature: &str,
    ) -> Result<Vec<ModalityStateChange>> {
        // Get contract
        let contract = self.get_contract(contract_id).await?;

        // Verify party is authorized
        if !contract.parties.contains(&signed_by.to_string()) {
            return Err(anyhow!(ModalityError::UnauthorizedParty { 
                party: signed_by.to_string() 
            }));
        }

        // Verify phase (must be active, not negotiating)
        if contract.phase != "active" {
            return Err(anyhow!(ModalityError::InvalidPhase {
                expected: "active".to_string(),
                actual: contract.phase.clone(),
            }));
        }

        // Verify signature
        let message = format!("domain_action:{}:{}:{}:{}", contract_id, commit_id, action, payload);
        self.verify_signature(signed_by, message.as_bytes(), signature)?;

        // Get all rules and validate action against them
        let rules = self.get_rules(contract_id).await?;
        self.validate_action_against_rules(action, payload, &rules)?;

        // Create action record
        let action_id = format!("action_{}", contract.action_count);
        let action_record = ModalityActionRecord {
            contract_id: contract_id.to_string(),
            action_id: action_id.clone(),
            action: action.to_string(),
            payload: payload.to_string(),
            executed_by: signed_by.to_string(),
            signature: signature.to_string(),
            commit_id: commit_id.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };

        // Save action
        {
            let ds = self.datastore.lock().await;
            action_record.save(&ds).await?;
        }

        // Update contract
        let mut updated_contract = contract.clone();
        updated_contract.action_count += 1;
        updated_contract.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        {
            let ds = self.datastore.lock().await;
            updated_contract.save(&ds).await?;
        }

        self.contract_cache.insert(contract_id.to_string(), updated_contract);

        log::info!("Action executed in contract {}: {} by {}", contract_id, action, signed_by);

        Ok(vec![ModalityStateChange::ActionExecuted {
            contract_id: contract_id.to_string(),
            action_id,
            action: action.to_string(),
            executed_by: signed_by.to_string(),
        }])
    }

    /// Finalize the negotiation phase
    async fn process_finalize(
        &mut self,
        contract_id: &str,
        commit_id: &str,
        signed_by: &str,
        signature: &str,
    ) -> Result<Vec<ModalityStateChange>> {
        // Get contract
        let contract = self.get_contract(contract_id).await?;

        // Verify party is authorized
        if !contract.parties.contains(&signed_by.to_string()) {
            return Err(anyhow!(ModalityError::UnauthorizedParty { 
                party: signed_by.to_string() 
            }));
        }

        // Verify phase
        if contract.phase != "negotiating" {
            return Err(anyhow!(ModalityError::InvalidPhase {
                expected: "negotiating".to_string(),
                actual: contract.phase.clone(),
            }));
        }

        // Verify signature
        let message = format!("finalize:{}:{}", contract_id, commit_id);
        self.verify_signature(signed_by, message.as_bytes(), signature)?;

        // Update contract to active phase
        // Note: In a full implementation, we'd track which parties have finalized
        // and only transition to "active" when all parties have finalized
        let mut updated_contract = contract.clone();
        updated_contract.phase = "active".to_string();
        updated_contract.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        {
            let ds = self.datastore.lock().await;
            updated_contract.save(&ds).await?;
        }

        self.contract_cache.insert(contract_id.to_string(), updated_contract);

        log::info!("Contract {} finalized by {}", contract_id, signed_by);

        Ok(vec![ModalityStateChange::ContractFinalized {
            contract_id: contract_id.to_string(),
            finalized_by: signed_by.to_string(),
        }])
    }

    /// Get contract from cache or datastore
    async fn get_contract(&mut self, contract_id: &str) -> Result<ModalityContract> {
        if let Some(contract) = self.contract_cache.get(contract_id) {
            return Ok(contract.clone());
        }

        let ds = self.datastore.lock().await;
        let contract = ModalityContract::find_by_id(&ds, contract_id).await?
            .ok_or_else(|| anyhow!(ModalityError::ContractNotFound { 
                contract_id: contract_id.to_string() 
            }))?;

        self.contract_cache.insert(contract_id.to_string(), contract.clone());
        Ok(contract)
    }

    /// Get rules from cache or datastore
    async fn get_rules(&mut self, contract_id: &str) -> Result<Vec<ModalityRule>> {
        if let Some(rules) = self.rules_cache.get(contract_id) {
            return Ok(rules.clone());
        }

        let ds = self.datastore.lock().await;
        let rules = ModalityRule::find_by_contract(&ds, contract_id).await?;

        self.rules_cache.insert(contract_id.to_string(), rules.clone());
        Ok(rules)
    }

    /// Verify an ed25519 signature
    fn verify_signature(&self, public_key: &str, message: &[u8], signature: &str) -> Result<()> {
        match verify_ed25519(public_key, message, signature) {
            VerifyResult::Valid => Ok(()),
            VerifyResult::Invalid => Err(anyhow!(ModalityError::InvalidSignature {
                signer: public_key.to_string(),
                reason: "Signature verification failed".to_string(),
            })),
            VerifyResult::Error(e) => Err(anyhow!(ModalityError::InvalidSignature {
                signer: public_key.to_string(),
                reason: e,
            })),
        }
    }

    /// Validate an action against all accumulated rules
    fn validate_action_against_rules(
        &self,
        action: &str,
        _payload: &serde_json::Value,
        rules: &[ModalityRule],
    ) -> Result<()> {
        // For now, we do basic validation
        // TODO: Integrate with modality-lang model checker for full formula verification
        
        for rule in rules {
            // Check if the rule's formula mentions this action
            // In a full implementation, we'd parse the formula and check semantically
            if rule.formula.contains(action) {
                log::debug!("Action {} is mentioned in rule: {}", action, rule.formula);
                // For Phase 1, we just log - Phase 2 will add full model checking
            }
        }

        Ok(())
    }

    /// Clear caches (useful for testing or reloading)
    pub fn clear_caches(&mut self) {
        self.contract_cache.clear();
        self.rules_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use modality_lang::crypto::{generate_keypair, sign_ed25519};

    async fn create_test_processor() -> (ModalityContractProcessor, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let datastore = DatastoreManager::open(temp_dir.path()).unwrap();
        let datastore = Arc::new(Mutex::new(datastore));
        let processor = ModalityContractProcessor::new(datastore);
        (processor, temp_dir)
    }

    #[tokio::test]
    async fn test_init_contract() {
        let (mut processor, _temp) = create_test_processor().await;
        
        let (_, alice_pub) = generate_keypair();
        let (_, bob_pub) = generate_keypair();

        let init_json = serde_json::json!({
            "type": "init_modality",
            "version": "0.1",
            "parties": [alice_pub, bob_pub]
        }).to_string();

        let changes = processor.process_commit(
            "test_contract",
            "commit_0",
            &init_json,
        ).await.unwrap();

        assert_eq!(changes.len(), 1);
        match &changes[0] {
            ModalityStateChange::ContractCreated { contract_id, parties } => {
                assert_eq!(contract_id, "test_contract");
                assert_eq!(parties.len(), 2);
            }
            _ => panic!("Expected ContractCreated"),
        }
    }

    #[tokio::test]
    async fn test_add_rule() {
        let (mut processor, _temp) = create_test_processor().await;
        
        let (alice_priv, alice_pub) = generate_keypair();
        let (_, bob_pub) = generate_keypair();

        // First init the contract
        let init_json = serde_json::json!({
            "type": "init_modality",
            "version": "0.1",
            "parties": [&alice_pub, &bob_pub]
        }).to_string();

        processor.process_commit("test_contract", "commit_0", &init_json).await.unwrap();

        // Now add a rule
        let formula = "[+DELIVER] eventually(paid | refunded)";
        let message = format!("add_rule:test_contract:commit_1:{}", formula);
        let signature = sign_ed25519(&alice_priv, message.as_bytes()).unwrap();

        let add_rule_json = serde_json::json!({
            "type": "add_rule",
            "formula": formula,
            "signed_by": alice_pub,
            "signature": signature
        }).to_string();

        let changes = processor.process_commit(
            "test_contract",
            "commit_1",
            &add_rule_json,
        ).await.unwrap();

        assert_eq!(changes.len(), 1);
        match &changes[0] {
            ModalityStateChange::RuleAdded { formula: f, added_by, .. } => {
                assert_eq!(f, formula);
                assert_eq!(added_by, &alice_pub);
            }
            _ => panic!("Expected RuleAdded"),
        }
    }

    #[tokio::test]
    async fn test_unauthorized_party() {
        let (mut processor, _temp) = create_test_processor().await;
        
        let (_, alice_pub) = generate_keypair();
        let (_, bob_pub) = generate_keypair();
        let (charlie_priv, charlie_pub) = generate_keypair();

        // Init with alice and bob
        let init_json = serde_json::json!({
            "type": "init_modality",
            "version": "0.1",
            "parties": [&alice_pub, &bob_pub]
        }).to_string();

        processor.process_commit("test_contract", "commit_0", &init_json).await.unwrap();

        // Charlie tries to add a rule (should fail)
        let formula = "[+STEAL] eventually(profit)";
        let message = format!("add_rule:test_contract:commit_1:{}", formula);
        let signature = sign_ed25519(&charlie_priv, message.as_bytes()).unwrap();

        let add_rule_json = serde_json::json!({
            "type": "add_rule",
            "formula": formula,
            "signed_by": charlie_pub,
            "signature": signature
        }).to_string();

        let result = processor.process_commit(
            "test_contract",
            "commit_1",
            &add_rule_json,
        ).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unauthorized party"));
    }
}
