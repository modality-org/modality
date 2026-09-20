/// Performance benchmarks for Shoal consensus implementation
///
/// Measures:
/// - Certificate formation throughput
/// - DAG insertion performance
/// - Consensus decision latency
/// - Multi-validator throughput
/// - Reputation updates
/// - Transaction ordering

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use modal_validator_consensus::narwhal::{
    AggregatedSignature, Certificate, Committee, Header, Transaction, Validator, Worker,
};
use modal_validator_consensus::narwhal::certificate::CertificateBuilder;
use modal_validator_consensus::narwhal::dag::DAG;
use modal_validator_consensus::shoal::{PerformanceRecord, ReputationConfig};
use modal_validator_consensus::shoal::reputation::ReputationManager;
use modal_validator_consensus::shoal::consensus::ShoalConsensus;
use modal_validator_consensus::shoal::ordering::OrderingEngine;
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper to create a test committee
fn create_committee(n: usize) -> Committee {
    let validators: Vec<Validator> = (0..n)
        .map(|i| Validator {
            public_key: vec![i as u8],
            stake: 1,
            network_address: SocketAddr::from(([127, 0, 0, 1], 8000 + i as u16)),
        })
        .collect();
    Committee::new(validators)
}

/// Helper to create test certificate
fn create_cert(author: u8, round: u64, parents: Vec<[u8; 32]>, committee_size: usize) -> Certificate {
    Certificate {
        header: Header {
            author: vec![author],
            round,
            batch_digest: [round as u8; 32],
            parents,
            timestamp: 1000 + round * 1000,
        },
        aggregated_signature: AggregatedSignature {
            signature: vec![],
        },
        signers: vec![true; committee_size],
    }
}

