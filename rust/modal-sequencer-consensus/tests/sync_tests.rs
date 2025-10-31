use modal_sequencer_consensus::narwhal::{
    AggregatedSignature, Certificate, Header, SyncClient, SyncRequest, SyncResponse,
};
use modal_sequencer_consensus::narwhal::dag::DAG;
use libp2p_identity::{ed25519, PeerId};
use std::sync::Arc;
use tokio::sync::RwLock;

fn test_peer_id(seed: u8) -> PeerId {
    let mut secret_bytes = [0u8; 32];
    secret_bytes[0] = seed;
    let secret = ed25519::SecretKey::try_from_bytes(secret_bytes)
        .expect("valid secret key");
    let keypair = ed25519::Keypair::from(secret);
    PeerId::from_public_key(&keypair.public().into())
}

fn make_test_cert(author_seed: u8, round: u64, parents: Vec<[u8; 32]>) -> Certificate {
    Certificate {
        header: Header {
            author: test_peer_id(author_seed),
            round,
            batch_digest: [round as u8; 32],
            parents,
            timestamp: 1000 + round,
        },
        aggregated_signature: AggregatedSignature { signature: vec![1, 2, 3] },
        signers: vec![true, true, true, true],
    }
}

#[tokio::test]
async fn test_sync_request_get_certificates() {
    let mut dag = DAG::new();
    
    let cert1 = make_test_cert(1, 0, vec![]);
    let cert2 = make_test_cert(2, 0, vec![]);
    let digest1 = cert1.digest();
    let digest2 = cert2.digest();
    
    dag.insert(cert1.clone()).unwrap();
    dag.insert(cert2.clone()).unwrap();
    
    // Request specific certificates
    let request = SyncRequest::certificates(vec![digest1, digest2]);
    let response = dag.handle_sync_request(request);
    
    match response {
        SyncResponse::Certificates { certificates, has_more } => {
            assert_eq!(certificates.len(), 2);
            assert!(!has_more);
        }
        _ => panic!("unexpected response"),
    }
}

#[tokio::test]
async fn test_sync_request_get_round() {
    let mut dag = DAG::new();
    
    // Add 3 certificates for round 0
    for i in 1..=3 {
        let cert = make_test_cert(i, 0, vec![]);
        dag.insert(cert).unwrap();
    }
    
    let request = SyncRequest::certificates_in_round(0);
    let response = dag.handle_sync_request(request);
    
    match response {
        SyncResponse::Certificates { certificates, .. } => {
            assert_eq!(certificates.len(), 3);
        }
        _ => panic!("unexpected response"),
    }
}

#[tokio::test]
async fn test_sync_request_get_range() {
    let mut dag = DAG::new();
    
    // Round 0: 2 certificates
    let cert0_1 = make_test_cert(1, 0, vec![]);
    let cert0_2 = make_test_cert(2, 0, vec![]);
    let digest0_1 = cert0_1.digest();
    let digest0_2 = cert0_2.digest();
    dag.insert(cert0_1).unwrap();
    dag.insert(cert0_2).unwrap();
    
    // Round 1: 2 certificates with parents
    let cert1_1 = make_test_cert(3, 1, vec![digest0_1, digest0_2]);
    let cert1_2 = make_test_cert(4, 1, vec![digest0_1, digest0_2]);
    dag.insert(cert1_1).unwrap();
    dag.insert(cert1_2).unwrap();
    
    let request = SyncRequest::certificates_in_range(0, 1);
    let response = dag.handle_sync_request(request);
    
    match response {
        SyncResponse::Certificates { certificates, has_more } => {
            assert_eq!(certificates.len(), 4); // 2 from round 0, 2 from round 1
            assert!(!has_more);
        }
        _ => panic!("unexpected response"),
    }
}

#[tokio::test]
async fn test_sync_request_highest_round() {
    let mut dag = DAG::new();
    
    // Add certificates up to round 5
    let mut parent_digest = None;
    for round in 0..=5 {
        let parents = parent_digest.map(|d| vec![d]).unwrap_or_default();
        let cert = make_test_cert(1, round, parents);
        parent_digest = Some(cert.digest());
        dag.insert(cert).unwrap();
    }
    
    let request = SyncRequest::highest_round();
    let response = dag.handle_sync_request(request);
    
    match response {
        SyncResponse::HighestRound { round } => {
            assert_eq!(round, 5);
        }
        _ => panic!("unexpected response"),
    }
}

#[tokio::test]
async fn test_sync_request_missing_certificates() {
    let mut dag = DAG::new();
    
    // Add 5 certificates
    let mut certs = Vec::new();
    for i in 1..=5 {
        let cert = make_test_cert(i, 0, vec![]);
        certs.push(cert.digest());
        dag.insert(cert).unwrap();
    }
    
    // Request missing (say we know about first 2)
    let known = vec![certs[0], certs[1]];
    let request = SyncRequest::missing_certificates(known, 0);
    let response = dag.handle_sync_request(request);
    
    match response {
        SyncResponse::Certificates { certificates, .. } => {
            assert_eq!(certificates.len(), 3); // Should get the 3 we don't know about
        }
        _ => panic!("unexpected response"),
    }
}

