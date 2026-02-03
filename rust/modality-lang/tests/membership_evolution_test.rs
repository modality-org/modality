//! Test: Model Evolution via Membership Growth
//!
//! This test demonstrates a key Modality principle:
//! The same rule can require different signatures as the membership set grows.
//! The model "evolves" not because rules change, but because state changes.
//!
//! Scenario:
//! 1. Alice creates a blank contract
//! 2. Alice adds herself to /members/alice.id
//! 3. Alice adds a rule: "modifies(/members) implies all_signed(/members)"
//! 4. Alice adds Bob - succeeds (she's the only member, so all_signed = [alice])
//! 5. Now to add Carol, BOTH alice and bob must sign (all_signed = [alice, bob])
//!
//! The rule never changed. The model evolved because /members grew.

use std::collections::{HashMap, HashSet};

/// Simulated commit with multi-signature support
#[derive(Debug, Clone)]
struct Commit {
    actions: Vec<Action>,
    signers: HashSet<String>,
}

#[derive(Debug, Clone)]
enum Action {
    Post { path: String, value: String },
    Rule { formula: String },
}

impl Commit {
    fn new() -> Self {
        Self {
            actions: vec![],
            signers: HashSet::new(),
        }
    }

    fn with_post(mut self, path: &str, value: &str) -> Self {
        self.actions.push(Action::Post {
            path: path.to_string(),
            value: value.to_string(),
        });
        self
    }

    fn with_rule(mut self, formula: &str) -> Self {
        self.actions.push(Action::Rule {
            formula: formula.to_string(),
        });
        self
    }

    fn signed_by(mut self, signer: &str) -> Self {
        self.signers.insert(signer.to_string());
        self
    }

    fn modifies_path(&self, prefix: &str) -> bool {
        self.actions.iter().any(|a| match a {
            Action::Post { path, .. } => path.starts_with(prefix),
            _ => false,
        })
    }
}

/// Contract state tracking members and rules
#[derive(Debug, Default)]
struct MemberContract {
    /// Member IDs stored at /members/*.id
    members: HashMap<String, String>,
    /// Active rules
    rules: Vec<String>,
}

impl MemberContract {
    fn new() -> Self {
        Self::default()
    }

    /// Get all current member IDs
    fn all_member_ids(&self) -> HashSet<String> {
        self.members.values().cloned().collect()
    }

    /// Check if commit satisfies the "all_signed(/members)" predicate
    fn all_members_signed(&self, commit: &Commit) -> bool {
        let required = self.all_member_ids();
        if required.is_empty() {
            return true; // No members yet, anything goes
        }
        required.iter().all(|m| commit.signers.contains(m))
    }

    /// Validate and apply a commit
    fn apply(&mut self, commit: &Commit) -> Result<(), String> {
        // Check rules
        for rule in &self.rules {
            if rule.contains("modifies(/members) implies all_signed(/members)") {
                if commit.modifies_path("/members") && !self.all_members_signed(commit) {
                    let required: Vec<_> = self.all_member_ids().into_iter().collect();
                    let have: Vec<_> = commit.signers.iter().cloned().collect();
                    return Err(format!(
                        "Rule violation: modifies(/members) requires all_signed(/members). Required: {:?}, Have: {:?}",
                        required, have
                    ));
                }
            }
        }

        // Apply actions
        for action in &commit.actions {
            match action {
                Action::Post { path, value } => {
                    if path.starts_with("/members/") && path.ends_with(".id") {
                        let name = path
                            .strip_prefix("/members/")
                            .unwrap()
                            .strip_suffix(".id")
                            .unwrap();
                        self.members.insert(name.to_string(), value.clone());
                    }
                }
                Action::Rule { formula } => {
                    self.rules.push(formula.clone());
                }
            }
        }

        Ok(())
    }
}

#[test]
fn test_membership_evolution_with_rule_enforcement() {
    let mut contract = MemberContract::new();

    // Simulated public keys
    let alice_id = "ed25519:alice_key";
    let bob_id = "ed25519:bob_key";
    let carol_id = "ed25519:carol_key";

    // === Step 1: Alice adds herself (no rules yet, succeeds) ===
    let commit1 = Commit::new()
        .with_post("/members/alice.id", alice_id)
        .signed_by(alice_id);

    let result = contract.apply(&commit1);
    assert!(result.is_ok(), "Alice should add herself (no rules yet)");
    assert_eq!(contract.members.len(), 1);

    // === Step 2: Alice adds the membership rule ===
    let commit2 = Commit::new()
        .with_rule("always (modifies(/members) implies all_signed(/members))")
        .signed_by(alice_id);

    let result = contract.apply(&commit2);
    assert!(result.is_ok(), "Alice should add the rule");
    assert_eq!(contract.rules.len(), 1);

    // === Step 3: Alice adds Bob ===
    // At this point: all_signed(/members) = [alice]
    // Alice is the only member, so her signature is sufficient
    let commit3 = Commit::new()
        .with_post("/members/bob.id", bob_id)
        .signed_by(alice_id);

    let result = contract.apply(&commit3);
    assert!(result.is_ok(), "Alice alone can add Bob (she's the only member)");
    assert_eq!(contract.members.len(), 2);

    // === Step 4: Try to add Carol with only Alice's signature ===
    // Now: all_signed(/members) = [alice, bob]
    // The MODEL has evolved! Same rule, different requirements.
    let commit4_alice_only = Commit::new()
        .with_post("/members/carol.id", carol_id)
        .signed_by(alice_id);

    let result = contract.apply(&commit4_alice_only);
    assert!(
        result.is_err(),
        "Alice alone cannot add Carol - Bob must also sign"
    );
    assert!(
        result.unwrap_err().contains("all_signed(/members)"),
        "Error should mention the rule"
    );

    // Carol was NOT added
    assert_eq!(contract.members.len(), 2);

    // === Step 5: Add Carol with BOTH signatures ===
    let commit4_both = Commit::new()
        .with_post("/members/carol.id", carol_id)
        .signed_by(alice_id)
        .signed_by(bob_id);

    let result = contract.apply(&commit4_both);
    assert!(
        result.is_ok(),
        "Both Alice and Bob signing should allow adding Carol"
    );
    assert_eq!(contract.members.len(), 3);

    // === Verify final state ===
    assert!(contract.members.contains_key("alice"));
    assert!(contract.members.contains_key("bob"));
    assert!(contract.members.contains_key("carol"));

    // === Key Insight ===
    // The rule "modifies(/members) implies all_signed(/members)" never changed.
    // But its INTERPRETATION evolved as /members grew:
    //
    // Time     | Members          | all_signed requirement
    // ---------|------------------|------------------------
    // Step 1   | (none)           | (no rule yet)
    // Step 3   | [alice]          | [alice]
    // Step 4-5 | [alice, bob]     | [alice, bob]
    // After 5  | [alice,bob,carol]| [alice, bob, carol]
    //
    // This is model evolution through state change, not rule change.
}

#[test]
fn test_empty_membership_allows_anyone() {
    let mut contract = MemberContract::new();

    // Add the rule BEFORE any members exist
    let commit1 = Commit::new()
        .with_rule("always (modifies(/members) implies all_signed(/members))")
        .signed_by("anyone");

    let result = contract.apply(&commit1);
    assert!(result.is_ok());

    // With no members, "all_signed(/members)" is vacuously true
    // Anyone can add the first member
    let commit2 = Commit::new()
        .with_post("/members/first.id", "first_key")
        .signed_by("random_person");

    let result = contract.apply(&commit2);
    assert!(
        result.is_ok(),
        "With no members, anyone can add the first member"
    );
}
