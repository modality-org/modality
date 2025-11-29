use crate::narwhal::{CertificateDigest, Transaction};
use crate::narwhal::dag::DAG;
use anyhow::Result;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Engine for ordering committed certificates into a linear sequence
pub struct OrderingEngine {
    dag: Arc<RwLock<DAG>>,
}

impl OrderingEngine {
    /// Create a new ordering engine
    pub fn new(dag: Arc<RwLock<DAG>>) -> Self {
        Self { dag }
    }

    /// Order a set of committed certificates and extract transactions
    pub async fn order_certificates(
        &self,
        committed: &BTreeSet<CertificateDigest>,
    ) -> Result<Vec<Transaction>> {
        let dag = self.dag.read().await;

        // Topological sort of committed certificates
        let ordered_certs = self.topological_sort(&*dag, committed)?;

        // Extract transactions from ordered certificates
        let transactions = Vec::new();
        for cert_digest in ordered_certs {
            if let Some(cert) = dag.get(&cert_digest) {
                // Note: In a real implementation, we would fetch the actual batch
                // For now, we log that we would process this batch
                log::debug!(
                    "would extract transactions from batch {:?} in cert {}",
                    cert.header.batch_digest,
                    hex::encode(&cert_digest)
                );
                
                // Placeholder: actual batch fetching would go here
                // transactions.extend(fetch_batch(cert.header.batch_digest).transactions);
            }
        }

        Ok(transactions)
    }