/// Benchmark certificate formation (vote collection)
fn bench_certificate_formation(c: &mut Criterion) {
    let mut group = c.benchmark_group("certificate_formation");
    
    for validator_count in [4, 7, 10, 16].iter() {
        let committee = create_committee(*validator_count);
        let header = Header {
            author: vec![0],
            round: 1,
            batch_digest: [0u8; 32],
            parents: vec![],
            timestamp: 1000,
        };
        
        group.bench_with_input(
            BenchmarkId::new("form_certificate", validator_count),
            validator_count,
            |b, _| {
                b.iter(|| {
                    let mut builder = CertificateBuilder::new(header.clone(), committee.clone());
                    
                    // Add quorum votes
                    let quorum = committee.quorum_threshold() as usize;
                    for i in 0..quorum {
                        builder.add_vote(vec![i as u8], vec![]).unwrap();
                    }
                    
                    builder.build().unwrap()
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark DAG insertion performance
fn bench_dag_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_insertion");
    group.throughput(Throughput::Elements(1));
    
    let _committee = create_committee(4);
    
    for round in [0, 10, 100].iter() {
        let mut dag = DAG::new();
        let mut prev_round_digests = Vec::new();
        
        // Pre-populate DAG with previous rounds
        for r in 0..*round {
            let mut current_round_digests = Vec::new();
            for i in 0..4 {
                let parents = if r == 0 {
                    vec![]
                } else {
                    prev_round_digests.clone()
                };
                let cert = create_cert(i, r, parents, 4);
                let digest = cert.digest();
                current_round_digests.push(digest);
                dag.insert(cert).unwrap();
            }
            prev_round_digests = current_round_digests;
        }
        
        group.bench_with_input(
            BenchmarkId::new("insert_cert", round),
            round,
            |b, _| {
                let mut local_dag = dag.clone();
                let mut counter = 0u8;
                b.iter(|| {
                    let parents = if *round == 0 {
                        vec![]
                    } else {
                        prev_round_digests.clone()
                    };
                    let cert = create_cert(counter % 4, *round, parents, 4);
                    counter = counter.wrapping_add(1);
                    local_dag.insert(cert).unwrap();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark path finding in DAG
fn bench_dag_path_finding(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_path_finding");
    
    for chain_length in [10, 50, 100].iter() {
        let mut dag = DAG::new();
        let mut digests = Vec::new();
        
        // Build a chain
        for r in 0..*chain_length {
            let parents = if r == 0 {
                vec![]
            } else {
                vec![digests[r as usize - 1]]
            };
            let cert = create_cert(0, r, parents, 4);
            let digest = cert.digest();
            digests.push(digest);
            dag.insert(cert).unwrap();
        }
        
        group.bench_with_input(
            BenchmarkId::new("has_path", chain_length),
            chain_length,
            |b, _| {
                b.iter(|| {
                    dag.has_path(&digests[*chain_length as usize - 1], &digests[0])
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark consensus decision making
fn bench_consensus_processing(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("consensus_processing");
    group.throughput(Throughput::Elements(1));
    
    for validator_count in [4, 7, 10].iter() {
        let committee = create_committee(*validator_count);
        
        group.bench_with_input(
            BenchmarkId::new("process_genesis_cert", validator_count),
            validator_count,
            |b, _| {
                b.to_async(&runtime).iter(|| async {
                    let dag = Arc::new(RwLock::new(DAG::new()));
                    let reputation = ReputationManager::new(
                        committee.clone(),
                        ReputationConfig::default(),
                    );
                    let mut consensus = ShoalConsensus::new(
                        dag.clone(),
                        reputation,
                        committee.clone(),
                    );
                    
                    let cert = create_cert(0, 0, vec![], *validator_count);
                    consensus.process_certificate(cert).await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark reputation updates
fn bench_reputation_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("reputation_updates");
    group.throughput(Throughput::Elements(1));
    
    for validator_count in [4, 10, 25, 50].iter() {
        let committee = create_committee(*validator_count);
        let config = ReputationConfig::default();
        let mut reputation = ReputationManager::new(committee.clone(), config);
        
        // Pre-populate with performance records
        for round in 0..10 {
            for i in 0..*validator_count {
                reputation.record_performance(PerformanceRecord {
                    validator: vec![i as u8],
                    round,
                    latency_ms: 500,
                    success: true,
                    timestamp: 1000 + round * 1000,
                });
            }
        }
        
        group.bench_with_input(
            BenchmarkId::new("update_scores", validator_count),
            validator_count,
            |b, _| {
                let mut local_rep = reputation.clone();
                b.iter(|| {
                    local_rep.update_scores();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark leader selection
fn bench_leader_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("leader_selection");
    
    for validator_count in [4, 10, 25, 50, 100].iter() {
        let committee = create_committee(*validator_count);
        let config = ReputationConfig::default();
        let reputation = ReputationManager::new(committee.clone(), config);
        
        group.bench_with_input(
            BenchmarkId::new("select_leader", validator_count),
            validator_count,
            |b, _| {
                b.iter(|| {
                    reputation.select_leader(1);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark transaction ordering (topological sort)
fn bench_transaction_ordering(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("transaction_ordering");
    
    for cert_count in [10, 50, 100, 500].iter() {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let mut committed = BTreeSet::new();
        let mut prev_digest = None;
        
        runtime.block_on(async {
            let mut dag_write = dag.write().await;
            
            // Build a DAG with cert_count certificates
            for r in 0..*cert_count {
                let parents = if r == 0 {
                    vec![]
                } else {
                    // Reference actual previous certificate
                    vec![prev_digest.unwrap()]
                };
                let cert = create_cert(0, r, parents, 4);
                let digest = cert.digest();
                prev_digest = Some(digest);
                committed.insert(digest);
                dag_write.insert(cert).unwrap();
            }
        });
        
        let ordering = OrderingEngine::new(dag.clone());
        
        group.bench_with_input(
            BenchmarkId::new("order_certificates", cert_count),
            cert_count,
            |b, _| {
                b.to_async(&runtime).iter(|| async {
                    ordering.order_certificates(&committed).await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark worker batch formation
fn bench_worker_batch_formation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("worker_batch_formation");
    group.throughput(Throughput::Elements(1));
    
    for tx_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("form_batch", tx_count),
            tx_count,
            |b, _| {
                b.to_async(&runtime).iter(|| async {
                    let mut worker = Worker::new(0, vec![0], 1000, 512 * 1024);
                    
                    // Add transactions
                    for i in 0..*tx_count {
                        worker.add_transaction(Transaction {
                            data: vec![i as u8; 100], // 100 bytes per tx
                            timestamp: 1000 + i,
                        });
                    }
                    
                    worker.form_batch().await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

/// End-to-end throughput benchmark
fn bench_end_to_end_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end_throughput");
    group.sample_size(20); // Fewer samples for longer benchmark
    
    for validator_count in [4, 7].iter() {
        group.bench_with_input(
            BenchmarkId::new("multi_round_consensus", validator_count),
            validator_count,
            |b, count| {
                b.to_async(&runtime).iter(|| async {
                    let committee = create_committee(*count);
                    let dag = Arc::new(RwLock::new(DAG::new()));
                    let reputation = ReputationManager::new(
                        committee.clone(),
                        ReputationConfig::default(),
                    );
                    let mut consensus = ShoalConsensus::new(
                        dag.clone(),
                        reputation,
                        committee.clone(),
                    );
                    
                    // Process 10 rounds with all validators
                    for round in 0..10 {
                        // Get parents from previous round
                        let parents: Vec<[u8; 32]> = if round == 0 {
                            vec![]
                        } else {
                            let dag_read = dag.read().await;
                            dag_read
                                .get_round(round - 1)
                                .iter()
                                .map(|c| c.digest())
                                .collect()
                        };
                        
                        // All validators propose
                        for i in 0..*count {
                            let cert = create_cert(i as u8, round, parents.clone(), *count);
                            consensus.process_certificate(cert).await.unwrap();
                        }
                        
                        if round < 9 {
                            consensus.advance_round();
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_certificate_formation,
    bench_dag_insertion,
    bench_dag_path_finding,
    bench_consensus_processing,
    bench_reputation_updates,
    bench_leader_selection,
    bench_transaction_ordering,
    bench_worker_batch_formation,
    bench_end_to_end_throughput,
);

criterion_main!(benches);

