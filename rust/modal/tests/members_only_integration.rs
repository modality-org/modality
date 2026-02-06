//! Integration test: Members-Only Contract Lifecycle
//!
//! This test validates the FULL lifecycle of a members-only contract:
//! 1. Create contract
//! 2. Add first member (alice)
//! 3. Add model
//! 4. Add rules (any_signed, all_signed)
//! 5. Valid commits (member signs) - should PASS
//! 6. Invalid commits (non-member signs) - should FAIL
//! 7. Membership evolution (adding bob requires alice, adding carol requires both)
//!
//! This test exposes gaps in predicate evaluation wiring.

use tempfile::TempDir;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::path::PathBuf;

/// Test identity with signing capability
struct TestIdentity {
    name: String,
    signing_key: SigningKey,
    public_key_hex: String,
}

impl TestIdentity {
    fn new(name: &str) -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(verifying_key.as_bytes());
        
        Self {
            name: name.to_string(),
            signing_key,
            public_key_hex,
        }
    }
    
    fn sign(&self, message: &[u8]) -> String {
        let signature = self.signing_key.sign(message);
        hex::encode(signature.to_bytes())
    }
}

/// Test contract state tracker
struct TestContract {
    data_dir: PathBuf,
    contract_id: String,
    members: HashMap<String, String>, // name -> pubkey_hex
    commits: Vec<serde_json::Value>,
}

impl TestContract {
    fn new(data_dir: PathBuf, contract_id: &str) -> Self {
        let contract_dir = data_dir.join("contracts").join(contract_id);
        std::fs::create_dir_all(contract_dir.join("commits")).unwrap();
        std::fs::create_dir_all(contract_dir.join("state").join("members")).unwrap();
        
        Self {
            data_dir,
            contract_id: contract_id.to_string(),
            members: HashMap::new(),
            commits: Vec::new(),
        }
    }
    
    fn contract_dir(&self) -> PathBuf {
        self.data_dir.join("contracts").join(&self.contract_id)
    }
    
    /// Add a member to state (simulates POST /members/name.id)
    fn add_member_to_state(&mut self, identity: &TestIdentity) {
        let member_path = self.contract_dir()
            .join("state")
            .join("members")
            .join(format!("{}.id", identity.name));
        std::fs::write(&member_path, &identity.public_key_hex).unwrap();
        self.members.insert(identity.name.clone(), identity.public_key_hex.clone());
    }
    
    /// Get current member public keys
    fn get_member_keys(&self) -> Vec<String> {
        self.members.values().cloned().collect()
    }
    
    /// Create a commit with signatures
    fn create_signed_commit(
        &mut self,
        method: &str,
        path: &str,
        value: &str,
        action: Option<&str>,
        signers: &[&TestIdentity],
    ) -> serde_json::Value {
        let parent = self.commits.last()
            .and_then(|c| c.get("hash"))
            .and_then(|h| h.as_str())
            .map(|s| s.to_string());
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Build commit body
        let mut body_item = serde_json::json!({
            "method": method,
            "path": path,
            "value": value,
        });
        
        if let Some(act) = action {
            body_item["action"] = serde_json::json!(act);
        }
        
        let commit_data = serde_json::json!({
            "head": {
                "parent": parent,
                "timestamp": timestamp,
            },
            "body": [body_item],
        });
        
        // Create message to sign (canonical JSON of commit data)
        let message = serde_json::to_string(&commit_data).unwrap();
        let message_hex = hex::encode(message.as_bytes());
        
        // Collect signatures
        let signatures: Vec<serde_json::Value> = signers.iter()
            .map(|signer| {
                serde_json::json!({
                    "signer": signer.public_key_hex.clone(),
                    "signature": signer.sign(message.as_bytes()),
                })
            })
            .collect();
        
        // Compute commit hash
        let mut hasher = Sha256::new();
        hasher.update(message.as_bytes());
        let hash = hex::encode(hasher.finalize());
        
        let full_commit = serde_json::json!({
            "hash": hash,
            "head": commit_data["head"],
            "body": commit_data["body"],
            "message_hex": message_hex,
            "signatures": signatures,
        });
        
        self.commits.push(full_commit.clone());
        full_commit
    }
}

