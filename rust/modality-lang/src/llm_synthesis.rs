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
| "X is allowed" | `<+X> true` |
| "Can always do X" | `always([<+X>] true)` |
| "X after Y" | `always([+X] implies eventually(<+Y> true))` |
| "Only A can X" | `always([+X] implies <+signed_by(/users/a.id)> true)` |
| "X requires A and B signatures" | `always([+X] implies <+signed_by(/users/a.id) +signed_by(/users/b.id)> true)` |
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
        SYSTEM_PROMPT, nl_description
    )
}

/// Parse LLM response to extract formulas
pub fn parse_llm_response(response: &str) -> Vec<String> {
    let mut formulas = Vec::new();

    'lines: for line in response.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        let line = strip_list_marker(line);

        // Look for F1:, F2., Formula 3), etc. labels.
        for separator in [':', '.', ')'] {
            if let Some(separator_pos) = line.find(separator) {
                let prefix = &line[..separator_pos];
                if is_formula_prefix(prefix) {
                    let formula = strip_formula_wrapping(line[separator_pos + 1..].trim());
                    if !formula.is_empty() {
                        formulas.push(formula.to_string());
                    }
                    continue 'lines;
                }
            }
        }

        // Also accept raw formula lines directly when no F1: prefix is present.
        let line = strip_formula_wrapping(line);
        if is_raw_formula_line(line) {
            formulas.push(line.to_string());
        }
    }

    formulas
}

fn is_formula_prefix(prefix: &str) -> bool {
    if let Some(label) = prefix.strip_prefix(['F', 'f']) {
        if !label.is_empty() && label.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    let lower_prefix = prefix.to_ascii_lowercase();
    let Some(label) = lower_prefix.strip_prefix("formula") else {
        return false;
    };
    let label = label.trim_start();

    !label.is_empty() && label.chars().all(|c| c.is_ascii_digit())
}

fn strip_list_marker(line: &str) -> &str {
    let Some((marker, rest)) = line.split_once(char::is_whitespace) else {
        return line;
    };

    if marker == "-" || marker == "*" || marker.ends_with('.') || marker.ends_with(')') {
        let marker_body = marker.trim_end_matches(['.', ')']);
        if marker == "-" || marker == "*" || marker_body.chars().all(|c| c.is_ascii_digit()) {
            return rest.trim_start();
        }
    }

    line
}

fn strip_formula_wrapping(line: &str) -> &str {
    strip_matching_wrapper(line.trim(), "`")
        .or_else(|| strip_matching_wrapper(line.trim(), "**"))
        .or_else(|| strip_matching_wrapper(line.trim(), "__"))
        .or_else(|| strip_matching_wrapper(line.trim(), "*"))
        .or_else(|| strip_matching_wrapper(line.trim(), "_"))
        .unwrap_or(line)
        .trim()
}

fn strip_matching_wrapper<'a>(line: &'a str, wrapper: &str) -> Option<&'a str> {
    line.strip_prefix(wrapper)
        .and_then(|line| line.strip_suffix(wrapper))
}

fn is_raw_formula_line(line: &str) -> bool {
    line.starts_with("always")
        || line.starts_with('[')
        || line.starts_with('<')
        || line.starts_with("eventually")
        || line.starts_with("formula ")
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
    fn test_parse_llm_response_accepts_lowercase_prefix() {
        let response = "f1: always([+RELEASE] implies eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] implies eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_formula_prefix() {
        let response = "Formula 1: always([+RELEASE] implies eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] implies eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_label_separators() {
        let response = r#"
F1. always([+PAY] implies eventually(<+WORK> true))
Formula 2) <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] implies eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_no_prefix() {
        let response = r#"
always([+PAY] implies eventually(<+WORK> true))
[+EXECUTE] implies <+signed_by(/users/admin.id)> true
<+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert_eq!(formulas[2], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_formula_declaration() {
        let response = "formula generated_1 { always([+PAY] implies eventually(<+WORK> true)) }";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "formula generated_1 { always([+PAY] implies eventually(<+WORK> true)) }"
        );
    }

    #[test]
    fn test_parse_llm_response_strips_list_markers() {
        let response = r#"
- always([+PAY] implies eventually(<+WORK> true))
1. [+EXECUTE] implies <+signed_by(/users/admin.id)> true
2) <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert_eq!(
            formulas[0],
            "always([+PAY] implies eventually(<+WORK> true))"
        );
        assert_eq!(
            formulas[1],
            "[+EXECUTE] implies <+signed_by(/users/admin.id)> true"
        );
        assert_eq!(formulas[2], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_list_markers_before_prefixes() {
        let response = r#"
- F1: always([+PAY] implies eventually(<+WORK> true))
1. Formula 2: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] implies eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_inline_code_wrapping() {
        let response = r#"
F1: `always([+PAY] implies eventually(<+WORK> true))`
- `<+CANCEL> true`
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] implies eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_markdown_emphasis_wrapping() {
        let response = r#"
F1: **always([+PAY] implies eventually(<+WORK> true))**
- _<+CANCEL> true_
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] implies eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_extract_parties() {
        let parties = extract_parties("Alice wants to buy from Bob");
        assert!(parties.contains(&"Alice".to_string()));
        assert!(parties.contains(&"Bob".to_string()));
    }

    #[test]
    fn test_prompt_includes_multi_signer_authorization_pattern() {
        let prompt = generate_prompt("Approval requires Alice and Bob signatures");

        assert!(prompt.contains("<+signed_by(/users/a.id) +signed_by(/users/b.id)> true"));
    }

    #[test]
    fn test_prompt_includes_direct_diamond_patterns() {
        let prompt = generate_prompt("Approval is always allowed");

        assert!(prompt.contains("`<+X> true`"));
        assert!(prompt.contains("`always([<+X>] true)`"));
    }
}
