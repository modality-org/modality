//! LLM-assisted Rule Generation (Step 1 of two-step pipeline)
//!
//! NL → Formulas using LLM

/// System prompt for LLM to generate formulas from natural language
pub const SYSTEM_PROMPT: &str = r#"You are a formal verification expert. Convert natural language contract requirements into temporal modal logic formulas using Modality syntax.

## Syntax Reference

### Modal Operators
- `[+ACTION] φ` — all +ACTION transitions lead to φ
- `<+ACTION> φ` — some +ACTION transition leads to φ  
- `[<+ACTION>] φ` — committed to ACTION (can do, cannot refuse)

### Temporal Operators
- `always(φ)` — φ holds forever on all paths
- `eventually(φ)` — φ holds at some future state

### Predicates
- `+signed_by(/users/name.id)` — requires signature from name

## Common Patterns

| Requirement | Formula |
|-------------|---------|
| "X after Y" | `always([+X] implies eventually(<+Y> true))` |
| "Only A can X" | `always([+X] implies <+signed_by(/users/a.id)> true)` |
| "X requires Y and Z" | `always([+X] implies (eventually(<+Y> true) & eventually(<+Z> true)))` |
| "Never X after Y" | `always([+Y] implies always([-X] true))` |

## Output Format

Output ONLY the formulas, one per line, prefixed with F1:, F2:, etc.
No explanations, no markdown, just formulas.

Example output:
F1: always([+RELEASE] implies eventually(<+DELIVER> true))
F2: always([+RELEASE] implies <+signed_by(/users/alice.id)> true)
"#;

/// Generate LLM prompt for NL → Formula conversion
pub fn generate_prompt(nl_description: &str) -> String {
    format!(
        "{}\n\n## Contract Description\n\n{}\n\n## Generate Formulas\n",
        SYSTEM_PROMPT,
        nl_description
    )
}

/// Parse LLM response to extract formulas
pub fn parse_llm_response(response: &str) -> Vec<String> {
    let mut formulas = Vec::new();
    
    for line in response.lines() {
        let line = line.trim();
        
        // Skip empty lines
        if line.is_empty() {
            continue;
        }
        
        // Look for F1:, F2:, etc. pattern
        if let Some(colon_pos) = line.find(':') {
            let prefix = &line[..colon_pos];
            if prefix.starts_with('F') && prefix[1..].chars().all(|c| c.is_ascii_digit()) {
                let formula = line[colon_pos + 1..].trim();
                if !formula.is_empty() {
                    formulas.push(formula.to_string());
                }
                continue;
            }
        }
        
        // Also accept lines starting with "always" or "[" directly
        if line.starts_with("always") || line.starts_with('[') || line.starts_with("eventually") {
            formulas.push(line.to_string());
        }
    }
    
    formulas
}

/// Extract parties from NL description (simple heuristic)
pub fn extract_parties(description: &str) -> Vec<String> {
    let mut parties = Vec::new();
    let description_lower = description.to_lowercase();
    
    // Common party names
    let common_names = [
        ("alice", "Alice"),
        ("bob", "Bob"),
        ("carol", "Carol"),
        ("buyer", "Buyer"),
        ("seller", "Seller"),
        ("client", "Client"),
        ("contractor", "Contractor"),
        ("provider", "Provider"),
        ("consumer", "Consumer"),
        ("principal", "Principal"),
        ("agent", "Agent"),
        ("sender", "Sender"),
        ("receiver", "Receiver"),
    ];
    
    for (lower, proper) in common_names {
        if description_lower.contains(lower) {
            parties.push(proper.to_string());
        }
    }
    
    // Default if none found
    if parties.is_empty() {
        parties.push("PartyA".to_string());
        parties.push("PartyB".to_string());
    }
    
    parties
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_llm_response() {
        let response = r#"
F1: always([+RELEASE] implies eventually(<+DELIVER> true))
F2: always([+RELEASE] implies <+signed_by(/users/alice.id)> true)
F3: always([+DELIVER] implies <+signed_by(/users/bob.id)> true)
"#;
        
        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert!(formulas[0].contains("RELEASE"));
        assert!(formulas[1].contains("signed_by"));
    }
    
    #[test]
    fn test_parse_llm_response_no_prefix() {
        let response = r#"
always([+PAY] implies eventually(<+WORK> true))
[+EXECUTE] implies <+signed_by(/users/admin.id)> true
"#;
        
        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
    }
    
    #[test]
    fn test_extract_parties() {
        let parties = extract_parties("Alice wants to buy from Bob");
        assert!(parties.contains(&"Alice".to_string()));
        assert!(parties.contains(&"Bob".to_string()));
    }
}
