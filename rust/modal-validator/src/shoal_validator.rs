use crate::error::{Result, ValidatorError};
use modal_datastore::DatastoreManager;
use modal_validator_consensus::narwhal::{
    Certificate, Committee, Primary, PublicKey, Transaction, Validator, Worker,
    SyncClient, SyncRequest, SyncResponse,
};
use modal_validator_consensus::narwhal::dag::DAG;
use modal_validator_consensus::shoal::ReputationConfig;
use modal_validator_consensus::shoal::reputation::ReputationManager;
use modal_validator_consensus::shoal::consensus::ShoalConsensus;
use modal_validator_consensus::shoal::ordering::OrderingEngine;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Configuration for Shoal consensus
#[derive(Debug, Clone)]
pub struct ShoalValidatorConfig {
    /// This validator's keypair (placeholder - would use real crypto)
    pub validator_key: PublicKey,
    
    /// Committee of all validators
    pub committee: Committee,
    
    /// Narwhal configuration
    pub narwhal_config: NarwhalConfig,
    
    /// Shoal reputation configuration
    pub reputation_config: ReputationConfig,
}

/// Narwhal-specific configuration
#[derive(Debug, Clone)]
pub struct NarwhalConfig {
    /// Number of worker threads per validator
    pub workers_per_validator: usize,
    
    /// Maximum transactions per batch
    pub batch_size: usize,
    
    /// Maximum batch size in bytes
    pub max_batch_bytes: usize,
}

impl Default for NarwhalConfig {
    fn default() -> Self {
        Self {
            workers_per_validator: 4,
            batch_size: 1000,
            max_batch_bytes: 512 * 1024, // 512KB
        }
    }
}

impl ShoalValidatorConfig {
    /// Create a simple test configuration with N validators
    pub fn new_test(n_validators: usize, validator_index: usize) -> Self {
        use libp2p_identity::ed25519;
        
        let validators: Vec<Validator> = (0..n_validators)
            .map(|i| {
                // Create deterministic PeerId for testing
                let mut secret_bytes = [0u8; 32];
                secret_bytes[0] = i as u8 + 1;
                let secret = ed25519::SecretKey::try_from_bytes(secret_bytes)
                    .expect("valid secret key");
                let keypair = ed25519::Keypair::from(secret);
                let peer_id = libp2p_identity::PeerId::from_public_key(&keypair.public().into());
                
                Validator {
                    public_key: peer_id,
                    stake: 1,
                    network_address: format!("127.0.0.1:800{}", i)
                        .parse::<SocketAddr>()
                        .unwrap(),
                }
            })
            .collect();
        
        // Create validator key
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = validator_index as u8 + 1;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes)
            .expect("valid secret key");
        let keypair = ed25519::Keypair::from(secret);
        let validator_key = libp2p_identity::PeerId::from_public_key(&keypair.public().into());
        
        let committee = Committee::new(validators);
        
        Self {
            validator_key,
            committee,
            narwhal_config: NarwhalConfig::default(),
            reputation_config: ReputationConfig::default(),
        }
    }

    /// Create configuration from a list of peer ID strings
    /// 
    /// This is useful for creating a committee from static validator configuration.
    /// All validators will have equal stake (1) and placeholder network addresses.
    pub fn from_peer_ids(
        peer_id_strings: Vec<String>,
        validator_index: usize,
    ) -> Result<Self> {
        if validator_index >= peer_id_strings.len() {
            return Err(ValidatorError::InitializationFailed(
                format!("validator_index {} out of range for {} validators", 
                        validator_index, peer_id_strings.len())
            ));
        }
        
        // Parse all peer IDs
        let mut validators = Vec::new();
        for (i, peer_id_str) in peer_id_strings.iter().enumerate() {
            let peer_id: PublicKey = peer_id_str.parse()
                .map_err(|e| ValidatorError::InitializationFailed(
                    format!("invalid peer ID '{}': {}", peer_id_str, e)
                ))?;
            
            validators.push(Validator {
                public_key: peer_id,
                stake: 1,
                network_address: format!("127.0.0.1:800{}", i)
                    .parse::<SocketAddr>()
                    .unwrap(),
            });
        }
        
        // Get this validator's key
        let validator_key = peer_id_strings[validator_index].parse()
            .map_err(|e| ValidatorError::InitializationFailed(
                format!("invalid validator peer ID: {}", e)
            ))?;
        
        let committee = Committee::new(validators);
        
        Ok(Self {
            validator_key,
            committee,
            narwhal_config: NarwhalConfig::default(),
            reputation_config: ReputationConfig::default(),
        })
    }
}