/// Validation result
#[derive(Debug)]
struct ValidationResult {
    valid: bool,
    errors: Vec<String>,
}

/// Validate a commit against rules
/// 
/// This is where the gap exists - we need to:
/// 1. Load current state (members list)
/// 2. Evaluate signature predicates (any_signed, all_signed)
/// 3. Check rule formulas with predicate results
fn validate_commit(
    contract: &TestContract,
    commit: &serde_json::Value,
    rules: &[String],
) -> ValidationResult {
    let signatures = commit.get("signatures")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    
    let message_hex = commit.get("message_hex")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    
    let member_keys = contract.get_member_keys();
    
    let mut errors = Vec::new();
    
    // Check any_signed if required by rules
    if rules.iter().any(|r| r.contains("any_signed")) {
        let any_valid = check_any_signed(&member_keys, message_hex, &signatures);
        if !any_valid {
            errors.push("any_signed(/members) failed: no valid member signature".to_string());
        }
    }
    
    // Check all_signed if required by rules (for specific actions)
    let action = commit.get("body")
        .and_then(|b| b.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("action"))
        .and_then(|a| a.as_str());
    
    if let Some(act) = action {
        if (act == "ADD_MEMBER" || act == "REMOVE_MEMBER") 
            && rules.iter().any(|r| r.contains("all_signed")) 
        {
            let all_valid = check_all_signed(&member_keys, message_hex, &signatures);
            if !all_valid {
                errors.push(format!(
                    "all_signed(/members) failed for {}: not all {} members signed",
                    act, member_keys.len()
                ));
            }
        }
    }
    
    ValidationResult {
        valid: errors.is_empty(),
        errors,
    }
}

