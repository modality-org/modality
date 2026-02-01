//! One-Step Rule Validation
//!
//! A one-step rule is a formula that applies only to the commit it's attached to.
//! Unlike persistent rules (added via RULE method), one-step rules are not accumulated
//! into the contract's ongoing ruleset.
//!
//! Common use case: threshold signatures
//! ```json
//! {
//!   "head": {
//!     "one_step_rule": {
//!       "formula": "threshold(2, [/users/alice.id, /users/bob.id, /users/carol.id])"
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

/// One-step rule formula types
#[derive(Debug, Clone)]
pub enum OneStepFormula {
    /// threshold(n, [signer1, signer2, ...])
    /// Requires at least n valid signatures from the given signers
    Threshold {
        required: usize,
        signers: Vec<String>,
    },
    /// signed_by(signer) - single signature required
    SignedBy(String),
    /// Conjunction: formula1 & formula2
    And(Box<OneStepFormula>, Box<OneStepFormula>),
    /// Disjunction: formula1 | formula2  
    Or(Box<OneStepFormula>, Box<OneStepFormula>),
}

/// Parse a one-step rule formula string
pub fn parse_formula(formula: &str) -> Result<OneStepFormula> {
    let formula = formula.trim();
    
    // Handle conjunction (lowest precedence)
    if let Some(pos) = find_top_level_operator(formula, '&') {
        let left = parse_formula(&formula[..pos])?;
        let right = parse_formula(&formula[pos + 1..])?;
        return Ok(OneStepFormula::And(Box::new(left), Box::new(right)));
    }
    
    // Handle disjunction
    if let Some(pos) = find_top_level_operator(formula, '|') {
        let left = parse_formula(&formula[..pos])?;
        let right = parse_formula(&formula[pos + 1..])?;
        return Ok(OneStepFormula::Or(Box::new(left), Box::new(right)));
    }
    
    // Handle parentheses
    if formula.starts_with('(') && formula.ends_with(')') {
        return parse_formula(&formula[1..formula.len()-1]);
    }
    
    // Handle threshold(n, [...])
    if formula.starts_with("threshold(") && formula.ends_with(')') {
        return parse_threshold(formula);
    }
    
    // Handle signed_by(path)
    if formula.starts_with("signed_by(") && formula.ends_with(')') {
        let inner = &formula[10..formula.len()-1];
        return Ok(OneStepFormula::SignedBy(inner.trim().to_string()));
    }
    
    bail!("Cannot parse one-step formula: {}", formula);
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

/// Parse threshold(n, [signer1, signer2, ...])
fn parse_threshold(formula: &str) -> Result<OneStepFormula> {
    // Extract the content inside threshold(...)
    let inner = &formula[10..formula.len()-1];
    
    // Find the comma separating n from the array
    let comma_pos = inner.find(',')
        .ok_or_else(|| anyhow::anyhow!("threshold requires format: threshold(n, [signers])"))?;
    
    let n_str = inner[..comma_pos].trim();
    let required: usize = n_str.parse()
        .map_err(|_| anyhow::anyhow!("threshold count must be a number, got: {}", n_str))?;
    
    // Parse the array of signers
    let array_str = inner[comma_pos + 1..].trim();
    if !array_str.starts_with('[') || !array_str.ends_with(']') {
        bail!("threshold second argument must be an array: [signer1, signer2, ...]");
    }
    
    let signers_str = &array_str[1..array_str.len()-1];
    let signers: Vec<String> = signers_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    if required > signers.len() {
        bail!("threshold {} exceeds number of signers {}", required, signers.len());
    }
    
    Ok(OneStepFormula::Threshold { required, signers })
}

/// Evaluate a one-step formula against a set of signatures
/// 
/// Note: This checks the structural requirement (sufficient signers present).
/// The cryptographic signature verification should happen elsewhere using
/// the actual message and public keys.
pub fn evaluate_formula(
    formula: &OneStepFormula, 
    present_signers: &[String],
) -> bool {
    match formula {
        OneStepFormula::Threshold { required, signers } => {
            // Count how many of the required signers are present
            let count = signers.iter()
                .filter(|s| present_signers.contains(s))
                .count();
            count >= *required
        }
        OneStepFormula::SignedBy(signer) => {
            present_signers.contains(signer)
        }
        OneStepFormula::And(left, right) => {
            evaluate_formula(left, present_signers) && evaluate_formula(right, present_signers)
        }
        OneStepFormula::Or(left, right) => {
            evaluate_formula(left, present_signers) || evaluate_formula(right, present_signers)
        }
    }
}

/// Validate a commit's one-step rule against its signatures
pub fn validate_one_step_rule(
    formula_str: &str,
    signatures: &[CommitSignature],
) -> Result<()> {
    let formula = parse_formula(formula_str)?;
    
    // Extract signer paths from signatures
    let present_signers: Vec<String> = signatures.iter()
        .map(|s| s.signer.clone())
        .collect();
    
    if evaluate_formula(&formula, &present_signers) {
        Ok(())
    } else {
        bail!(
            "One-step rule not satisfied: {} (present signers: {:?})",
            formula_str,
            present_signers
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_threshold() {
        let formula = parse_formula("threshold(2, [/users/alice.id, /users/bob.id, /users/carol.id])").unwrap();
        
        match formula {
            OneStepFormula::Threshold { required, signers } => {
                assert_eq!(required, 2);
                assert_eq!(signers.len(), 3);
                assert_eq!(signers[0], "/users/alice.id");
            }
            _ => panic!("Expected Threshold formula"),
        }
    }
    
    #[test]
    fn test_parse_signed_by() {
        let formula = parse_formula("signed_by(/users/alice.id)").unwrap();
        
        match formula {
            OneStepFormula::SignedBy(signer) => {
                assert_eq!(signer, "/users/alice.id");
            }
            _ => panic!("Expected SignedBy formula"),
        }
    }
    
    #[test]
    fn test_parse_conjunction() {
        let formula = parse_formula("signed_by(/users/alice.id) & signed_by(/users/bob.id)").unwrap();
        
        match formula {
            OneStepFormula::And(_, _) => {}
            _ => panic!("Expected And formula"),
        }
    }
    
    #[test]
    fn test_evaluate_threshold_success() {
        let formula = OneStepFormula::Threshold {
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
    fn test_evaluate_threshold_failure() {
        let formula = OneStepFormula::Threshold {
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
    fn test_validate_one_step_rule_success() {
        let sigs = vec![
            CommitSignature { signer: "/users/alice.id".to_string(), sig: "sig1".to_string() },
            CommitSignature { signer: "/users/bob.id".to_string(), sig: "sig2".to_string() },
        ];
        
        let result = validate_one_step_rule(
            "threshold(2, [/users/alice.id, /users/bob.id, /users/carol.id])",
            &sigs,
        );
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validate_one_step_rule_failure() {
        let sigs = vec![
            CommitSignature { signer: "/users/alice.id".to_string(), sig: "sig1".to_string() },
        ];
        
        let result = validate_one_step_rule(
            "threshold(2, [/users/alice.id, /users/bob.id, /users/carol.id])",
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
}