/// Shoal-based validator implementation
pub struct ShoalValidator {
    /// Configuration
    config: ShoalValidatorConfig,
    
    /// Multi-store datastore manager
    datastore_manager: Option<Arc<Mutex<DatastoreManager>>>,
    
    /// The DAG
    dag: Arc<RwLock<DAG>>,
    
    /// Primary node
    primary: Arc<Mutex<Primary>>,
    
    /// Worker nodes
    workers: Vec<Arc<Mutex<Worker>>>,
    
    /// Shoal consensus engine
    consensus: Arc<Mutex<ShoalConsensus>>,
    
    /// Ordering engine
    ordering: OrderingEngine,
    
    /// Sync client for DAG synchronization
    sync_client: SyncClient,
}

impl ShoalValidator {
    /// Create a new Shoal-based validator with DatastoreManager
    pub async fn new(
        datastore_manager: Arc<Mutex<DatastoreManager>>,
        config: ShoalValidatorConfig,
    ) -> Result<Self> {
        // Start with a fresh DAG for multi-store mode
        let dag = Arc::new(RwLock::new(DAG::new()));
        
        // Create workers
        let mut workers = Vec::new();
        for i in 0..config.narwhal_config.workers_per_validator {
            let worker = Worker::new(
                i as u32,
                config.validator_key.clone(),
                config.narwhal_config.batch_size,
                config.narwhal_config.max_batch_bytes,
            );
            workers.push(Arc::new(Mutex::new(worker)));
        }
        
        // Create primary
        let primary = Primary::new(
            config.validator_key.clone(),
            config.committee.clone(),
            dag.clone(),
        );
        let primary = Arc::new(Mutex::new(primary));
        
        // Create reputation manager
        let reputation = ReputationManager::new(
            config.committee.clone(),
            config.reputation_config.clone(),
        );
        
        // Create consensus engine
        let consensus = {
            let cons = ShoalConsensus::new(
                dag.clone(),
                reputation,
                config.committee.clone(),
            );
            Arc::new(Mutex::new(cons))
        };
        
        // Create ordering engine
        let ordering = OrderingEngine::new(dag.clone());
        
        // Create sync client
        let sync_client = SyncClient::new(dag.clone());
        
        log::info!(
            "created Shoal validator (multi-store) for validator {:?}",
            config.validator_key
        );
        
        Ok(Self {
            config,
            datastore_manager: Some(datastore_manager),
            dag,
            primary,
            workers,
            consensus,
            ordering,
            sync_client,
        })
    }
    
    /// Initialize the validator by loading existing state
    pub async fn initialize(&self) -> Result<()> {
        // TODO: Load DAG and consensus state from datastore
        log::info!("Shoal validator initialized");
        Ok(())
    }
    
    /// Submit a transaction for ordering
    pub async fn submit_transaction(&self, tx: Transaction) -> Result<()> {
        // Add transaction to first available worker
        if let Some(worker) = self.workers.first() {
            let mut worker = worker.lock().await;
            worker.add_transaction(tx);
            log::debug!("transaction submitted, {} pending", worker.pending_count());
            Ok(())
        } else {
            Err(ValidatorError::InitializationFailed(
                "no workers available".to_string(),
            ).into())
        }
    }
    
