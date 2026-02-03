//! Rule For This Commit Validation
//!
//! A `rule_for_this_commit` is a formula that applies only to the commit it's attached to.
//! Unlike persistent rules (added via RULE method), these are not accumulated
//! into the contract's ongoing ruleset.
//!
//! Common use case: threshold signatures
//! ```json
//! {
//!   "head": {
//!     "rule_for_this_commit": {
//!       "formula": "signed_by_n(2, [/users/alice.id, /users/bob.id, /users/carol.id])"
//!     },
//!     "signatures": [
//!       { "signer": "/users/alice.id", "sig": "..." },
//!       { "signer": "/users/bob.id", "sig": "..." }
//!     ]
//!   }
//! }
//! ```

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Signature entry in a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSignature {
    /// Path to signer's identity (e.g., "/users/alice.id")
    pub signer: String,
    /// The signature (hex or base64)
    pub sig: String,
}

/// Parse signatures from the commit head
pub fn parse_signatures(signatures_value: &Value) -> Result<Vec<CommitSignature>> {
    match signatures_value {
        Value::Array(arr) => {
            let mut sigs = Vec::new();
            for item in arr {
                let sig: CommitSignature = serde_json::from_value(item.clone())?;
                sigs.push(sig);
            }
            Ok(sigs)
        }
        Value::Null => Ok(Vec::new()),
        _ => bail!("signatures must be an array"),
    }
}

/// Commit rule formula types
#[derive(Debug, Clone)]
pub enum CommitRuleFormula {
    /// signed_by_n(n, [signer1, signer2, ...])
    /// Requires at least n valid signatures from the given signers
    SignedByN {
        required: usize,
        signers: Vec<String>,
    },
    /// signed_by(signer) - single signature required
    SignedBy(String),
    /// all_signed(path) - all members at path must sign
    /// Reads array from contract state, requires ALL to be signers
    AllSigned(String),
    /// any_signed(path) - at least one member at path must sign
    /// Reads array from contract state, requires at least ONE to be a signer
    AnySigned(String),
    /// modifies(path_prefix) - commit modifies paths under this prefix
    /// Returns true if any POST/DELETE in commit body touches paths starting with prefix
    Modifies(String),
    /// Conjunction: formula1 & formula2
    And(Box<CommitRuleFormula>, Box<CommitRuleFormula>),
    /// Disjunction: formula1 | formula2  
    Or(Box<CommitRuleFormula>, Box<CommitRuleFormula>),
}

/// Parse a commit rule formula string
pub fn parse_formula(formula: &str) -> Result<CommitRuleFormula> {
    let formula = formula.trim();
    
    // Handle conjunction (lowest precedence)
    if let Some(pos) = find_top_level_operator(formula, '&') {
        let left = parse_formula(&formula[..pos])?;
        let right = parse_formula(&formula[pos + 1..])?;
        return Ok(CommitRuleFormula::And(Box::new(left), Box::new(right)));
    }
    
    // Handle disjunction
    if let Some(pos) = find_top_level_operator(formula, '|') {
        let left = parse_formula(&formula[..pos])?;
        let right = parse_formula(&formula[pos + 1..])?;
        return Ok(CommitRuleFormula::Or(Box::new(left), Box::new(right)));
    }
    
    // Handle parentheses
    if formula.starts_with('(') && formula.ends_with(')') {
        return parse_formula(&formula[1..formula.len()-1]);
    }
    
    // Handle signed_by_n(n, [...])
    if formula.starts_with("signed_by_n(") && formula.ends_with(')') {
        return parse_signed_by_n(formula);
    }
    
    // Handle signed_by(path)
    if formula.starts_with("signed_by(") && formula.ends_with(')') {
        let inner = &formula[10..formula.len()-1];
        return Ok(CommitRuleFormula::SignedBy(inner.trim().to_string()));
    }
    
    // Handle all_signed(path) - all members at path must sign
    if formula.starts_with("all_signed(") && formula.ends_with(')') {
        let inner = &formula[11..formula.len()-1];
        return Ok(CommitRuleFormula::AllSigned(inner.trim().to_string()));
    }
    
    // Handle any_signed(path) - at least one member at path must sign
    if formula.starts_with("any_signed(") && formula.ends_with(')') {
        let inner = &formula[11..formula.len()-1];
        return Ok(CommitRuleFormula::AnySigned(inner.trim().to_string()));
    }
    
    // Handle modifies(path) - commit touches paths under this prefix
    if formula.starts_with("modifies(") && formula.ends_with(')') {
        let inner = &formula[9..formula.len()-1];
        return Ok(CommitRuleFormula::Modifies(inner.trim().to_string()));
    }
    
    bail!("Cannot parse commit rule formula: {}", formula);
}

