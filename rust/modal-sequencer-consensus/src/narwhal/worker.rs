use crate::narwhal::{Batch, BatchDigest, PublicKey, Transaction, WorkerId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Worker node that collects transactions and forms batches
pub struct Worker {
    /// Worker ID
    pub id: WorkerId,
    /// Validator this worker belongs to
    pub validator: PublicKey,
    /// Maximum number of transactions per batch
    pub batch_size: usize,
    /// Maximum batch size in bytes
    pub max_batch_bytes: usize,
    /// Buffer of pending transactions
    tx_buffer: Vec<Transaction>,
    /// Storage for batches (digest -> batch)
    storage: Arc<Mutex<HashMap<BatchDigest, Batch>>>,
}

impl Worker {
    /// Create a new worker
    pub fn new(
        id: WorkerId,
        validator: PublicKey,
        batch_size: usize,
        max_batch_bytes: usize,
    ) -> Self {
        Self {
            id,
            validator,
            batch_size,
            max_batch_bytes,
            tx_buffer: Vec::new(),
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a transaction to the buffer
    pub fn add_transaction(&mut self, tx: Transaction) {
        self.tx_buffer.push(tx);
    }

    /// Form a batch from buffered transactions
    pub async fn form_batch(&mut self) -> Option<(Batch, BatchDigest)> {
        if self.tx_buffer.is_empty() {
            return None;
        }

        // Take transactions up to batch_size
        let transactions: Vec<Transaction> = self.tx_buffer
            .drain(..self.tx_buffer.len().min(self.batch_size))
            .collect();

        if transactions.is_empty() {
            return None;
        }

        let batch = Batch {
            transactions,
            worker_id: self.id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let digest = batch.digest();

        // Store batch
        let mut storage = self.storage.lock().await;
        storage.insert(digest, batch.clone());

        Some((batch, digest))
    }

    /// Serve a batch by digest (for availability protocol)
    pub async fn serve_batch(&self, digest: BatchDigest) -> Option<Batch> {
        let storage = self.storage.lock().await;
        storage.get(&digest).cloned()
    }

    /// Get the number of pending transactions
    pub fn pending_count(&self) -> usize {
        self.tx_buffer.len()
    }
}

#[cfg(test)]
mod tests {

    /// Helper to create a deterministic PeerId for testing
    fn test_peer_id(seed: u8) -> libp2p_identity::PeerId {
        use libp2p_identity::ed25519;
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = seed;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes).expect("valid secret key");
        let keypair = ed25519::Keypair::from(secret);
        libp2p_identity::PeerId::from_public_key(&keypair.public().into())
    }
    use super::*;

    #[tokio::test]
    async fn test_worker_add_transaction() {
        let mut worker = Worker::new(0, test_peer_id(1), 100, 1024 * 512);
        
        worker.add_transaction(Transaction {
            data: vec![1, 2, 3],
            timestamp: 1000,
        });
        
        assert_eq!(worker.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_worker_form_batch() {
        let mut worker = Worker::new(0, test_peer_id(1), 100, 1024 * 512);
        
        // Add some transactions
        for i in 0..5 {
            worker.add_transaction(Transaction {
                data: vec![i],
                timestamp: 1000 + i as u64,
            });
        }
        
        let result = worker.form_batch().await;
        assert!(result.is_some());
        
        let (batch, _digest) = result.unwrap();
        assert_eq!(batch.transactions.len(), 5);
        assert_eq!(batch.worker_id, 0);
        assert_eq!(worker.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_worker_serve_batch() {
        let mut worker = Worker::new(0, test_peer_id(1), 100, 1024 * 512);
        
        worker.add_transaction(Transaction {
            data: vec![1, 2, 3],
            timestamp: 1000,
        });
        
        let (batch, digest) = worker.form_batch().await.unwrap();
        
        // Serve the batch
        let served = worker.serve_batch(digest).await;
        assert!(served.is_some());
        assert_eq!(served.unwrap().transactions.len(), batch.transactions.len());
        
        // Serve non-existent batch
        let not_found = worker.serve_batch([99u8; 32]).await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_worker_batch_size_limit() {
        let mut worker = Worker::new(0, test_peer_id(1), 3, 1024 * 512); // Max 3 transactions
        
        // Add 5 transactions
        for i in 0..5 {
            worker.add_transaction(Transaction {
                data: vec![i],
                timestamp: 1000 + i as u64,
            });
        }
        
        // First batch should have only 3
        let (batch1, _) = worker.form_batch().await.unwrap();
        assert_eq!(batch1.transactions.len(), 3);
        assert_eq!(worker.pending_count(), 2);
        
        // Second batch should have remaining 2
        let (batch2, _) = worker.form_batch().await.unwrap();
        assert_eq!(batch2.transactions.len(), 2);
        assert_eq!(worker.pending_count(), 0);
    }
}