    /// Propose a new batch (called periodically by consensus loop)
    pub async fn propose_batch(&self) -> Result<Option<Certificate>> {
        // Form batch from first worker
        let batch_opt = if let Some(worker) = self.workers.first() {
            let mut worker = worker.lock().await;
            worker.form_batch().await
        } else {
            None
        };
        
        if let Some((batch, batch_digest)) = batch_opt {
            log::info!("formed batch with {} transactions", batch.transactions.len());
            
            // Create header
            let mut primary = self.primary.lock().await;
            let header = primary.propose(batch_digest).await?;
            
            log::info!("proposed header for round {}", header.round);
            
            // In a real implementation, we would broadcast header and collect votes
            // For now, simulate immediate certificate formation for testing
            let mut builder = primary.create_certificate_builder(header);
            
            // Simulate votes from all validators (for testing)
            for validator in &self.config.committee.validator_order {
                builder.add_vote(validator.clone(), vec![])?;
            }
            
            let cert = builder.build()?;
            
            // Process certificate through consensus
            let _digest = cert.digest();
            primary.process_certificate(cert.clone()).await?;
            
            let mut consensus = self.consensus.lock().await;
            let committed = consensus.process_certificate(cert.clone()).await?;
            
            if !committed.is_empty() {
                log::info!("committed {} certificates", committed.len());
            }
            
            Ok(Some(cert))
        } else {
            Ok(None)
        }
    }
    