/// Find operator at top level (not inside parentheses or brackets)
fn find_top_level_operator(s: &str, op: char) -> Option<usize> {
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    
    for (i, c) in s.chars().enumerate() {
        match c {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            _ if c == op && paren_depth == 0 && bracket_depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Parse signed_by_n(n, [signer1, signer2, ...])
fn parse_signed_by_n(formula: &str) -> Result<CommitRuleFormula> {
    // Extract the content inside signed_by_n(...)
    let inner = &formula[12..formula.len()-1];
    
    // Find the comma separating n from the array
    let comma_pos = inner.find(',')
        .ok_or_else(|| anyhow::anyhow!("signed_by_n requires format: signed_by_n(n, [signers])"))?;
    
    let n_str = inner[..comma_pos].trim();
    let required: usize = n_str.parse()
        .map_err(|_| anyhow::anyhow!("signed_by_n count must be a number, got: {}", n_str))?;
    
    // Parse the array of signers
    let array_str = inner[comma_pos + 1..].trim();
    if !array_str.starts_with('[') || !array_str.ends_with(']') {
        bail!("signed_by_n second argument must be an array: [signer1, signer2, ...]");
    }
    
    let signers_str = &array_str[1..array_str.len()-1];
    let signers: Vec<String> = signers_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    if required > signers.len() {
        bail!("signed_by_n {} exceeds number of signers {}", required, signers.len());
    }
    
    Ok(CommitRuleFormula::SignedByN { required, signers })
}

/// Context for evaluating commit rule formulas
#[derive(Debug, Clone, Default)]
pub struct EvalContext<'a> {
    /// Signers present on the commit
    pub signers: &'a [String],
    /// Current contract state (for resolving paths)
    pub state: &'a Value,
    /// Paths modified by the commit body
    pub modified_paths: Vec<String>,
}

impl<'a> EvalContext<'a> {
    pub fn new(signers: &'a [String], state: &'a Value, commit_body: &Value) -> Self {
        let modified_paths = extract_modified_paths(commit_body);
        Self { signers, state, modified_paths }
    }
}

/// Extract paths modified by a commit body
fn extract_modified_paths(body: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(actions) = body.as_array() {
        for action in actions {
            let method = action.get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            
            if matches!(method.to_lowercase().as_str(), "post" | "delete" | "rule") {
                if let Some(path) = action.get("path").and_then(|p| p.as_str()) {
                    paths.push(path.trim_start_matches('/').to_string());
                }
            }
        }
    }
    paths
}

/// Evaluate a commit rule formula against a set of signatures
pub fn evaluate_formula(
    formula: &CommitRuleFormula, 
    present_signers: &[String],
) -> bool {
    let ctx = EvalContext {
        signers: present_signers,
        state: &Value::Null,
        modified_paths: Vec::new(),
    };
    evaluate_formula_full(formula, &ctx)
}

/// Evaluate a commit rule formula with access to contract state
pub fn evaluate_formula_with_state(
    formula: &CommitRuleFormula, 
    present_signers: &[String],
    contract_state: &Value,
) -> bool {
    let ctx = EvalContext {
        signers: present_signers,
        state: contract_state,
        modified_paths: Vec::new(),
    };
    evaluate_formula_full(formula, &ctx)
}

/// Evaluate a commit rule formula with full context
pub fn evaluate_formula_full(
    formula: &CommitRuleFormula, 
    ctx: &EvalContext,
) -> bool {
    match formula {
        CommitRuleFormula::SignedByN { required, signers } => {
            let count = signers.iter()
                .filter(|s| ctx.signers.contains(s))
                .count();
            count >= *required
        }
        CommitRuleFormula::SignedBy(signer) => {
            ctx.signers.contains(signer)
        }
        CommitRuleFormula::AllSigned(path) => {
            let members = resolve_path_as_strings(ctx.state, path);
            if members.is_empty() {
                true
            } else {
                members.iter().all(|m| ctx.signers.contains(m))
            }
        }
        CommitRuleFormula::AnySigned(path) => {
            let members = resolve_path_as_strings(ctx.state, path);
            if members.is_empty() {
                true
            } else {
                members.iter().any(|m| ctx.signers.contains(m))
            }
        }
        CommitRuleFormula::Modifies(prefix) => {
            let normalized = prefix.trim_start_matches('/');
            ctx.modified_paths.iter().any(|p| {
                p.starts_with(normalized) || p == normalized
            })
        }
        CommitRuleFormula::And(left, right) => {
            evaluate_formula_full(left, ctx) && evaluate_formula_full(right, ctx)
        }
        CommitRuleFormula::Or(left, right) => {
            evaluate_formula_full(left, ctx) || evaluate_formula_full(right, ctx)
        }
    }
}

/// Resolve a path in contract state to a list of identity strings
/// 
/// Supports two patterns:
/// 1. Array value: /members.json → ["alice_key", "bob_key"]
/// 2. Directory of .id files: /members → scans for /members/*.id values
fn resolve_path_as_strings(state: &Value, path: &str) -> Vec<String> {
    let normalized = path.trim_start_matches('/');
    
    // First, try direct array lookup
    if let Some(arr) = state.get(normalized).and_then(|v| v.as_array()) {
        return arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    
    // Otherwise, scan for {path}/*.id entries in state
    let prefix = if normalized.ends_with('/') {
        normalized.to_string()
    } else {
        format!("{}/", normalized)
    };
    
    let mut ids = Vec::new();
    
    if let Some(obj) = state.as_object() {
        for (key, value) in obj {
            // Match keys like "members/alice.id" when path is "members"
            if key.starts_with(&prefix) && key.ends_with(".id") {
                if let Some(id_value) = value.as_str() {
                    ids.push(id_value.to_string());
                }
            }
        }
    }
    
    ids
}

/// Validate a commit's rule_for_this_commit against its signatures
pub fn validate_rule_for_this_commit(
    formula_str: &str,
    signatures: &[CommitSignature],
) -> Result<()> {
    validate_rule_for_this_commit_with_state(formula_str, signatures, &Value::Null)
}

/// Validate a commit's rule_for_this_commit against its signatures and contract state
pub fn validate_rule_for_this_commit_with_state(
    formula_str: &str,
    signatures: &[CommitSignature],
    contract_state: &Value,
) -> Result<()> {
    let formula = parse_formula(formula_str)?;
    
    // Extract signer paths from signatures
    let present_signers: Vec<String> = signatures.iter()
        .map(|s| s.signer.clone())
        .collect();
    
    if evaluate_formula_with_state(&formula, &present_signers, contract_state) {
        Ok(())
    } else {
        bail!(
            "rule_for_this_commit not satisfied: {} (present signers: {:?})",
            formula_str,
            present_signers
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_signed_by_n() {
        let formula = parse_formula("signed_by_n(2, [/users/alice.id, /users/bob.id, /users/carol.id])").unwrap();
        
        match formula {
            CommitRuleFormula::SignedByN { required, signers } => {
                assert_eq!(required, 2);
                assert_eq!(signers.len(), 3);
                assert_eq!(signers[0], "/users/alice.id");
            }
            _ => panic!("Expected SignedByN formula"),
        }
    }
    
    #[test]
    fn test_parse_signed_by() {
        let formula = parse_formula("signed_by(/users/alice.id)").unwrap();
        
        match formula {
            CommitRuleFormula::SignedBy(signer) => {
                assert_eq!(signer, "/users/alice.id");
            }
            _ => panic!("Expected SignedBy formula"),
        }
    }
    
    #[test]
    fn test_parse_conjunction() {
        let formula = parse_formula("signed_by(/users/alice.id) & signed_by(/users/bob.id)").unwrap();
        
        match formula {
            CommitRuleFormula::And(_, _) => {}
            _ => panic!("Expected And formula"),
        }
    }
    
    #[test]
    fn test_evaluate_signed_by_n_success() {
        let formula = CommitRuleFormula::SignedByN {
            required: 2,
            signers: vec![
                "/users/alice.id".to_string(),
                "/users/bob.id".to_string(),
                "/users/carol.id".to_string(),
            ],
        };
        
        let present = vec![
            "/users/alice.id".to_string(),
            "/users/bob.id".to_string(),
        ];
        
        assert!(evaluate_formula(&formula, &present));
    }
    
    #[test]
    fn test_evaluate_signed_by_n_failure() {
        let formula = CommitRuleFormula::SignedByN {
            required: 2,
            signers: vec![
                "/users/alice.id".to_string(),
                "/users/bob.id".to_string(),
                "/users/carol.id".to_string(),
            ],
        };
        
        // Only one signer present
        let present = vec!["/users/alice.id".to_string()];
        
        assert!(!evaluate_formula(&formula, &present));
    }
    
    #[test]
    fn test_validate_rule_for_this_commit_success() {
        let sigs = vec![
            CommitSignature { signer: "/users/alice.id".to_string(), sig: "sig1".to_string() },
            CommitSignature { signer: "/users/bob.id".to_string(), sig: "sig2".to_string() },
        ];
        
        let result = validate_rule_for_this_commit(
            "signed_by_n(2, [/users/alice.id, /users/bob.id, /users/carol.id])",
            &sigs,
        );
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validate_rule_for_this_commit_failure() {
        let sigs = vec![
            CommitSignature { signer: "/users/alice.id".to_string(), sig: "sig1".to_string() },
        ];
        
        let result = validate_rule_for_this_commit(
            "signed_by_n(2, [/users/alice.id, /users/bob.id, /users/carol.id])",
            &sigs,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_conjunction_both_required() {
        let formula = parse_formula("signed_by(/users/alice.id) & signed_by(/users/bob.id)").unwrap();
        
        // Both present - should pass
        let both = vec![
            "/users/alice.id".to_string(),
            "/users/bob.id".to_string(),
        ];
        assert!(evaluate_formula(&formula, &both));
        
        // Only one - should fail
        let one = vec!["/users/alice.id".to_string()];
        assert!(!evaluate_formula(&formula, &one));
    }
    
    #[test]
    fn test_disjunction_either_works() {
        let formula = parse_formula("signed_by(/users/alice.id) | signed_by(/users/bob.id)").unwrap();
        
        // Alice alone - should pass
        let alice = vec!["/users/alice.id".to_string()];
        assert!(evaluate_formula(&formula, &alice));
        
        // Bob alone - should pass
        let bob = vec!["/users/bob.id".to_string()];
        assert!(evaluate_formula(&formula, &bob));
        
        // Neither - should fail
        let neither: Vec<String> = vec![];
        assert!(!evaluate_formula(&formula, &neither));
    }

    // ===== Dynamic member tests (all_signed / any_signed) =====

    #[test]
    fn test_parse_all_signed() {
        let formula = parse_formula("all_signed(/members.json)").unwrap();
        match formula {
            CommitRuleFormula::AllSigned(path) => {
                assert_eq!(path, "/members.json");
            }
            _ => panic!("Expected AllSigned formula"),
        }
    }

    #[test]
    fn test_parse_any_signed() {
        let formula = parse_formula("any_signed(/members.json)").unwrap();
        match formula {
            CommitRuleFormula::AnySigned(path) => {
                assert_eq!(path, "/members.json");
            }
            _ => panic!("Expected AnySigned formula"),
        }
    }

    #[test]
    fn test_all_signed_with_directory() {
        let formula = CommitRuleFormula::AllSigned("/members".to_string());
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key",
            "members/carol.id": "carol_key"
        });
        let signers = vec![
            "alice_key".to_string(),
            "bob_key".to_string(),
            "carol_key".to_string(),
        ];
        
        assert!(evaluate_formula_with_state(&formula, &signers, &state));
    }

    #[test]
    fn test_all_signed_failure_missing_signer() {
        let formula = CommitRuleFormula::AllSigned("/members".to_string());
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key",
            "members/carol.id": "carol_key"
        });
        // Missing carol_key
        let signers = vec!["alice_key".to_string(), "bob_key".to_string()];
        
        assert!(!evaluate_formula_with_state(&formula, &signers, &state));
    }

    #[test]
    fn test_all_signed_empty_members_passes() {
        let formula = CommitRuleFormula::AllSigned("/members".to_string());
        let state = serde_json::json!({});  // No members/*.id files
        let signers = vec!["anyone".to_string()];
        
        // Empty member list = trivially satisfied
        assert!(evaluate_formula_with_state(&formula, &signers, &state));
    }

    #[test]
    fn test_any_signed_with_directory() {
        let formula = CommitRuleFormula::AnySigned("/members".to_string());
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key",
            "members/carol.id": "carol_key"
        });
        // Only bob signed
        let signers = vec!["bob_key".to_string()];
        
        assert!(evaluate_formula_with_state(&formula, &signers, &state));
    }

    #[test]
    fn test_any_signed_failure_no_members_signed() {
        let formula = CommitRuleFormula::AnySigned("/members".to_string());
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key",
            "members/carol.id": "carol_key"
        });
        // Stranger signed, not a member
        let signers = vec!["stranger_key".to_string()];
        
        assert!(!evaluate_formula_with_state(&formula, &signers, &state));
    }

    #[test]
    fn test_validate_with_state_all_signed() {
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key"
        });
        let sigs = vec![
            CommitSignature { signer: "alice_key".to_string(), sig: "sig1".to_string() },
            CommitSignature { signer: "bob_key".to_string(), sig: "sig2".to_string() },
        ];
        
        let result = validate_rule_for_this_commit_with_state(
            "all_signed(/members)",
            &sigs,
            &state,
        );
        
        assert!(result.is_ok(), "All members signed, should pass");
    }

    #[test]
    fn test_validate_with_state_all_signed_fails() {
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key",
            "members/carol.id": "carol_key"
        });
        let sigs = vec![
            CommitSignature { signer: "alice_key".to_string(), sig: "sig1".to_string() },
            CommitSignature { signer: "bob_key".to_string(), sig: "sig2".to_string() },
            // Missing carol_key
        ];
        
        let result = validate_rule_for_this_commit_with_state(
            "all_signed(/members)",
            &sigs,
            &state,
        );
        
        assert!(result.is_err(), "Missing carol, should fail");
    }

    #[test]
    fn test_all_signed_with_array_still_works() {
        // Backward compat: array format still works
        let formula = CommitRuleFormula::AllSigned("/signers.json".to_string());
        let state = serde_json::json!({
            "signers.json": ["alice_key", "bob_key"]
        });
        let signers = vec!["alice_key".to_string(), "bob_key".to_string()];
        
        assert!(evaluate_formula_with_state(&formula, &signers, &state));
    }

    // ===== modifies() predicate tests =====

    #[test]
    fn test_parse_modifies() {
        let formula = parse_formula("modifies(/members)").unwrap();
        match formula {
            CommitRuleFormula::Modifies(path) => {
                assert_eq!(path, "/members");
            }
            _ => panic!("Expected Modifies formula"),
        }
    }

    #[test]
    fn test_modifies_matches_path() {
        let formula = CommitRuleFormula::Modifies("/members".to_string());
        let body = serde_json::json!([
            {"method": "post", "path": "/members/dave.id", "value": "dave_key"}
        ]);
        let ctx = EvalContext::new(&[], &Value::Null, &body);
        
        assert!(evaluate_formula_full(&formula, &ctx));
    }

    #[test]
    fn test_modifies_no_match() {
        let formula = CommitRuleFormula::Modifies("/members".to_string());
        let body = serde_json::json!([
            {"method": "post", "path": "/data/notes.txt", "value": "hello"}
        ]);
        let ctx = EvalContext::new(&[], &Value::Null, &body);
        
        assert!(!evaluate_formula_full(&formula, &ctx));
    }

    #[test]
    fn test_modifies_combined_with_signature() {
        // modifies(/members) & all_signed(/members) 
        let formula = parse_formula("modifies(/members) & all_signed(/members)").unwrap();
        
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key"
        });
        let body = serde_json::json!([
            {"method": "post", "path": "/members/carol.id", "value": "carol_key"}
        ]);
        let signers = vec!["alice_key".to_string(), "bob_key".to_string()];
        let ctx = EvalContext::new(&signers, &state, &body);
        
        // Modifies /members: true, all_signed: true (alice + bob)
        assert!(evaluate_formula_full(&formula, &ctx));
    }

    #[test]
    fn test_modifies_data_any_signed() {
        // modifies(/data) & any_signed(/members)
        let formula = parse_formula("modifies(/data) & any_signed(/members)").unwrap();
        
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key"
        });
        let body = serde_json::json!([
            {"method": "post", "path": "/data/note.txt", "value": "hello"}
        ]);
        let signers = vec!["bob_key".to_string()];
        let ctx = EvalContext::new(&signers, &state, &body);
        
        // Modifies /data: true, any_signed: true (bob is member)
        assert!(evaluate_formula_full(&formula, &ctx));
    }

    #[test]
    fn test_transition_guard_pattern() {
        // This is the pattern for: "modifying /members requires all signatures"
        // If modifies(/members) then must have all_signed(/members)
        // Expressed as: !modifies(/members) | all_signed(/members)
        // Or: modifies(/members) implies all_signed(/members)
        
        let state = serde_json::json!({
            "members/alice.id": "alice_key",
            "members/bob.id": "bob_key"
        });
        
        // Case 1: Modifying members WITH all signatures - should pass
        let body = serde_json::json!([
            {"method": "post", "path": "/members/carol.id", "value": "carol_key"}
        ]);
        let signers = vec!["alice_key".to_string(), "bob_key".to_string()];
        let ctx = EvalContext::new(&signers, &state, &body);
        
        let modifies_members = CommitRuleFormula::Modifies("/members".to_string());
        let all_signed = CommitRuleFormula::AllSigned("/members".to_string());
        
        // Both should be true
        assert!(evaluate_formula_full(&modifies_members, &ctx));
        assert!(evaluate_formula_full(&all_signed, &ctx));
        
        // Case 2: Modifying members WITHOUT all signatures - all_signed fails
        let signers_partial = vec!["alice_key".to_string()]; // missing bob
        let ctx2 = EvalContext::new(&signers_partial, &state, &body);
        
        assert!(evaluate_formula_full(&modifies_members, &ctx2));
        assert!(!evaluate_formula_full(&all_signed, &ctx2));
        
        // Case 3: NOT modifying members - don't need all signatures
        let body_data = serde_json::json!([
            {"method": "post", "path": "/data/note.txt", "value": "hello"}
        ]);
        let ctx3 = EvalContext::new(&signers_partial, &state, &body_data);
        
        assert!(!evaluate_formula_full(&modifies_members, &ctx3));
        // all_signed still fails but doesn't matter since we're not modifying members
    }
}