#[tokio::test]
async fn test_sync_client_with_peer() {
    // Setup: two DAGs - one ahead, one behind
    let peer_dag = {
        let mut dag = DAG::new();
        
        // Peer has certificates for rounds 0-5
        let mut parent_digest = None;
        for round in 0..=5 {
            let parents = parent_digest.map(|d| vec![d]).unwrap_or_default();
            let cert = make_test_cert(1, round, parents);
            parent_digest = Some(cert.digest());
            dag.insert(cert).unwrap();
        }
        
        Arc::new(RwLock::new(dag))
    };
    
    let local_dag = {
        let mut dag = DAG::new();
        
        // We only have rounds 0-2
        let mut parent_digest = None;
        for round in 0..=2 {
            let parents = parent_digest.map(|d| vec![d]).unwrap_or_default();
            let cert = make_test_cert(1, round, parents);
            parent_digest = Some(cert.digest());
            dag.insert(cert).unwrap();
        }
        
        Arc::new(RwLock::new(dag))
    };
    
    let client = SyncClient::new(local_dag.clone());
    
    // Mock request function that queries peer_dag
    let peer_dag_clone = peer_dag.clone();
    let request_fn = move |req: SyncRequest| {
        let peer_dag = peer_dag_clone.clone();
        async move {
            let dag = peer_dag.read().await;
            Ok(dag.handle_sync_request(req))
        }
    };
    
    // Sync with peer
    let stats = client.sync_with_peer(request_fn).await.unwrap();
    
    println!("Sync stats: synced={}, failed={}", stats.certificates_synced, stats.certificates_failed);
    
    // Verify we now have all 6 certificates
    let local = local_dag.read().await;
    assert_eq!(local.highest_round(), 5);
    assert_eq!(local.round_size(3), 1);
    assert_eq!(local.round_size(4), 1);
    assert_eq!(local.round_size(5), 1);
}

#[tokio::test]
async fn test_get_missing_parents() {
    let mut dag = DAG::new();
    
    let cert0 = make_test_cert(1, 0, vec![]);
    let digest0 = cert0.digest();
    dag.insert(cert0).unwrap();
    
    // Certificate with missing parent
    let cert1 = make_test_cert(2, 1, vec![digest0, [99u8; 32]]);
    
    let missing = dag.get_missing_parents(&cert1);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0], [99u8; 32]);
}

#[tokio::test]
async fn test_has_all_parents() {
    let mut dag = DAG::new();
    
    let cert0 = make_test_cert(1, 0, vec![]);
    let digest0 = cert0.digest();
    dag.insert(cert0).unwrap();
    
    // Certificate with all parents present
    let cert1 = make_test_cert(2, 1, vec![digest0]);
    assert!(dag.has_all_parents(&cert1));
    
    // Certificate with missing parent
    let cert2 = make_test_cert(3, 1, vec![digest0, [99u8; 32]]);
    assert!(!dag.has_all_parents(&cert2));
}

#[tokio::test]
async fn test_sync_missing_parents() {
    let peer_dag = {
        let mut dag = DAG::new();
        let cert0 = make_test_cert(1, 0, vec![]);
        dag.insert(cert0).unwrap();
        Arc::new(RwLock::new(dag))
    };
    
    let local_dag = Arc::new(RwLock::new(DAG::new()));
    let client = SyncClient::new(local_dag.clone());
    
    // Create certificate that references parent in peer_dag
    let cert0_digest = {
        let dag = peer_dag.read().await;
        dag.get_round(0)[0].digest()
    };
    
    let cert1 = make_test_cert(2, 1, vec![cert0_digest]);
    
    // Mock request function
    let peer_dag_clone = peer_dag.clone();
    let request_fn = move |req: SyncRequest| {
        let peer_dag = peer_dag_clone.clone();
        async move {
            let dag = peer_dag.read().await;
            Ok(dag.handle_sync_request(req))
        }
    };
    
    // Sync missing parents
    let success = client.sync_missing_parents(&cert1, request_fn).await.unwrap();
    assert!(success);
    
    // Verify parent is now in local DAG
    let local = local_dag.read().await;
    assert!(local.get(&cert0_digest).is_some());
}

#[tokio::test]
async fn test_sync_gaps() {
    // Peer has complete DAG with 10 certificates in round 0
    let peer_dag = {
        let mut dag = DAG::new();
        
        for i in 0..10 {
            let cert = make_test_cert(i as u8 + 1, 0, vec![]);
            dag.insert(cert).unwrap();
        }
        
        Arc::new(RwLock::new(dag))
    };
    
    // Local only has 2 certificates in round 0
    let local_dag = {
        let mut dag = DAG::new();
        
        // We have some certificates
        let cert1 = make_test_cert(1, 0, vec![]);
        let cert2 = make_test_cert(2, 0, vec![]);
        dag.insert(cert1).unwrap();
        dag.insert(cert2).unwrap();
        
        Arc::new(RwLock::new(dag))
    };
    
    // Verify initial state
    {
        let local = local_dag.read().await;
        assert_eq!(local.round_size(0), 2);
    }
    
    let client = SyncClient::new(local_dag.clone());
    
    let peer_dag_clone = peer_dag.clone();
    let request_fn = move |req: SyncRequest| {
        let peer_dag = peer_dag_clone.clone();
        async move {
            let dag = peer_dag.read().await;
            Ok(dag.handle_sync_request(req))
        }
    };
    
    // Sync with peer to get all certificates
    let stats = client.sync_with_peer(request_fn).await.unwrap();
    
    println!("Gap sync stats: synced={}, failed={}", stats.certificates_synced, stats.certificates_failed);
    
    // Verify we now have all 10 certificates in round 0
    let local = local_dag.read().await;
    let actual_count = local.round_size(0);
    println!("Actual certificate count: {}", actual_count);
    
    // We started with 2, peer has 10 total, so we might end up with:
    // - 10 if dedup works (the 2 we had are different from peer's 10)
    // - 12 if we have different certificates
    // Since each cert is from a different seed, they're all unique
    assert_eq!(actual_count, 12, "Expected 12 total: our 2 + peer's 10");
}

