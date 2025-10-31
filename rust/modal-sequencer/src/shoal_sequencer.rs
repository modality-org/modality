use crate::error::{Result, SequencerError};
use modal_datastore::NetworkDatastore;
use modal_sequencer_consensus::narwhal::{
    Batch, Certificate, Committee, Header, Primary, PublicKey, Transaction, Validator, Worker,
};
use modal_sequencer_consensus::narwhal::dag::DAG;
use modal_sequencer_consensus::shoal::{ReputationConfig, ReputationState};
use modal_sequencer_consensus::shoal::reputation::ReputationManager;
use modal_sequencer_consensus::shoal::consensus::ShoalConsensus;
use modal_sequencer_consensus::shoal::ordering::OrderingEngine;
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Configuration for Shoal consensus
#[derive(Debug, Clone)]
pub struct ShoalSequencerConfig {
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

impl ShoalSequencerConfig {
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
}

/// Shoal-based sequencer implementation
pub struct ShoalSequencer {
    /// Configuration
    config: ShoalSequencerConfig,
    
    /// Persistent datastore
    datastore: Arc<Mutex<NetworkDatastore>>,
    
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
}

impl ShoalSequencer {
    /// Create a new Shoal-based sequencer
    pub async fn new(
        datastore: Arc<Mutex<NetworkDatastore>>,
        config: ShoalSequencerConfig,
    ) -> Result<Self> {
        // Try to recover DAG from datastore
        let dag = {
            let ds = datastore.lock().await;
            #[cfg(feature = "persistence")]
            {
                use modal_sequencer_consensus::persistence::recovery::{recover_dag, RecoveryStrategy};
                
                log::info!("Attempting to recover DAG from datastore...");
                match recover_dag(&ds, RecoveryStrategy::Hybrid).await {
                    Ok(result) => {
                        log::info!("Recovered DAG: {} certificates, highest round: {}, used checkpoint: {}",
                                   result.certificates_loaded, result.highest_round, result.used_checkpoint);
                        Arc::new(RwLock::new(result.dag))
                    }
                    Err(e) => {
                        log::warn!("DAG recovery failed ({}), starting fresh", e);
                        Arc::new(RwLock::new(DAG::new()))
                    }
                }
            }
            #[cfg(not(feature = "persistence"))]
            {
                Arc::new(RwLock::new(DAG::new()))
            }
        };
        
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
        
        // Create consensus engine with datastore integration
        let consensus = {
            let mut cons = ShoalConsensus::new(
                dag.clone(),
                reputation,
                config.committee.clone(),
            );
            
            #[cfg(feature = "persistence")]
            {
                let ds_clone = datastore.clone();
                let ds_guard = ds_clone.lock().await;
                cons = cons.with_datastore(Arc::new(ds_guard.clone_to_memory().await?));
            }
            
            Arc::new(Mutex::new(cons))
        };
        
        // Create ordering engine
        let ordering = OrderingEngine::new(dag.clone());
        
        log::info!(
            "created Shoal sequencer for validator {:?}",
            config.validator_key
        );
        
        Ok(Self {
            config,
            datastore,
            dag,
            primary,
            workers,
            consensus,
            ordering,
        })
    }
    
    /// Initialize the sequencer by loading existing state
    pub async fn initialize(&self) -> Result<()> {
        // TODO: Load DAG and consensus state from datastore
        log::info!("Shoal sequencer initialized");
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
            Err(SequencerError::InitializationFailed(
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
            let digest = cert.digest();
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
        // Persist certificate to datastore
        #[cfg(feature = "persistence")]
        {
            let dag = self.dag.read().await;
            let ds = self.datastore.lock().await;
            if let Err(e) = dag.persist_certificate(&cert, &ds).await {
                log::warn!("Failed to persist certificate: {}", e);
            }
        }
        
        let mut primary = self.primary.lock().await;
        primary.process_certificate(cert.clone()).await?;
        
        let mut consensus = self.consensus.lock().await;
        let committed = consensus.process_certificate(cert).await?;
        
        if !committed.is_empty() {
            log::info!("committed {} certificates", committed.len());
            
            // Create checkpoint every 100 rounds
            #[cfg(feature = "persistence")]
            {
                let current_round = {
                    let dag = self.dag.read().await;
                    dag.highest_round()
                };
                
                if current_round > 0 && current_round % 100 == 0 {
                    log::info!("Creating checkpoint at round {}", current_round);
                    let dag = self.dag.read().await;
                    let ds = self.datastore.lock().await;
                    let consensus_state = &consensus.state;
                    let reputation_state = consensus.reputation.get_state();
                    
                    if let Err(e) = dag.create_checkpoint(
                        current_round,
                        consensus_state,
                        reputation_state,
                        &ds
                    ).await {
                        log::warn!("Failed to create checkpoint: {}", e);
                    }
                }
            }
            
            // Order and extract transactions
            let consensus_state = &consensus.state;
            let transactions = self.ordering
                .order_certificates(&consensus_state.committed)
                .await?;
            
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_sequencer(validator_index: usize) -> (ShoalSequencer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let datastore = NetworkDatastore::new(temp_dir.path())
            .unwrap();
        let datastore = Arc::new(Mutex::new(datastore));
        
        let config = ShoalSequencerConfig::new_test(4, validator_index);
        let sequencer = ShoalSequencer::new(datastore, config).await.unwrap();
        
        (sequencer, temp_dir)
    }
    
    #[tokio::test]
    async fn test_shoal_sequencer_create() {
        let (sequencer, _temp) = create_test_sequencer(0).await;
        sequencer.initialize().await.unwrap();
        
        assert_eq!(sequencer.get_current_round().await, 0);
        assert_eq!(sequencer.get_chain_tip().await, 0);
    }
    
    #[tokio::test]
    async fn test_shoal_sequencer_submit_transaction() {
        let (sequencer, _temp) = create_test_sequencer(0).await;
        sequencer.initialize().await.unwrap();
        
        let tx = Transaction {
            data: vec![1, 2, 3],
            timestamp: 1000,
        };
        
        sequencer.submit_transaction(tx).await.unwrap();
        assert_eq!(sequencer.pending_transaction_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_shoal_sequencer_propose_batch() {
        let (sequencer, _temp) = create_test_sequencer(0).await;
        sequencer.initialize().await.unwrap();
        
        // Submit some transactions
        for i in 0..5 {
            let tx = Transaction {
                data: vec![i],
                timestamp: 1000 + i as u64,
            };
            sequencer.submit_transaction(tx).await.unwrap();
        }
        
        // Propose batch (should form certificate and commit for genesis)
        let cert = sequencer.propose_batch().await.unwrap();
        assert!(cert.is_some());
        
        let cert = cert.unwrap();
        assert_eq!(cert.header.round, 0); // Genesis round
        
        // Should be committed
        assert_eq!(sequencer.get_chain_tip().await, 0);
    }
    
    #[tokio::test]
    async fn test_shoal_sequencer_advance_round() {
        let (sequencer, _temp) = create_test_sequencer(0).await;
        sequencer.initialize().await.unwrap();
        
        assert_eq!(sequencer.get_current_round().await, 0);
        
        sequencer.advance_round().await;
        assert_eq!(sequencer.get_current_round().await, 1);
        
        sequencer.advance_round().await;
        assert_eq!(sequencer.get_current_round().await, 2);
    }
}

