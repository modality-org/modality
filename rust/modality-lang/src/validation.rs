//! Contract Validation
//!
//! Ensures contracts only contain predicates, not raw propositions.
//! Predicates are verifiable; propositions are just claims.

use crate::ast::{Model, Property, PropertySource};

/// Validation error types
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Raw proposition found where predicate required
    RawProposition {
        property_name: String,
        transition_from: String,
        transition_to: String,
        hint: String,
    },
    /// Other validation errors
    Other(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::RawProposition { property_name, transition_from, transition_to, hint } => {
                write!(
                    f,
                    "Raw proposition '+{}' in transition {} --> {} is not allowed in contracts. {}",
                    property_name, transition_from, transition_to, hint
                )
            }
            ValidationError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// Validation result
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// Validate that a model only contains predicates, not raw propositions
/// 
/// In contracts, every action must be verifiable. Raw propositions like `+DELIVER`
/// cannot be verified - only predicates like `+signed_by(bob)` can be checked
/// against commit data.
pub fn validate_no_raw_propositions(model: &Model) -> ValidationResult {
    let mut errors = Vec::new();
    
    // Check direct transitions
    for transition in &model.transitions {
        for prop in &transition.properties {
            if is_raw_proposition(prop) {
                errors.push(ValidationError::RawProposition {
                    property_name: prop.name.clone(),
                    transition_from: transition.from.clone(),
                    transition_to: transition.to.clone(),
                    hint: format!(
                        "Use a predicate like '+signed_by(/users/party.id)' or '+{}(...)'",
                        prop.name.to_lowercase()
                    ),
                });
            }
        }
    }
    
    // Check transitions in parts
    for part in &model.parts {
        for transition in &part.transitions {
            for prop in &transition.properties {
                if is_raw_proposition(prop) {
                    errors.push(ValidationError::RawProposition {
                        property_name: prop.name.clone(),
                        transition_from: transition.from.clone(),
                        transition_to: transition.to.clone(),
                        hint: format!(
                            "Use a predicate like '+signed_by(/users/party.id)' or '+{}(...)'",
                            prop.name.to_lowercase()
                        ),
                    });
                }
            }
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check if a property is a raw proposition (no predicate source)
fn is_raw_proposition(prop: &Property) -> bool {
    match &prop.source {
        None => true, // No source = raw proposition
        Some(PropertySource::Static) => true, // Static = raw proposition
        Some(PropertySource::Predicate { .. }) => false, // Predicate = OK
    }
}

/// List of known predicates for helpful error messages
pub const KNOWN_PREDICATES: &[&str] = &[
    "signed_by",
    "threshold",
    "before",
    "after", 
    "hash_matches",
    "preimage_of",
    "amount_equals",
    "amount_gte",
    "oracle_attests",
    "state_equals",
    "state_exists",
];

/// Suggest predicates for common action names
pub fn suggest_predicate(action_name: &str) -> String {
    let lower = action_name.to_lowercase();
    
    if lower.contains("sign") || lower.contains("approve") || lower.contains("commit") {
        return "signed_by(/users/party.id)".to_string();
    }
    
    if lower.contains("pay") || lower.contains("deposit") || lower.contains("transfer") {
        return "signed_by(/users/payer.id) and consider amount_equals(value)".to_string();
    }
    
    if lower.contains("deliver") || lower.contains("complete") || lower.contains("done") {
        return "signed_by(/users/provider.id)".to_string();
    }
    
    if lower.contains("deadline") || lower.contains("expire") {
        return "before(/state/deadline.datetime) or after(/state/deadline.datetime)".to_string();
    }
    
    if lower.contains("reveal") || lower.contains("claim") {
        return "hash_matches(/state/commitment.hash) or preimage_of(/state/hash.hash)".to_string();
    }
    
    // Default suggestion
    "signed_by(/users/party.id)".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Model, Part, Transition, Property, PropertySign};
    
    #[test]
    fn test_raw_proposition_rejected() {
        let mut model = Model::new("Test".to_string());
        let mut part = Part::new("flow".to_string());
        
        let mut t = Transition::new("a".to_string(), "b".to_string());
        t.add_property(Property::new(PropertySign::Plus, "DELIVER".to_string()));
        part.add_transition(t);
        model.add_part(part);
        
        let result = validate_no_raw_propositions(&model);
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::RawProposition { .. }));
    }
    
    #[test]
    fn test_predicate_accepted() {
        let mut model = Model::new("Test".to_string());
        let mut part = Part::new("flow".to_string());
        
        let mut t = Transition::new("a".to_string(), "b".to_string());
        t.add_property(Property::new_predicate_from_call(
            "signed_by".to_string(),
            "/users/alice.id".to_string(),
        ));
        part.add_transition(t);
        model.add_part(part);
        
        let result = validate_no_raw_propositions(&model);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_mixed_properties() {
        let mut model = Model::new("Test".to_string());
        let mut part = Part::new("flow".to_string());
        
        let mut t = Transition::new("a".to_string(), "b".to_string());
        // One predicate, one raw
        t.add_property(Property::new_predicate_from_call(
            "signed_by".to_string(),
            "/users/alice.id".to_string(),
        ));
        t.add_property(Property::new(PropertySign::Plus, "DELIVER".to_string()));
        part.add_transition(t);
        model.add_part(part);
        
        let result = validate_no_raw_propositions(&model);
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
    }
}