    /// Perform topological sort on committed certificates
    fn topological_sort(
        &self,
        dag: &DAG,
        committed: &BTreeSet<CertificateDigest>,
    ) -> Result<Vec<CertificateDigest>> {
        // Kahn's algorithm with deterministic tie-breaking
        
        // Build in-degree map
        let mut in_degree: HashMap<CertificateDigest, usize> = HashMap::new();
        let mut adjacency: HashMap<CertificateDigest, Vec<CertificateDigest>> = HashMap::new();

        for &cert_digest in committed {
            in_degree.entry(cert_digest).or_insert(0);
            
            if let Some(cert) = dag.get(&cert_digest) {
                // Add edges from parents to this certificate
                for parent_digest in &cert.header.parents {
                    if committed.contains(parent_digest) {
                        adjacency
                            .entry(*parent_digest)
                            .or_default()
                            .push(cert_digest);
                        *in_degree.entry(cert_digest).or_insert(0) += 1;
                    }
                }
            }
        }

        // Find all nodes with in-degree 0
        let mut queue: Vec<CertificateDigest> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&digest, _)| digest)
            .collect();

        // Sort queue for deterministic ordering
        self.sort_by_round_and_author(dag, &mut queue);

        let mut result = Vec::new();
        let mut visited = HashSet::new();

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);
            result.push(current);

            // Process neighbors
            if let Some(neighbors) = adjacency.get(&current) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(&neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }

            // Re-sort queue for deterministic tie-breaking
            self.sort_by_round_and_author(dag, &mut queue);
        }

        // Check if all nodes were processed (no cycles)
        if result.len() != committed.len() {
            anyhow::bail!("cycle detected in DAG or missing certificates");
        }

        Ok(result)
    }

    /// Sort certificates deterministically by (round, author)
    fn sort_by_round_and_author(&self, dag: &DAG, certs: &mut Vec<CertificateDigest>) {
        certs.sort_by(|a, b| {
            let cert_a = dag.get(a);
            let cert_b = dag.get(b);

            match (cert_a, cert_b) {
                (Some(ca), Some(cb)) => {
                    // Sort by round first (lower first)
                    ca.header.round
                        .cmp(&cb.header.round)
                        // Then by author (lexicographic)
                        .then_with(|| ca.header.author.cmp(&cb.header.author))
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narwhal::PublicKey;
    use crate::narwhal::{AggregatedSignature, Certificate, Header};

    fn make_test_cert(author: crate::narwhal::PublicKey,
        round: u64,
        parents: Vec<CertificateDigest>,
    ) -> Certificate {
        Certificate {
            header: Header {
                author,
                round,
                batch_digest: [0u8; 32],
                parents,
                timestamp: 1000 + round * 1000,
            },
            aggregated_signature: AggregatedSignature {
                signature: vec![],
            },
            signers: vec![true, true, true, false],
        }
    }

    #[tokio::test]
    async fn test_ordering_single_certificate() {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let engine = OrderingEngine::new(dag.clone());

        let cert = make_test_cert(vec![1], 0, vec![]);
        let digest = cert.digest();
        
        dag.write().await.insert(cert).unwrap();

        let mut committed = BTreeSet::new();
        committed.insert(digest);

        let ordered = engine.topological_sort(&*dag.read().await, &committed).unwrap();
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0], digest);
    }

    #[tokio::test]
    async fn test_ordering_linear_chain() {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let engine = OrderingEngine::new(dag.clone());

        // Build chain: cert0 -> cert1 -> cert2
        let cert0 = make_test_cert(vec![1], 0, vec![]);
        let digest0 = cert0.digest();
        dag.write().await.insert(cert0).unwrap();

        let cert1 = make_test_cert(vec![2], 1, vec![digest0]);
        let digest1 = cert1.digest();
        dag.write().await.insert(cert1).unwrap();

        let cert2 = make_test_cert(vec![3], 2, vec![digest1]);
        let digest2 = cert2.digest();
        dag.write().await.insert(cert2).unwrap();

        let mut committed = BTreeSet::new();
        committed.insert(digest0);
        committed.insert(digest1);
        committed.insert(digest2);

        let ordered = engine.topological_sort(&*dag.read().await, &committed).unwrap();
        assert_eq!(ordered.len(), 3);
        // Should be in order: 0, 1, 2
        assert_eq!(ordered[0], digest0);
        assert_eq!(ordered[1], digest1);
        assert_eq!(ordered[2], digest2);
    }

    #[tokio::test]
    async fn test_ordering_deterministic() {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let engine = OrderingEngine::new(dag.clone());

        // Create 3 certificates in round 0 (no causal relationship)
        let cert1 = make_test_cert(vec![1], 0, vec![]);
        let digest1 = cert1.digest();
        dag.write().await.insert(cert1).unwrap();

        let cert2 = make_test_cert(vec![2], 0, vec![]);
        let digest2 = cert2.digest();
        dag.write().await.insert(cert2).unwrap();

        let cert3 = make_test_cert(vec![3], 0, vec![]);
        let digest3 = cert3.digest();
        dag.write().await.insert(cert3).unwrap();

        let mut committed = BTreeSet::new();
        committed.insert(digest1);
        committed.insert(digest2);
        committed.insert(digest3);

        // Order multiple times - should be deterministic
        let ordered1 = engine.topological_sort(&*dag.read().await, &committed).unwrap();
        let ordered2 = engine.topological_sort(&*dag.read().await, &committed).unwrap();
        
        assert_eq!(ordered1, ordered2);
        assert_eq!(ordered1.len(), 3);
    }

    #[tokio::test]
    async fn test_ordering_dag_structure() {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let engine = OrderingEngine::new(dag.clone());

        // Build DAG:
        //     cert0
        //    /     \
        // cert1   cert2
        //    \     /
        //     cert3

        let cert0 = make_test_cert(vec![1], 0, vec![]);
        let digest0 = cert0.digest();
        dag.write().await.insert(cert0).unwrap();

        let cert1 = make_test_cert(vec![2], 1, vec![digest0]);
        let digest1 = cert1.digest();
        dag.write().await.insert(cert1).unwrap();

        let cert2 = make_test_cert(vec![3], 1, vec![digest0]);
        let digest2 = cert2.digest();
        dag.write().await.insert(cert2).unwrap();

        let cert3 = make_test_cert(vec![4], 2, vec![digest1, digest2]);
        let digest3 = cert3.digest();
        dag.write().await.insert(cert3).unwrap();

        let mut committed = BTreeSet::new();
        committed.insert(digest0);
        committed.insert(digest1);
        committed.insert(digest2);
        committed.insert(digest3);

        let ordered = engine.topological_sort(&*dag.read().await, &committed).unwrap();
        assert_eq!(ordered.len(), 4);
        
        // cert0 must come first
        assert_eq!(ordered[0], digest0);
        
        // cert3 must come last
        assert_eq!(ordered[3], digest3);
        
        // cert1 and cert2 must come after cert0 and before cert3
        let pos1 = ordered.iter().position(|&d| d == digest1).unwrap();
        let pos2 = ordered.iter().position(|&d| d == digest2).unwrap();
        assert!(pos1 > 0 && pos1 < 3);
        assert!(pos2 > 0 && pos2 < 3);
    }
}