/// Check if ANY member has validly signed
fn check_any_signed(
    members: &[String],
    message_hex: &str,
    signatures: &[serde_json::Value],
) -> bool {
    let message_bytes = match hex::decode(message_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    for sig_entry in signatures {
        let signer = match sig_entry.get("signer").and_then(|s| s.as_str()) {
            Some(s) => s,
            None => continue,
        };
        
        // Check if signer is a member
        if !members.contains(&signer.to_string()) {
            continue;
        }
        
        // Verify signature
        if verify_signature(signer, &sig_entry, &message_bytes) {
            return true;
        }
    }
    
    false
}

/// Check if ALL members have validly signed
fn check_all_signed(
    members: &[String],
    message_hex: &str,
    signatures: &[serde_json::Value],
) -> bool {
    use std::collections::HashSet;
    
    if members.is_empty() {
        return true; // Trivially satisfied
    }
    
    let message_bytes = match hex::decode(message_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    let mut signed_members: HashSet<String> = HashSet::new();
    
    for sig_entry in signatures {
        let signer = match sig_entry.get("signer").and_then(|s| s.as_str()) {
            Some(s) => s,
            None => continue,
        };
        
        // Check if signer is a member
        if !members.contains(&signer.to_string()) {
            continue;
        }
        
        // Verify signature
        if verify_signature(signer, &sig_entry, &message_bytes) {
            signed_members.insert(signer.to_string());
        }
    }
    
    // Check all members signed
    members.iter().all(|m| signed_members.contains(m))
}

/// Verify an ed25519 signature
fn verify_signature(
    signer_hex: &str,
    sig_entry: &serde_json::Value,
    message: &[u8],
) -> bool {
    use ed25519_dalek::{Signature, VerifyingKey, Verifier};
    
    let pubkey_bytes = match hex::decode(signer_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    let pubkey_array: [u8; 32] = match pubkey_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    
    let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
        Ok(k) => k,
        Err(_) => return false,
    };
    
    let sig_hex = match sig_entry.get("signature").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => return false,
    };
    
    let sig_bytes = match hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    let sig_array: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    
    let signature = Signature::from_bytes(&sig_array);
    
    verifying_key.verify(message, &signature).is_ok()
}

// ============================================================
// TESTS
// ============================================================

#[test]
fn test_members_only_full_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let mut contract = TestContract::new(temp_dir.path().to_path_buf(), "members-test-1");
    
    // Create identities
    let alice = TestIdentity::new("alice");
    let bob = TestIdentity::new("bob");
    let carol = TestIdentity::new("carol");
    let stranger = TestIdentity::new("stranger");
    
    // Rules we'll enforce
    let rules = vec![
        "always (any_signed(/members))".to_string(),
        "always ([+ADD_MEMBER] implies all_signed(/members))".to_string(),
    ];
    
    // Step 1: Add Alice as first member (before rules, so no validation yet)
    contract.add_member_to_state(&alice);
    println!("✓ Step 1: Alice added as first member");
    
    // Step 2: Alice creates a POST commit (should pass - she's a member)
    let commit1 = contract.create_signed_commit(
        "post", "/data/notes.md", "Hello world", Some("POST"), &[&alice]
    );
    let result1 = validate_commit(&contract, &commit1, &rules);
    assert!(result1.valid, "Alice's POST should pass: {:?}", result1.errors);
    println!("✓ Step 2: Alice's POST passed validation");
    
    // Step 3: Stranger tries to POST (should FAIL - not a member)
    let commit2 = contract.create_signed_commit(
        "post", "/data/hack.md", "Unauthorized", Some("POST"), &[&stranger]
    );
    let result2 = validate_commit(&contract, &commit2, &rules);
    assert!(!result2.valid, "Stranger's POST should fail");
    assert!(result2.errors[0].contains("any_signed"));
    println!("✓ Step 3: Stranger's POST correctly rejected");
    
    // Step 4: Alice adds Bob (only alice needs to sign - she's the only member)
    contract.add_member_to_state(&bob);
    let commit3 = contract.create_signed_commit(
        "post", "/members/bob.id", &bob.public_key_hex, Some("ADD_MEMBER"), &[&alice]
    );
    // Note: We added bob to state BEFORE validation for simplicity
    // In real flow, validation happens against PRE-commit state
    // For this test, we manually track that alice was the only member when she signed
    let pre_bob_members = vec![alice.public_key_hex.clone()];
    let result3 = check_all_signed(&pre_bob_members, 
        commit3.get("message_hex").and_then(|m| m.as_str()).unwrap(),
        commit3.get("signatures").and_then(|s| s.as_array()).unwrap()
    );
    assert!(result3, "Alice adding Bob should pass (she's the only member)");
    println!("✓ Step 4: Alice added Bob (only alice needed to sign)");
    
    // Step 5: Now adding Carol requires BOTH alice and bob
    let commit4_fail = contract.create_signed_commit(
        "post", "/members/carol.id", &carol.public_key_hex, Some("ADD_MEMBER"), &[&alice]
    );
    let pre_carol_members = vec![alice.public_key_hex.clone(), bob.public_key_hex.clone()];
    let result4_fail = check_all_signed(&pre_carol_members,
        commit4_fail.get("message_hex").and_then(|m| m.as_str()).unwrap(),
        commit4_fail.get("signatures").and_then(|s| s.as_array()).unwrap()
    );
    assert!(!result4_fail, "Adding Carol with only Alice's sig should fail");
    println!("✓ Step 5a: Partial signatures correctly rejected for ADD_MEMBER");
    
    // Step 6: Both alice and bob sign to add carol
    let commit5 = contract.create_signed_commit(
        "post", "/members/carol.id", &carol.public_key_hex, Some("ADD_MEMBER"), &[&alice, &bob]
    );
    let result5 = check_all_signed(&pre_carol_members,
        commit5.get("message_hex").and_then(|m| m.as_str()).unwrap(),
        commit5.get("signatures").and_then(|s| s.as_array()).unwrap()
    );
    assert!(result5, "Adding Carol with both Alice and Bob should pass");
    contract.add_member_to_state(&carol);
    println!("✓ Step 5b: Alice + Bob successfully added Carol");
    
    // Step 7: Bob can now POST (he's a member)
    let commit6 = contract.create_signed_commit(
        "post", "/data/bob-notes.md", "Bob's notes", Some("POST"), &[&bob]
    );
    let result6 = validate_commit(&contract, &commit6, &rules);
    assert!(result6.valid, "Bob's POST should pass: {:?}", result6.errors);
    println!("✓ Step 6: Bob's POST passed (he's now a member)");
    
    println!("\n✅ All members_only lifecycle tests passed!");
}