    /// Process a certificate received from another validator
    pub async fn process_certificate(&self, cert: Certificate) -> Result<Vec<Transaction>> {
        // Note: Certificate persistence requires DatastoreManager support in DAG (TODO)
        // #[cfg(feature = "persistence")]
        // {
        //     if let Some(ref ds) = self.datastore_manager {
        //         let dag = self.dag.read().await;
        //         let ds_guard = ds.lock().await;
        //         // TODO: Update DAG to support DatastoreManager
        //     }
        // }
        
        let primary = self.primary.lock().await;
        primary.process_certificate(cert.clone()).await?;
        drop(primary);
        
        let mut consensus = self.consensus.lock().await;
        let committed = consensus.process_certificate(cert).await?;
        
        if !committed.is_empty() {
            log::info!("committed {} certificates", committed.len());
            
            // Create checkpoint every 100 rounds
            // Note: Checkpoint creation requires DatastoreManager support in DAG (TODO)
            // #[cfg(feature = "persistence")]
            // {
            //     let current_round = {
            //         let dag = self.dag.read().await;
            //         dag.highest_round()
            //     };
            //     
            //     if current_round > 0 && current_round % 100 == 0 {
            //         // TODO: Update DAG to support DatastoreManager
            //     }
            // }
            
            // Order and extract transactions
            let consensus_state = &consensus.state;
            let transactions = self.ordering
                .order_certificates(&consensus_state.committed)
                .await?;
            
            // Process contract commits for asset state updates
            use crate::contract_processor::ContractProcessor;
            // Use datastore_manager if available, otherwise skip contract processing
            let Some(datastore_for_contracts) = self.datastore_manager.clone() else {
                log::debug!("No datastore manager available, skipping contract processing");
                return Ok(transactions);
            };
            let contract_processor = ContractProcessor::new(datastore_for_contracts);
            
            for tx in &transactions {
                // Parse transaction to see if it contains a contract commit
                if let Ok(tx_str) = std::str::from_utf8(&tx.data) {
                    if let Ok(tx_json) = serde_json::from_str::<serde_json::Value>(tx_str) {
                        // Check if this is a contract push transaction
                        if let Some(req_type) = tx_json.get("type").and_then(|v| v.as_str()) {
                            if req_type == "contract_push" {
                                // Extract contract data
                                if let Some(data) = tx_json.get("data") {
                                    if let (Some(contract_id), Some(commits)) = (
                                        data.get("contract_id").and_then(|v| v.as_str()),
                                        data.get("commits").and_then(|v| v.as_array())
                                    ) {
                                        // Process each commit
                                        for commit_entry in commits {
                                            if let (Some(commit_id), Some(commit_data_obj)) = (
                                                commit_entry.get("commit_id").and_then(|v| v.as_str()),
                                                commit_entry.get("body")
                                            ) {
                                                // Reconstruct commit data string
                                                let commit_data = serde_json::json!({
                                                    "body": commit_data_obj,
                                                    "head": commit_entry.get("head")
                                                });
                                                let commit_data_str = serde_json::to_string(&commit_data).unwrap_or_default();
                                                
                                                // Process the commit
                                                match contract_processor.process_commit(contract_id, commit_id, &commit_data_str).await {
                                                    Ok(state_changes) => {
                                                        log::info!("Processed commit {} for contract {}: {} state changes", 
                                                            commit_id, contract_id, state_changes.len());
                                                    }
                                                    Err(e) => {
                                                        log::warn!("Failed to process commit {} for contract {}: {}", 
                                                            commit_id, contract_id, e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            return Ok(transactions);
        }
        
        Ok(vec![])
    }
    
    /// Get the current consensus round
    pub async fn get_current_round(&self) -> u64 {
        let consensus = self.consensus.lock().await;
        consensus.current_round()
    }
    
    /// Get the last committed round
    pub async fn get_chain_tip(&self) -> u64 {
        let consensus = self.consensus.lock().await;
        consensus.last_committed_round()
    }
    
    /// Advance to the next round
    pub async fn advance_round(&self) {
        let mut primary = self.primary.lock().await;
        primary.advance_round();
        
        let mut consensus = self.consensus.lock().await;
        consensus.advance_round();
        
        log::info!("advanced to round {}", consensus.current_round());
    }
    
    /// Get committed transactions up to a certain round
    pub async fn get_committed_transactions(&self, _from_round: u64, _to_round: u64) -> Result<Vec<Transaction>> {
        // TODO: Implement range queries
        let consensus = self.consensus.lock().await;
        let transactions = self.ordering
            .order_certificates(&consensus.state.committed)
            .await?;
        Ok(transactions)
    }
    
    /// Get the number of pending transactions
    pub async fn pending_transaction_count(&self) -> usize {
        let mut total = 0;
        for worker in &self.workers {
            let worker = worker.lock().await;
            total += worker.pending_count();
        }
        total
    }
    
    // Sync methods for DAG synchronization
    
    /// Handle sync request from another node
    pub async fn handle_sync_request(&self, request: SyncRequest) -> SyncResponse {
        let dag = self.dag.read().await;
        dag.handle_sync_request(request)
    }
    
    /// Sync DAG with a peer using a request function
    /// The request_fn should send requests to the peer and return responses
    pub async fn sync_with_peer<F, Fut>(&self, request_fn: F) -> Result<modal_validator_consensus::narwhal::SyncStats>
    where
        F: Fn(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<SyncResponse>>,
    {
        self.sync_client.sync_with_peer(request_fn).await.map_err(|e| e.into())
    }
    
    /// Request specific certificates from a peer
    pub async fn request_certificates<F, Fut>(
        &self,
        digests: Vec<modal_validator_consensus::narwhal::CertificateDigest>,
        request_fn: F,
    ) -> Result<Vec<Certificate>>
    where
        F: Fn(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<SyncResponse>>,
    {
        self.sync_client.request_certificates(digests, request_fn).await.map_err(|e| e.into())
    }
    
    /// Sync missing parents for a certificate before processing it
    pub async fn sync_and_process_certificate<F, Fut>(
        &self,
        cert: Certificate,
        request_fn: F,
    ) -> Result<Vec<Transaction>>
    where
        F: Fn(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<SyncResponse>>,
    {
        // Check if we have all parents
        let has_parents = {
            let dag = self.dag.read().await;
            dag.has_all_parents(&cert)
        };
        
        if !has_parents {
            log::info!("Certificate has missing parents, syncing...");
            let synced = self.sync_client.sync_missing_parents(&cert, request_fn).await
                .map_err(|e| anyhow::Error::from(e))?;
            
            if !synced {
                return Err(ValidatorError::Custom(
                    "Failed to sync all parents for certificate".to_string()
                ).into());
            }
        }
        
        // Now process the certificate
        self.process_certificate(cert).await
    }
    
    /// Get the highest round in our DAG
    pub async fn get_highest_round(&self) -> u64 {
        let dag = self.dag.read().await;
        dag.highest_round()
    }
    
    /// Check if we have all certificates in a round
    pub async fn has_complete_round(&self, round: u64) -> bool {
        let dag = self.dag.read().await;
        let quorum_threshold = self.config.committee.quorum_threshold();
        dag.round_size(round) >= quorum_threshold as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_validator(validator_index: usize) -> (ShoalValidator, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let datastore_manager = DatastoreManager::open(temp_dir.path())
            .unwrap();
        let datastore_manager = Arc::new(Mutex::new(datastore_manager));
        
        let config = ShoalValidatorConfig::new_test(4, validator_index);
        let validator = ShoalValidator::new(datastore_manager, config).await.unwrap();
        
        (validator, temp_dir)
    }
    
    #[tokio::test]
    async fn test_shoal_validator_create() {
        let (validator, _temp) = create_test_validator(0).await;
        validator.initialize().await.unwrap();
        
        assert_eq!(validator.get_current_round().await, 0);
        assert_eq!(validator.get_chain_tip().await, 0);
    }
    
    #[tokio::test]
    async fn test_shoal_validator_from_peer_ids() {
        // Test creating a validator configuration from peer IDs
        let peer_ids = vec![
            "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string(),
            "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB".to_string(),
            "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se".to_string(),
        ];
        
        // Create config with validator at index 1
        let config = ShoalValidatorConfig::from_peer_ids(peer_ids.clone(), 1).unwrap();
        
        // Verify committee has all validators
        assert_eq!(config.committee.size(), 3);
        
        // Verify validator key is correct
        let expected_key: libp2p_identity::PeerId = peer_ids[1].parse().unwrap();
        assert_eq!(config.validator_key, expected_key);
        
        // Verify all peer IDs are in the committee
        for peer_id_str in peer_ids {
            let peer_id: libp2p_identity::PeerId = peer_id_str.parse().unwrap();
            assert!(config.committee.contains(&peer_id));
        }
    }
    
    #[tokio::test]
    async fn test_shoal_validator_from_peer_ids_invalid_index() {
        let peer_ids = vec![
            "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string(),
            "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB".to_string(),
        ];
        
        // Try to create config with out-of-bounds index
        let result = ShoalValidatorConfig::from_peer_ids(peer_ids, 5);
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_shoal_validator_submit_transaction() {
        let (validator, _temp) = create_test_validator(0).await;
        validator.initialize().await.unwrap();
        
        let tx = Transaction {
            data: vec![1, 2, 3],
            timestamp: 1000,
        };
        
        validator.submit_transaction(tx).await.unwrap();
        assert_eq!(validator.pending_transaction_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_shoal_validator_propose_batch() {
        let (validator, _temp) = create_test_validator(0).await;
        validator.initialize().await.unwrap();
        
        // Submit some transactions
        for i in 0..5 {
            let tx = Transaction {
                data: vec![i],
                timestamp: 1000 + i as u64,
            };
            validator.submit_transaction(tx).await.unwrap();
        }
        
        // Propose batch (should form certificate and commit for genesis)
        let cert = validator.propose_batch().await.unwrap();
        assert!(cert.is_some());
        
        let cert = cert.unwrap();
        assert_eq!(cert.header.round, 0); // Genesis round
        
        // Should be committed
        assert_eq!(validator.get_chain_tip().await, 0);
    }
    
    #[tokio::test]
    async fn test_shoal_validator_advance_round() {
        let (validator, _temp) = create_test_validator(0).await;
        validator.initialize().await.unwrap();
        
        assert_eq!(validator.get_current_round().await, 0);
        
        validator.advance_round().await;
        assert_eq!(validator.get_current_round().await, 1);
        
        validator.advance_round().await;
        assert_eq!(validator.get_current_round().await, 2);
    }
}