#[test]
fn test_any_signed_predicate_direct() {
    let alice = TestIdentity::new("alice");
    let bob = TestIdentity::new("bob");
    let stranger = TestIdentity::new("stranger");
    
    let members = vec![alice.public_key_hex.clone(), bob.public_key_hex.clone()];
    let message = b"test message";
    let message_hex = hex::encode(message);
    
    // Alice signs
    let alice_sig = serde_json::json!({
        "signer": alice.public_key_hex,
        "signature": alice.sign(message),
    });
    
    // Check any_signed with alice's signature
    let result = check_any_signed(&members, &message_hex, &[alice_sig.clone()]);
    assert!(result, "any_signed should pass with alice's signature");
    
    // Stranger signs
    let stranger_sig = serde_json::json!({
        "signer": stranger.public_key_hex,
        "signature": stranger.sign(message),
    });
    
    // Check any_signed with only stranger's signature
    let result = check_any_signed(&members, &message_hex, &[stranger_sig]);
    assert!(!result, "any_signed should fail with only stranger's signature");
}

#[test]
fn test_all_signed_predicate_direct() {
    let alice = TestIdentity::new("alice");
    let bob = TestIdentity::new("bob");
    
    let members = vec![alice.public_key_hex.clone(), bob.public_key_hex.clone()];
    let message = b"unanimous consent";
    let message_hex = hex::encode(message);
    
    let alice_sig = serde_json::json!({
        "signer": alice.public_key_hex,
        "signature": alice.sign(message),
    });
    
    let bob_sig = serde_json::json!({
        "signer": bob.public_key_hex,
        "signature": bob.sign(message),
    });
    
    // Only alice signs - should fail
    let result = check_all_signed(&members, &message_hex, &[alice_sig.clone()]);
    assert!(!result, "all_signed should fail with only alice");
    
    // Both sign - should pass
    let result = check_all_signed(&members, &message_hex, &[alice_sig, bob_sig]);
    assert!(result, "all_signed should pass with both alice and bob");
}

#[test]
fn test_membership_evolution() {
    // This test demonstrates how all_signed interpretation evolves
    let alice = TestIdentity::new("alice");
    let bob = TestIdentity::new("bob");
    let carol = TestIdentity::new("carol");
    
    let message = b"add member";
    let message_hex = hex::encode(message);
    
    // Phase 1: Only alice is member
    let members_v1 = vec![alice.public_key_hex.clone()];
    let alice_sig = serde_json::json!({
        "signer": alice.public_key_hex,
        "signature": alice.sign(message),
    });
    
    let result = check_all_signed(&members_v1, &message_hex, &[alice_sig.clone()]);
    assert!(result, "Phase 1: all_signed([alice]) should pass with alice's sig");
    
    // Phase 2: alice and bob are members
    let members_v2 = vec![alice.public_key_hex.clone(), bob.public_key_hex.clone()];
    let bob_sig = serde_json::json!({
        "signer": bob.public_key_hex,
        "signature": bob.sign(message),
    });
    
    // Only alice signs - should fail
    let result = check_all_signed(&members_v2, &message_hex, &[alice_sig.clone()]);
    assert!(!result, "Phase 2: all_signed([alice,bob]) should fail with only alice");
    
    // Both sign - should pass
    let result = check_all_signed(&members_v2, &message_hex, &[alice_sig.clone(), bob_sig.clone()]);
    assert!(result, "Phase 2: all_signed([alice,bob]) should pass with both");
    
    // Phase 3: alice, bob, carol are members
    let members_v3 = vec![
        alice.public_key_hex.clone(), 
        bob.public_key_hex.clone(),
        carol.public_key_hex.clone(),
    ];
    let carol_sig = serde_json::json!({
        "signer": carol.public_key_hex,
        "signature": carol.sign(message),
    });
    
    // Alice and bob sign - should fail (missing carol)
    let result = check_all_signed(&members_v3, &message_hex, &[alice_sig.clone(), bob_sig.clone()]);
    assert!(!result, "Phase 3: all_signed([a,b,c]) should fail without carol");
    
    // All three sign - should pass
    let result = check_all_signed(&members_v3, &message_hex, &[alice_sig, bob_sig, carol_sig]);
    assert!(result, "Phase 3: all_signed([a,b,c]) should pass with all three");
    
    println!("✅ Membership evolution test passed - rule interpretation evolves with state");
}
